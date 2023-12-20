use crate::driver::op::{CompletionMeta, Op, OpAble};
use crate::driver::uring::lifecycle::Lifecycle;
use crate::driver::{CURRENT, Driver, Inner};
use io_uring::{cqueue, opcode};
use io_uring::types::Timespec;
use std::cell::UnsafeCell;
use std::io;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;
use slab::Slab;
use crate::driver::util::timespec;

mod lifecycle;
mod waker;

/// 已取消操作
pub(crate) const CANCEL_USERDATA: u64 = u64::MAX;

/// 超时操作
pub(crate) const TIMEOUT_USERDATA: u64 = u64::MAX - 1;

pub(crate) const MIN_REVERSED_USERDATA: u64 = u64::MAX - 2;

/// 保存所有的io_uring操作结构，当io_uring关闭时，需要
struct Ops {
    slab: Slab<Lifecycle>,
}

impl Ops {
    const fn new() -> Self {
        Ops { slab: Slab::new() }
    }

    // Insert a new operation
    pub(crate) fn insert(&mut self) -> usize {
        self.slab.insert(Lifecycle::Submitted)
    }

    fn complete(&mut self, index: usize, result: io::Result<u32>, flags: u32) {
        let lifecycle = unsafe { self.slab.get(index).unwrap_unchecked() };
        lifecycle.complete(result, flags);
    }
}

pub(crate) struct LifecycleRef<'a> {
    index: usize,
    ptr: &'a mut Ops,
}

impl<'a> LifecycleRef<'a> {
    pub(crate) fn remove(self) -> Lifecycle {
        self.ptr.remove(self.index)
    }

    /// io_uring操作完成时执行该函数，修改状态，或者唤醒协程
    pub(crate) fn complete(mut self, result: io::Result<u32>, flags: u32) {
        let mut_ref = &mut (*self);
        match mut_ref {
            Lifecycle::Submitted => {
                *mut_ref = Lifecycle::Completed(result, flags);
            }
            Lifecycle::Waiting(_) => {
                let old = std::mem::replace(mut_ref, Lifecycle::Completed(result, flags));
                match old {
                    Lifecycle::Waiting(waker) => {
                        waker.wake();
                    }
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };
            }
            Lifecycle::Ignored(_) => {
                self.remove();
            }
            Lifecycle::Completed(..) => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// 轮询操作事件
    pub(crate) fn poll_op(mut self, cx: &mut Context<'a>) -> Poll<CompletionMeta> {
        let mut_ref = &mut (*self);
        match mut_ref {
            Lifecycle::Submitted => {
                *mut_ref = Lifecycle::Waiting(cx.waker().clone());
                return Poll::Pending;
            }
            Lifecycle::Waiting(waker) => {
                if !waker.will_wake(cx.waker()) {
                    *mut_ref = Lifecycle::Waiting(cx.waker().clone());
                }
                return Poll::Pending;
            }
            _ => {}
        }

        match self.remove() {
            Lifecycle::Completed(result, flags) => Poll::Ready(CompletionMeta { result, flags }),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    // TODO 这个接口有什么用呢？？？？？？
    pub(crate) fn drop_op<T: 'static>(mut self, data: &mut Option<T>) -> bool {
        let mut_ref = &mut (*self);
        match mut_ref {
            Lifecycle::Submitted | Lifecycle::Waiting(_) => {
                if let Some(data) = data.take() {
                    *mut_ref = Lifecycle::Ignored(Box::new(data));
                } else {
                    // TODO 需要修改，先要搞清楚这个地方有什么作用
                    // *mut_ref = Lifecycle::Ignored(Box::new(T));
                }
                return false;
            }
            Lifecycle::Completed(..) => {
                self.remove();
            }
            Lifecycle::Ignored(_) => unsafe { std::hint::unreachable_unchecked() },
        }
        true
    }
}

/// 封装uring数据
pub(crate) struct UringInner {
    /// 操作
    ops: Ops,
    /// IoUring对象
    uring: ManuallyDrop<io_uring::IoUring>,
    /// 是否支持ext_arg
    ext_arg: bool
}

impl UringInner {
    /// 提取已经完成的任务
    fn tick(&mut self) {
        let mut cq = self.uring.completion();
        cq.sync();

        /// 收获内核中已经完成的请求
        for cqe in cq {
            let index = cqe.user_data();
            match index {
                _ if index >= MIN_REVERSED_USERDATA => {},
                _ => self.ops.complete(index as usize, get_cqe_result(&cqe), cqe.flags()),
            }
        }
    }

    /// 提交任务
    fn submit(&mut self) -> io::Result<()> {
        loop {
            match self.uring.submit() {
                Err(e) => {
                    if e.kind() == io::ErrorKind::Other || e.kind() == io::ErrorKind::ResourceBusy {
                        self.tick();
                    }
                }
                Ok(_) => {
                    return Ok(());
                }
            }
        }
    }

    /// 创建新io操作op
    fn new_op<T>(data: T, inner: &mut UringInner, driver: Inner) -> Op<T> {
        Op {
            driver,
            index: inner.ops.insert() as usize,
            data: Some(data),
        }
    }

    /// 提交任务和data
    pub(crate) fn submit_with_data<T>(this: &Rc<UnsafeCell<UringInner>>, data: T) -> io::Result<Op<T>>
    where
        T: OpAble,
    {
        let mut inner = unsafe { &mut *this.get() };
        // 如果提交队列满了，就提交所有事件给linux内核
        if inner.uring.submission().is_full() {
            inner.submit()?;
        }

        // 创建新的OP操作
        let mut op = Self::new_op(data, inner, Inner(this.clone()));

        // 创建SQE
        let data = unsafe { op.data.as_mut().unwrap_unchecked() };
        // 通过sqe中的 user_data 字段索引存入ops中的Operation
        let sqe = data.uring_op().user_data(op.index as _);

        // 取得sq
        let mut sq = inner.uring.submission();
        unsafe {
            // 讲sqe放入sq中
            if sq.push(&sqe).is_err() {
                panic!("push sqe error!");
            }
        }

        Ok(op)
    }

    /// 轮询操作
    pub(crate) fn poll_op<'a>(
        this: &Rc<UnsafeCell<UringInner>>,
        index: usize,
        cx: &mut Context<'a>,
    ) -> Poll<CompletionMeta> {
        let uring = unsafe { &mut (*this.get()) };
        let lifecycle = unsafe { uring.ops.get(index).unwrap_unchecked() };
        lifecycle.poll_op(cx)
    }

    /// 清理操作
    pub(crate) fn drop_op<T: 'static>(
        this: &Rc<UnsafeCell<UringInner>>,
        index: usize,
        data: &mut Option<T>,
    ) {
        if index == usize::MAX {
            // 已经完成
            return;
        }

        let uring = unsafe { &mut (*this.get()) };
        if let Some(lifecycle) = uring.ops.get(index) {
            let must_finished = lifecycle.drop_op(data);
            if !must_finished {
                let cancel = opcode::AsyncCancel::new(index as u64).build().user_data(u64::MAX);
                unsafe {
                    if uring.uring.submission().push(&cancel).is_err() {
                        uring.submit();
                        uring.uring.submission().push(&cancel);
                    }
                }
            }
        }
    }

    /// 取消操作
    pub(crate) unsafe fn cancel_op(this: &Rc<UnsafeCell<UringInner>>, index: usize) {
        let uring = unsafe { &mut (*this.get()) };
        // 讲user_data设置为u64::MAX表示该操作已经取消
        let cancel = opcode::AsyncCancel::new(index as u64).build().user_data(u64::MAX);
        // 可能会因为sq满了导致放入sqe失败，提交一次在放入sqe。
        if uring.uring.submission().push(&cancel).is_err() {
            uring.submit();
            uring.uring.submission().push(&cancel);
        }
    }
}

impl Drop for UringInner {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.uring);
        };
    }
}

/// 封装iouring的driver
pub struct IoUringDriver {
    uring: Rc<UnsafeCell<UringInner>>,

    /// 超时缓冲区
    timespec: *mut Timespec,
}

impl IoUringDriver {
    const DEFAULT_ENTRIES: u32 = 1024;

    pub(crate) fn new(b: &io_uring::Builder) -> io::Result<IoUringDriver> {
        Self::new_with_entries(b, Self::DEFAULT_ENTRIES)
    }

    pub(crate) fn new_with_entries(
        uring_builder: &io_uring::Builder,
        entries_num: u32,
    ) -> io::Result<IoUringDriver> {
        let uring = ManuallyDrop::new(uring_builder.build(entries_num)?);

        let inner = Rc::new(UnsafeCell::new(UringInner {
            ops: Ops::new(),
            uring,
            ext_arg: uring.params().is_feature_ext_arg(),
        }));
        Ok(IoUringDriver{
            uring: inner,
            timespec: Box::leak(Box::new(Timespec::new())),
        })
    }

    /// 清理提交队列
    fn flush_space(inner: &mut UringInner, need: usize) -> io::Result<()> {
        let sq = inner.uring.submission();
        if sq.len() + need > sq.capacity() {
            drop(sq);
            inner.submit()?;
        }
        Ok(())
    }

    /// 加入一个超时op到sq，经过duration时间后该op会成功返回
    fn install_timeout(&self, inner: &mut UringInner, duration: Duration) {
        let timespec = timespec(duration);
        unsafe {std::ptr::replace(self.timespec, timespec);}
        let entry = opcode::Timeout::new(self.timespec).build().user_data(TIMEOUT_USERDATA);
        let mut sq = inner.uring.submission();
        unsafe { sq.push(&entry); }
    }

    /// TODO
    fn inner_park(&self, timeout: Option<Duration>) -> io::Result<()> {
        let inner = unsafe {&mut *(self.uring.get())};
        let mut need_wait = true;
        if need_wait {
            let mut space = 0;
            if timeout.is_some() {
                space += 1;
            }
            if space != 0 {
                Self::flush_space(inner, space)?;
            }
            if let Some(duration) = timeout {
                match inner.ext_arg {
                    false => {
                        self.install_timeout(inner, duration);
                        inner.uring.submit_and_wait(1)?; // 提交sq并且等待一个OP完成
                    },
                    true => {
                        let timespec = timespec(duration);
                        let args = io_uring::types::SubmitArgs::new().timespec(&timespec);
                        if let Err(e) = inner.uring.submitter().submit_with_args(1, &args) {
                            if e.raw_os_error() != Some(libc::ETIME) {
                                return Err(e);
                            }
                        }
                    }
                }
            } else {
                // 提交并且等待一个OP完成
                inner.uring.submit_and_wait(1)?;
            }
        } else {
            // 直接提交
            inner.uring.submit()?;
        }

        // Process CQ
        inner.tick();
        Ok(())
    }
}

impl Driver for IoUringDriver {
    fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        let inner = Inner::Uring(self.uring.clone());
        CURRENT.set(&inner, f)
    }

    fn submit(&self) -> io::Result<()> {
        let inner = unsafe { &mut *self.uring.get() };
        inner.submit()?;
        inner.tick();
        Ok(())
    }

    fn park(&self) -> io::Result<()> {
        self.inner_park(None)
    }

    fn park_timeout(&self, duration: Duration) -> io::Result<()> {
        self.inner_park(Some(duration))
    }
}

impl AsRawFd for IoUringDriver {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { (*self.uring.get()).uring.as_raw_fd() }
    }
}

impl Drop for IoUringDriver {
    fn drop(&mut self) {
        // 释放时间结构体内存。
        unsafe {
            std::ptr::drop_in_place(self.timespec);
        };
    }
}

#[inline]
fn get_cqe_result(cqe: &cqueue::Entry) -> io::Result<u32> {
    let res = cqe.result();
    if res >= 0 {
        Ok(res as u32)
    } else {
        Err(io::Error::from_raw_os_error(-res))
    }
}

use std::cell::{RefCell, UnsafeCell};
use std::io;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::os::fd::{AsRawFd, RawFd};
use std::ptr::addr_of_mut;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use io_uring::cqueue;
use io_uring::types::Timespec;
use libc::{eventfd, option, timespec};
use crate::driver::Driver;
use crate::driver::op::{CompletionMeta, Op, OpAble};
use crate::driver::uring::lifecycle::Lifecycle;

mod waker;
mod lifecycle;

pub(crate) struct ThreadLocalUring {
    uring: std::rc::Rc<std::cell::UnsafeCell<Uring>>,
}

/// 保存所有的io_uring操作结构，当io_uring关闭时，需要
struct Ops {
    array: Vec<Option<lifecycle::Lifecycle>>,
    cap: usize,
    current_index: usize,
    num: usize,
}

impl Ops {
    const DEFAULT_CAP: usize = 2048;

    fn new() -> Self {
        Self::new_with_max_size(Self::DEFAULT_CAP)
    }

    fn new_with_max_size(max_size: usize) -> Self {
        let mut array = Vec::<Option<Lifecycle>>::new();
        for _ in 0..max_size+1 {
            array.push(None);
        }
        Ops {
            array,
            current_index: 0,
            cap: max_size,
            num: 0,
        }
    }

    pub(crate) fn insert(&mut self) -> i64 {
        let mut index: i64 = -1;

        if self.num == self.cap {
            return index;
        }

        // 循环数组，找到一个可用位置
        for _ in 0..(self.cap as usize) {
            if let None = self.array[self.current_index] {
                index = self.current_index as i64;
                self.num += 1;
                self.array[self.current_index] = Some(lifecycle::Lifecycle::Submitted);
                self.current_index = self.current_index+1 % self.cap;
                break;
            }
            self.current_index = self.current_index+1 % self.cap;
        }
        index
    }

    pub(crate) fn remove(&mut self, index: usize) -> Lifecycle {
        if let Some(lc) = self.array[index].take() {
            return lc;
        } else {
            panic!("logic error, index should not invalid!");
        }
    }

    /// 根据index获取，在数组中的指针和位置
    pub(crate) fn get(&mut self, index: usize) -> Option<LifecycleRef> {
        return if let Some(_) = self.array.get(index) {
            let mut lc = LifecycleRef { index, ptr: self };
            Some(lc)
        } else {
            None
        }
    }

    fn complete(&mut self, index: usize, result: io::Result<u32>, flags: u32) {
        let lifecycle = unsafe {
            self.get(index).unwrap_unchecked()
        };
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
        let mut_ref = &mut(*self);
        match mut_ref {
            Lifecycle::Submitted => {
                *mut_ref = Lifecycle::Completed(result, flags);
            },
            Lifecycle::Waiting(_) => {
                let old = std::mem::replace(mut_ref, Lifecycle::Completed(result, flags));
                match old {
                    Lifecycle::Waiting(waker) => {
                        waker.wake();
                    },
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };
            },
            Lifecycle::Ignored(_) => {
                self.remove();
            },
            Lifecycle::Completed(..) => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// 轮询操作事件
    pub(crate) fn poll_op(mut self, cx: &mut Context<'a>) -> Poll<CompletionMeta> {
        let mut_ref = &mut(*self);
        match mut_ref {
            Lifecycle::Submitted => {
                *mut_ref = Lifecycle::Waiting(cx.waker().clone());
                return Poll::Pending;
            },
            Lifecycle::Waiting(waker) => {
                if !waker.will_wake(cx.waker()) {
                    *mut_ref = Lifecycle::Waiting(cx.waker().clone());
                }
                return Poll::Pending;
            },
            _ => {}
        }

        match self.remove() {
            Lifecycle::Completed(result, flags) => Poll::Ready(CompletionMeta{ result, flags }),
            _ => unsafe { std::hint::unreachable_unchecked() }
        }
    }

    // TODO 这个接口有什么用呢？？？？？？
    pub(crate) fn drop_op<T: 'static>(mut self, data: &mut Option<T>) -> bool {
        let mut_ref = &mut(*self);
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
            Lifecycle::Ignored(_) => {
                unsafe { std::hint::unreachable_unchecked() }
            }
        }
        true
    }
}

impl<'a> AsRef<Lifecycle> for LifecycleRef<'a> {
    fn as_ref(&self) -> &Lifecycle {
        unsafe {
            self.ptr.array[self.index].as_ref().unwrap_unchecked()
        }
    }
}

impl<'a> Deref for LifecycleRef<'a> {
    type Target = Lifecycle;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.ptr.array[self.index].as_ref().unwrap_unchecked()
        }
    }
}

impl<'a> AsMut<Lifecycle> for LifecycleRef<'a> {
    fn as_mut(&mut self) -> &mut Lifecycle {
        unsafe {
            self.ptr.array[self.index].as_mut().unwrap_unchecked()
        }
    }
}

impl<'a> DerefMut for LifecycleRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.ptr.array[self.index].as_mut().unwrap_unchecked()
        }
    }
}

/// 封装uring数据
pub(crate) struct Uring {
    /// 操作
    ops: Ops,

    /// IoUring对象
    uring: ManuallyDrop<io_uring::IoUring>,

    /// event fd
    shared_waker: std::sync::Arc<waker::EventWaker>,

    /// event fd 是否在ring中
    is_eventfd_in_ring: bool,

    // TODO
    // waker_receiver: flume::Re
}

impl Uring {
    fn tick(&mut self) {
        let mut cq = self.uring.completion();
        cq.sync();

        for cqe in cq {
            // TODO 什么JB意思
            if cqe.user_data() >= u64::MAX - 2 {
                if cqe.user_data() == u64::MAX -2 {
                    self.is_eventfd_in_ring = false;
                }
                continue;
            }
            let index = cqe.user_data() as usize;
            self.ops.complete(index, get_cqe_result(&cqe), cqe.flags());
        }
    }

    /// 提交任务
    fn submit(&mut self) -> io::Result<()> {
        loop {
            match self.uring.submit() {
                Ok(_) => {
                    self.uring.submission().sync();
                    return Ok(());
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::Other
                        // || e.kind() == io::ErrorKind::ResourceBusy
                    {
                         self.tick();
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// 创建新io操作op
    fn new_op<T>(data: T, inner: &mut Uring, driver: &ThreadLocalUring) -> Op<T> {
        Op{
            driver: driver.uring.clone(),
            index: inner.ops.insert() as usize,
            data: Some(data),
        }
    }

    /// 提交任务和data
    pub(crate) fn submit_with_data<T>(this: &Rc<UnsafeCell<Uring>>, data: T) -> io::Result<Op<T>>
    where T: OpAble,
    {
        let mut inner = unsafe { &mut *this.get() };
        // 如果提交队列满了，就提交所有事件给linux内核
        if inner.uring.submission().is_full() {
            inner.submit()?;
        }

        // 创建新的operation
        let mut op = Self::new_op(data, inner, &ThreadLocalUring{uring:this.clone()});

        // 创建SQE
        let data = unsafe { op.data.as_mut().unwrap_unchecked() };
        // 通过sqe中的 user_data 字段索引存入ops中的Operation
        let sqe = OpAble::uring_op(data).user_data(op.index as u64);

        // 取得sq
        let mut sq = inner.uring.submission();
        unsafe {
            // 讲sqe放入sq中
            match sq.push(&sqe) {
                Ok(_) => {},
                Err(err) => {
                    panic!("push sqe error!")
                },
            }
        }

        Ok(op)
    }

    /// 轮询操作
    pub(crate) fn poll_op<'a>(this: &Rc<UnsafeCell<Uring>>, index: usize, cx: &mut Context<'a>) -> Poll<CompletionMeta> {
        let uring = unsafe {&mut (*this.get())};
        let lifecycle = unsafe {uring.ops.get(index).unwrap_unchecked()};
        lifecycle.poll_op(cx)
    }

    /// 清理操作
    pub(crate) fn drop_op<T: 'static>(this: &Rc<UnsafeCell<Uring>>, index: usize, data: &mut Option<T>) {
        let uring = unsafe {&mut (*this.get())};
        if index == usize::MAX {
            // 已经完成
            return;
        }
        if let Some(lifecycle) = uring.ops.get(index) {
            let _ = lifecycle.drop_op(data);
        }
    }

    pub(crate) unsafe fn cancel_op(this: &Rc<UnsafeCell<Uring>>, index: usize) {
        let uring = unsafe {&mut (*this.get())};

        // 讲user_data设置为u64::MAX表示该操作已经取消
        let cancel = io_uring::opcode::AsyncCancel::new(index as u64).build().user_data(u64::MAX);

        // 可能会因为sq满了导致放入sqe失败，提交一次在放入sqe。
        if uring.uring.submission().push(&cancel).is_err() {
            let _ = uring.submit();
            let _ = uring.uring.submission().push(&cancel);
        }
    }
}

impl Drop for Uring {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.uring);
        };
    }
}

/// 封装iouring的driver
pub struct IoUringDriver {
    uring: Rc<UnsafeCell<Uring>>,

    /// 超时缓冲区
    timespec: *mut Timespec,

    /// eventfd中数据读取的目的地
    eventfd_read_dst: *mut u8,

    /// 当前io_uring所属的线程Id
    thread_id: usize,
}

impl IoUringDriver {
    const DEFAULT_ENTRIES: u32 = 1024;

    pub(crate) fn new(b: &io_uring::Builder) -> io::Result<IoUringDriver> {
        Self::new_with_entries(b, Self::DEFAULT_ENTRIES)
    }

    pub(crate) fn new_with_entries(b: &io_uring::Builder, entries_num: u32) -> io::Result<IoUringDriver> {
        let io_uring = ManuallyDrop::new(b.build(entries_num)?);

        let waker_fd = {
            unsafe {
                let event_fd = libc::eventfd(0, libc::EFD_CLOEXEC);
                event_fd as RawFd
            }
        };

        // TODO waker_sender


        let uring = Rc::new(UnsafeCell::new(Uring{
            ops: Ops::new(),
            uring: io_uring,
            shared_waker: Arc::new(waker::EventWaker::new(waker_fd)),
            is_eventfd_in_ring: false,
        }));

        // TODO TLS thread id
        let thread_id = 0;
        let driver = IoUringDriver{
            uring,
            timespec: Box::leak(Box::new(Timespec::new())) as *mut Timespec,
            eventfd_read_dst: Box::leak(Box::new([0u8; 8])) as *mut u8,
            thread_id,
        };

        // TODO Register unpark handle
        Ok(driver)
    }
}

// TODO Driver
// impl Driver for IoUringDriver {
//
// }

impl AsRawFd for IoUringDriver {
    fn as_raw_fd(&self) -> RawFd {
        unsafe {
            (*self.uring.get()).uring.as_raw_fd()
        }
    }
}

impl Drop for IoUringDriver {
    fn drop(&mut self) {
        // 释放时间结构体内存。
        unsafe {
            std::ptr::drop_in_place(self.timespec);
        };

        // 释放eventfd的读buffer。
        unsafe {
            std::ptr::drop_in_place(self.eventfd_read_dst);
        };

        // TODO 清理线程
        {




        }
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
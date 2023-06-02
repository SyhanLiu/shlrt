use std::cell::{RefCell, UnsafeCell};
use std::io;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::fd::RawFd;
use std::ptr::addr_of_mut;
use std::rc::Rc;
use std::sync::Arc;
use io_uring::types::Timespec;
use libc::eventfd;
use crate::driver::uring::lifecycle::Lifecycle;

mod waker;
mod lifecycle;

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
        let mut array = Vec::new();
        array.resize(max_size, None);
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
            index
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

    // pub(crate) fn get()

    fn complete(&mut self, index: usize, result: io::Result<u32>, flags: u32) {
        let lifecycle = unsafe {  };
    }
}

pub(crate) struct LifecycleRef<'a> {
    index: usize,
    ptr: &'a mut Ops,
}

impl<'a> LifecycleRef<'a> {
    pub(crate)  fn remove(self) -> Lifecycle {
        self.ptr.remove(self.index)
    }
}

impl<'a> AsRef<Lifecycle> for LifecycleRef<'a> {
    fn as_ref(&self) -> &Lifecycle {
        todo!()
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

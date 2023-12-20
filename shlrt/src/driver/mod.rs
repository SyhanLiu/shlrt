use io_uring;
use std::io;
use std::time::Duration;
use crate::driver::uring::UringInner;
use crate::scoped_thread_local;

mod op;
pub(crate) mod shared_fd;
mod uring;
mod util;

scoped_thread_local!(pub(crate) static CURRENT: Inner);

/// Core driver trait.
pub trait Driver {
    /// Run with driver TLS.
    fn with<R>(&self, f: impl FnOnce() -> R) -> R;
    /// Submit ops to kernel and process returned events.
    fn submit(&self) -> io::Result<()>;
    /// Wait infinitely and process returned events.
    fn park(&self) -> io::Result<()>;
    /// Wait with timeout and process returned events.
    fn park_timeout(&self, duration: Duration) -> io::Result<()>;

    // The struct to wake thread from another.
    // type Unpark: unpark::Unpark;

    // Get Unpark.
    // fn unpark(&self) -> Self::Unpark;
}

// TODO TLS变量，每个线程一个io_uring实例
// thread_local!(pub(crate) static CURRENT: uring::ThreadLocalUring = uring::ThreadLocalUring{
//     uring: std::rc::Rc::new(std::cell::UnsafeCell::new(std::ptr::null())),
// });
// TODO 写TMD的宏

pub(crate) struct Inner(std::rc::Rc<std::cell::UnsafeCell<UringInner>>);
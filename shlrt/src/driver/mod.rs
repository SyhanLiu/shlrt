use io_uring;
use std::io;
use std::task::{Context, Poll};
use std::time::Duration;
use crate::driver::op::{CompletionMeta, Op, OpAble, OpCanceller};
use crate::driver::uring::UringInner;
use crate::scoped_thread_local;

pub(crate) mod op;
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
}

pub(crate) struct Inner(std::rc::Rc<std::cell::UnsafeCell<UringInner>>);

impl Inner {
    /// 提交op操作
    fn submit_with<T: OpAble>(&self, data: T) -> io::Result<Op<T>> {
        UringInner::submit_with_data(&self.0, data)
    }

    fn poll_op<T: OpAble>(&self, data: &mut T, index: usize, cx: &mut Context<'_>) -> Poll<CompletionMeta> {
        UringInner::poll_op(&self.0, index, cx)
    }

    fn drop_op<T:'static>(&self, index: usize, data: &mut Option<T>) {
        UringInner::drop_op(&self.0, index, data);
    }

    unsafe fn cancel_op(&self, op_canceller: OpCanceller) {
        UringInner::cancel_op(&self.0, op_canceller.index);
    }
}
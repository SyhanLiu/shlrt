use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use crate::driver;
use crate::driver::Inner;

mod accept;
mod close;
mod connect;
mod fsync;
mod open;

/// 封装io_uring的operation
pub(crate) struct Op<T: 'static> {
    // 所属的io_uring
    pub(super) driver: Inner,
    // slab的index
    pub(super) index: usize,
    // op操作包含的data信息
    pub(super) data: Option<T>,
}

/// 操作完成时的元信息
#[derive(Debug)]
pub(crate) struct CompletionMeta {
    pub(crate) result: io::Result<u32>,
    pub(crate) flags: u32,
}

/// Op完成时。
#[derive(Debug)]
pub(crate) struct Completion<T> {
    pub(crate) data: T,
    pub(crate) meta: CompletionMeta,
}

/// 封装io_uring的操作
pub(crate) trait OpAble {
    /// 创建io_uring操作的SQE
    fn uring_op(&mut self) -> io_uring::squeue::Entry;
}

impl<T> Op<T> {
    /// 提交OP操作
    pub(super) fn submit_with(data: T) -> io::Result<Op<T>>
        where
            T: OpAble,
    {
        driver::CURRENT.with(|this| this.submit_with(data))
    }

    pub(super) fn try_submit_with(data: T) -> io::Result<Op<T>>
        where
            T: OpAble,
    {
        /// 检查当前的TLS是否被设置
        if driver::CURRENT.is_set() {
            Self::submit_with(data)
        } else {
            Err(io::ErrorKind::Other.into())
        }
    }

    pub(crate) fn op_canceller(&self) -> OpCanceller
        where
            T: OpAble
    {
        return OpCanceller{
            index: self.index,
        }
    }
}

impl<T> Future for Op<T>
where T: Unpin + OpAble + 'static
{
    type Output = Completion<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let data_mut = this.data.as_mut().expect("unexpected operation state");
        let meta = ready!(this.driver.0.poll_op::<T>(data_mut, this.index, cx));

        this.index = usize::MAX;
        let data = this.data.take().expect("unexpected operation state");
        Poll::Ready(Completion{data, meta})
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub(crate) struct OpCanceller {
    pub(super) index: usize,
}
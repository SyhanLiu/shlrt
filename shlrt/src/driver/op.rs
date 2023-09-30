use crate::driver;
use crate::driver::uring::Uring;
use std::io;

mod accept;

/// 封装io_uring的operation
pub(crate) struct Op<T: 'static> {
    // 所属的io_uring
    pub(super) driver: std::rc::Rc<std::cell::UnsafeCell<Uring>>,
    // 所属的Op队列
    pub(super) index: usize,
    // op操作包含的data信息
    pub(super) data: Option<T>,
}

/// 操作完成时的信息
#[derive(Debug)]
pub(crate) struct CompletionMeta {
    pub(crate) result: io::Result<u32>,
    pub(crate) flags: u32,
}

/// 封装io_uring的操作
pub(crate) trait OpAble {
    /// 创建io_uring操作的SQE
    fn uring_op(&mut self) -> io_uring::squeue::Entry;
}

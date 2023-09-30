use std::task::Waker;

/// Uring的生命周期

/// iouring操作的生命周期
pub(crate) enum Lifecycle {
    /// op已经被提交，且正在运行
    Submitted,
    /// 提交者正在等待op的完成
    Waiting(Waker),
    /// 提交者对结果已经不感兴趣。
    Ignored(Box<dyn std::any::Any>),
    /// op已经完成
    Completed(std::io::Result<u32>, u32),
}

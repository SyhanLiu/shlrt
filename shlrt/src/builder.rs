use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct RuntimeBuilder {
    /// iouring中的entry数量
    entries: Option<usize>,
    uring_builder: io_uring::Builder,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        RuntimeBuilder::new()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            entries: None,
            uring_builder: io_uring::IoUring::builder(),
        }
    }
}
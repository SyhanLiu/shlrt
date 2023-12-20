use crate::driver::shared_fd::SharedFd;

pub trait AsReadFd {
    fn as_reader_fd(&mut self) -> &SharedFdWrapper;
}

pub trait AsWriteFd {
    fn as_writer_fd(&mut self) -> &SharedFdWrapper;
}

#[repr(transparent)]
pub struct SharedFdWrapper(SharedFd);

impl SharedFdWrapper {
    #[allow(unused)]
    pub(crate) fn as_ref(&self) -> &SharedFd {
        &self.0
    }

    pub(crate) fn new(inner: &SharedFd) -> &Self {
        unsafe { std::mem::transmute(inner) }
    }
}

use crate::driver::shared_fd::SharedFd;
use io_uring::{opcode, types};
use std::io;
use std::mem::size_of;
use std::mem::MaybeUninit;

/// accept操作封装
pub(crate) struct Accept {
    pub(crate) fd: SharedFd,
    pub(crate) addr: Box<(MaybeUninit<libc::sockaddr_storage>, libc::socklen_t)>,
}

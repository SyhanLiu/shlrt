use std::io;
use std::mem::size_of;
use std::mem::MaybeUninit;
use io_uring::{opcode, types};

/// accept操作封装
pub(crate) struct Accept {
    pub(crate) fd: SharedFd,
    pub(crate) addr: Box<(MaybeUninit<libc::sockaddr_storage>, libc::socklen_t)>,
}
use std::intrinsics::size_of;
use std::io;
use crate::driver::shared_fd::SharedFd;
use std::mem::MaybeUninit;
use io_uring::{opcode, types};
use io_uring::squeue::Entry;
use crate::driver::op::{Op, OpAble};

/// accept操作封装
pub(crate) struct Accept {
    pub(crate) fd: SharedFd,
    pub(crate) addr: Box<(MaybeUninit<libc::sockaddr_storage>, libc::socklen_t)>,
}

impl Op<Accept> {
    /// 封装accept操作
    fn accept(fd:&SharedFd) -> io::Result<Self> {
        let addr = Box::new((
            MaybeUninit::uninit(),
            size_of::<libc::sockaddr_storage>() as libc::socklen_t),
        );

        Op::submit_with(Accept{
            fd: fd.clone(),
            addr,
        })
    }
}

impl OpAble for Accept {
    fn uring_op(&mut self) -> Entry {
        opcode::Accept::new(
            types::Fd(self.fd.raw_fd()),
            self.addr.0.as_mut_ptr() as *mut libc::sockaddr,
            &mut self.addr.1,
        ).build()
    }
}
use std::io;
use io_uring::{opcode, types};
use io_uring::squeue::Entry;
use crate::driver::op::{Op, OpAble};
use crate::driver::shared_fd::SharedFd;

pub(crate) struct Fsync {
    fd: SharedFd,
    data_sync: bool,
}

impl Op<Fsync> {
    pub(crate) fn fsync(fd: &SharedFd) -> io::Result<Op<Fsync>> {
        Op::submit_with(Fsync{
            fd: fd.clone(),
            data_sync: false,
        })
    }

    fn data_sync(fd: &SharedFd) -> io::Result<Op<Fsync>> {
        Op::submit_with(Fsync{
            fd: fd.clone(),
            data_sync: true,
        })
    }
}

impl OpAble for Fsync {
    fn uring_op(&mut self) -> Entry {
        let mut opcode = opcode::Fsync::new(types::Fd(self.fd.raw_fd()));
        if self.data_sync {
            opcode = opcode.flags(types::FsyncFlags::DATASYNC);
        }
        opcode.build()
    }
}
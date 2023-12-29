use std::io;
use std::os::unix::io::RawFd;
use io_uring::{opcode, types};
use io_uring::squeue::Entry;
use crate::driver::op::{Op, OpAble};

struct Close {
    fd: RawFd
}

impl Op<Close> {
    fn close(fd: RawFd) -> io::Result<Op<Close>> {
        Self::try_submit_with(Close{fd})
    }
}

impl OpAble for Close {
    fn uring_op(&mut self) -> Entry {
        opcode::Close::new(types::Fd(self.fd)).build()
    }
}
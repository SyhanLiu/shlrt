use std::io;
use core::net::SocketAddr;
use io_uring::{opcode, types};
use crate::driver::op::{Op, OpAble};
use crate::driver::shared_fd::SharedFd;

struct Connect {
    fd: SharedFd,
    socket_addr: Box<libc::sockaddr_in>,
    socket_addr_len: libc::socklen_t,
}

impl Op<Connect> {
    pub(crate) fn connect(
        socket: SharedFd,
        addr: SocketAddr,
        _tfo: bool,
    ) -> io::Result<Op<Connect>> {
        let (raw_addr, raw_addr_length) = socket_addr(&addr);
        Op::submit_with(Connect {
            fd: socket,
            socket_addr: Box::new(raw_addr),
            socket_addr_len: raw_addr_length,
        })
    }
}

impl OpAble for Connect {
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        opcode::Connect::new(
            types::Fd(self.fd.raw_fd()),
            self.socket_addr.as_ptr(),
            self.socket_addr_len,
        ).build()
    }
}

/// 转换为原生libc的ipv4地址
pub(crate) fn socket_addr(addr: &SocketAddr) -> (libc::sockaddr_in, libc::socklen_t) {
    match addr {
        SocketAddr::V4(addr) => {
            let sin_addr = libc::in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()),
            };
            let sockaddr_in = libc::sockaddr_in {
                sin_family: libc::AF_INET as libc::sa_family_t,
                sin_port: addr.port().to_be(),
                sin_addr,
                sin_zero: [0; 8],
            };

            return (sockaddr_in, std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t)
        }
        _ => {
            panic!("Only support IPv4");
        }
    }
}
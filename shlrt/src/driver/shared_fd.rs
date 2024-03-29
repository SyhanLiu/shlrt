use std::fmt::Formatter;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{cell::UnsafeCell, io, rc::Rc};

/// 封装fd
#[derive(Clone, Debug)]
pub(crate) struct SharedFd {
    inner: Rc<InnerFd>,
}

impl SharedFd {
    /// 新建初始化共享文件描述符结构
    pub(crate) fn new(fd: RawFd) -> io::Result<SharedFd> {
        Ok(SharedFd {
            inner: Rc::new(InnerFd {
                fd,
                state: UnsafeCell::new(State::Init),
            }),
        })
    }

    pub(crate) fn raw_fd(&self) -> RawFd {
        self.inner.fd
    }

    //
    // pub(crate) fn new_without_register(fd: RawFd) -> SharedFd {
    //
    // }
}



struct InnerFd {
    fd: RawFd,
    state: UnsafeCell<State>,
}

impl std::fmt::Debug for InnerFd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InnerFd").field("fd", &self.fd).finish()
    }
}

enum State {
    /// 初始化
    Init,
    /// 等待被唤醒
    Waiting(Option<std::task::Waker>),
    /// 正在关闭
    Closing(),
    /// 已经完全关闭
    Closed,
}

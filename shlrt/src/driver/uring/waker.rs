use std::ffi::c_void;
use std::os::fd::{AsRawFd, RawFd};

/// 通过eventfd来唤醒线程
pub(crate) struct EventWaker {
    /// event fd
    eventfd: RawFd,
    /// 状态
    pub(crate) awake: std::sync::atomic::AtomicBool,
}

impl EventWaker {
    pub(crate) fn new(fd: RawFd) -> Self {
        EventWaker {
            eventfd: fd,
            awake: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// 唤醒操作
    pub(crate) fn wake(&self) -> std::io::Result<()> {
        // 已经被唤醒时，直接返回OK
        if self.awake.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        // 没有被唤醒时，需要向eventFD中写入数据
        let buf = 0x1u64.to_ne_bytes();
        unsafe {
            libc::write(self.eventfd, buf.as_ptr() as *const c_void, buf.len());
            Ok(())
        }
    }
}

impl AsRawFd for EventWaker {
    fn as_raw_fd(&self) -> RawFd {
        self.eventfd
    }
}

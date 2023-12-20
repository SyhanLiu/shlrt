use super::{IoBuf, IoBufMut, IoVecBuf, IoVecBufMut};
use std::ptr::null;

/// 要保证ptr指向的内存区一定可用
pub struct RawBuf {
    ptr: *const u8,
    len: usize,
}

impl RawBuf {
    /// 创建空的RawBuf
    #[inline]
    pub unsafe fn uninit() -> Self {
        Self {
            ptr: null(),
            len: 0,
        }
    }

    /// 创建并初始化
    #[inline]
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        Self { ptr, len }
    }
}

unsafe impl IoBuf for RawBuf {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.ptr
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len
    }
}

unsafe impl IoBufMut for RawBuf {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.ptr as *mut u8
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.len
    }

    #[inline]
    unsafe fn set_init(&mut self, _pos: usize) {
        // TODO
    }
}

impl RawBuf {
    /// 从iovec创建RawBuf
    #[inline]
    pub unsafe fn new_from_iovec_mut<T: IoVecBufMut>(data: &mut T) -> Option<Self> {
        if data.write_iovec_len() == 0 {
            return None;
        }
        let iovec = *(data.write_iovec_ptr());
        Some(Self::new(iovec.iov_base as _, iovec.iov_len))
    }

    #[inline]
    pub unsafe fn new_from_iovec<T: IoVecBuf>(data: &T) -> Option<Self> {
        if data.read_iovec_len() == 0 {
            return None;
        }
        let iovec = *data.read_iovec_ptr();
        Some(Self::new(iovec.iov_base as _, iovec.iov_len))
    }
}

/// 要保证iovec一定可用
pub struct RawBufIovec {
    ptr: *const libc::iovec,
    len: usize,
}

impl RawBufIovec {
    #[inline]
    pub unsafe fn new(ptr: *const libc::iovec, len: usize) -> Self {
        Self { ptr, len }
    }
}

unsafe impl IoVecBuf for RawBufIovec {
    #[inline]
    fn read_iovec_ptr(&self) -> *const libc::iovec {
        self.ptr
    }

    #[inline]
    fn read_iovec_len(&self) -> usize {
        self.len
    }
}

unsafe impl IoVecBufMut for RawBufIovec {
    fn write_iovec_ptr(&mut self) -> *mut libc::iovec {
        self.ptr as *mut libc::iovec
    }

    fn write_iovec_len(&mut self) -> usize {
        self.len
    }

    unsafe fn set_init(&mut self, _pos: usize) {}
}

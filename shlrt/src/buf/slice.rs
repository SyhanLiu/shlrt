use std::ops;
use super::{IoVecBuf, IoVecBufMut, IoBuf, IoBufMut};

/// 使用 [IoBuf::slice] 创建Slice
pub struct SliceMut<T> {
    buf: T,
    begin: usize,
    end: usize,
}

impl<T: IoBuf + IoBufMut> SliceMut<T> {
    pub fn new(mut buf: T, begin: usize, end: usize) -> Self {
        assert!(end <= buf.bytes_total()); // 不能超过总容量
        assert!(begin <= buf.bytes_init()); // 开始区间要在初始化的数据之内
        assert!(begin <= end);
        Self { buf, begin, end }
    }
}

impl<T> SliceMut<T> {
    #[inline]
    pub unsafe fn new_unchecked(buf: T, begin: usize, end: usize) -> Self {
        Self { buf, begin, end }
    }

    #[inline]
    pub fn begin(&self) -> usize {
        self.begin
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn get_ref(&self) -> &T {
        &self.buf
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.buf
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.buf
    }
}

impl<T: IoBuf> ops::Deref for SliceMut<T> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        let buf_bytes = super::deref(&self.buf);
        let end = std::cmp::min(self.end, buf_bytes.len());
        &buf_bytes[self.begin..end]
    }
}

unsafe impl<T: IoBuf> IoBuf for SliceMut<T> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        super::deref(&self.buf)[self.begin..].as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        ops::Deref::deref(self).len()
    }
}

unsafe impl<T: IoBufMut> IoBufMut for SliceMut<T> {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        unsafe { self.buf.write_ptr().add(self.begin) }
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.end - self.begin
    }

    #[inline]
    unsafe fn set_init(&mut self, n: usize) {
        self.buf.set_init(self.begin + n);
    }
}

pub struct Slice<T> {
    buf: T,
    begin: usize,
    end: usize,
}

impl<T: IoBuf> Slice<T> {
    #[inline]
    pub fn new(buf: T, begin: usize, end: usize) -> Self {
        assert!(end <= buf.bytes_init());
        assert!(begin <= end);
        Self { buf, begin, end }
    }
}

impl<T> Slice<T> {
    #[inline]
    pub unsafe fn new_unchecked(buf: T, begin: usize, end: usize) -> Self {
        Self { buf, begin, end }
    }

    #[inline]
    pub fn begin(&self) -> usize {
        self.begin
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn get_ref(&self) -> &T {
        &self.buf
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.buf
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.buf
    }
}

unsafe impl<T: IoBuf> IoBuf for Slice<T> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        unsafe { self.buf.read_ptr().add(self.begin) }
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.end - self.begin
    }
}

pub struct IoVecWrapper<T> {
    raw: T,
}

impl<T: IoVecBuf> IoVecWrapper<T> {
    pub fn new(iovec_buf: T) -> Result<Self, T> {
        #[cfg(unix)]
        if iovec_buf.read_iovec_len() == 0 {
            return Err(iovec_buf);
        }
        Ok(Self { raw: iovec_buf })
    }

    pub fn into_inner(self) -> T {
        self.raw
    }
}

unsafe impl<T: IoVecBuf> IoBuf for IoVecWrapper<T> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        let iovec = unsafe { *(self.raw.read_iovec_ptr()) };
        iovec.iov_base as _
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        let iovec = unsafe { *(self.raw.read_iovec_ptr()) };
        iovec.iov_len
    }
}

pub struct IoVecWrapperMut<T> {
    raw: T,
}

impl<T: IoVecBufMut> IoVecWrapperMut<T> {
    pub fn new(mut iovec_buf: T) -> Result<Self, T> {
        if iovec_buf.write_iovec_len() == 0 {
            return Err(iovec_buf);
        }
        Ok(Self { raw: iovec_buf })
    }

    pub fn into_inner(self) -> T {
        self.raw
    }
}

unsafe impl<T: IoVecBufMut> IoBufMut for IoVecWrapperMut<T> {
    fn write_ptr(&mut self) -> *mut u8 {
        let iovec = unsafe { *(self.raw.write_iovec_ptr()) };
        iovec.iov_base as *mut u8
    }

    fn bytes_total(&mut self) -> usize {
        let iovec = unsafe { *(self.raw.write_iovec_ptr()) };
        iovec.iov_len
    }

    unsafe fn set_init(&mut self, _pos: usize) {}
}

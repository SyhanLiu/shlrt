use std::ops;
use super::Slice;
use crate::buf::slice::SliceMut;
use core::ops::Bound;

pub unsafe trait IoBuf: Unpin + 'static {
    /// 返回读缓冲区的指针
    fn read_ptr(&self) -> *const u8;

    /// 返回长度
    fn bytes_init(&self) -> usize;

    /// 返回一个可读片段，不带所有权
    #[inline]
    fn slice(self, range: impl ops::RangeBounds<usize>) -> Slice<Self>
    where
        Self: Sized,
    {
        let (begin, end) = parse_range(range, self.bytes_init());
        Slice::new(self, begin, end)
    }

    /// 同slice方法，但是不检查是否越界
    #[inline]
    unsafe fn slice_unchecked(self, range: impl ops::RangeBounds<usize>) -> Slice<Self>
    where
        Self: Sized,
    {
        let (begin, end) = parse_range(range, self.bytes_init());
        Slice::new_unchecked(self, begin, end)
    }
}

unsafe impl IoBuf for Vec<u8> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for Box<[u8]> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for &'static [u8] {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        <[u8]>::len(self)
    }
}

unsafe impl<const N: usize> IoBuf for Box<[u8; N]> {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl<const N: usize> IoBuf for &'static [u8; N] {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl<const N: usize> IoBuf for &'static mut [u8; N] {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for &'static str {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        <str>::len(self)
    }
}

unsafe impl IoBuf for bytes::Bytes {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

unsafe impl IoBuf for bytes::BytesMut {
    #[inline]
    fn read_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn bytes_init(&self) -> usize {
        self.len()
    }
}

pub unsafe trait IoBufMut: Unpin + 'static {
    /// 获取写缓冲区的指针
    fn write_ptr(&mut self) -> *mut u8;

    /// 获取缓冲区的cap
    fn bytes_total(&mut self) -> usize;

    /// 设置初始化到的位置
    unsafe fn set_init(&mut self, pos: usize);

    /// 返回带有所有权的切片
    /// # Examples
    /// ```
    /// use shlrt::buf::{IoBuf, IoBufMut};
    ///
    /// let buf = b"hello world".to_vec();
    /// buf.slice_mut(5..10);
    /// ```
    #[inline]
    fn slice_mut(mut self, range: impl ops::RangeBounds<usize>) -> SliceMut<Self>
    where
        Self: Sized,
        Self: IoBuf,
    {
        let (begin, end) = parse_range(range, self.bytes_total());
        SliceMut::new(self, begin, end)
    }

    /// 同slice方法，但是不检查是否越界
    #[inline]
    unsafe fn slice_mut_unchecked(mut self, range: impl ops::RangeBounds<usize>) -> SliceMut<Self>
    where
        Self: Sized,
    {
        let (begin, end) = parse_range(range, self.bytes_total());
        SliceMut::new_unchecked(self, begin, end)
    }
}

unsafe impl IoBufMut for Vec<u8> {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.capacity()
    }

    #[inline]
    unsafe fn set_init(&mut self, init_len: usize) {
        self.set_len(init_len);
    }
}

unsafe impl IoBufMut for Box<[u8]> {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.len()
    }

    #[inline]
    unsafe fn set_init(&mut self, _: usize) {}
}

unsafe impl<const N: usize> IoBufMut for Box<[u8; N]> {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.len()
    }

    #[inline]
    unsafe fn set_init(&mut self, _: usize) {}
}

unsafe impl<const N: usize> IoBufMut for &'static mut [u8; N] {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.len()
    }

    #[inline]
    unsafe fn set_init(&mut self, _: usize) {}
}

unsafe impl IoBufMut for bytes::BytesMut {
    #[inline]
    fn write_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    #[inline]
    fn bytes_total(&mut self) -> usize {
        self.capacity()
    }

    #[inline]
    unsafe fn set_init(&mut self, init_len: usize) {
        if self.len() < init_len {
            self.set_len(init_len);
        }
    }
}

/// 解析范围
fn parse_range(range: impl ops::RangeBounds<usize>, end: usize) -> (usize, usize) {
    let begin = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n + 1,
        Bound::Unbounded => 0,
    };

    let end = match range.end_bound() {
        Bound::Included(&n) => n.checked_add(1).expect("out of range"),
        Bound::Excluded(&n) => n,
        Bound::Unbounded => end,
    };
    (begin, end)
}
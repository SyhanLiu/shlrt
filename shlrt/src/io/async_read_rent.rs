use std::future::Future;

use crate::{
    buf::{IoBufMut, IoVecBufMut, RawBuf},
    BufResult,
};

pub trait AsyncReadRent {
    fn read<T: IoBufMut>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>>;

    fn readv<T: IoVecBufMut>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>>;
}

pub trait AsyncReadRentAt {
    fn read_at<T: IoBufMut>(
        &mut self,
        buf: T,
        pos: usize,
    ) -> impl Future<Output = BufResult<usize, T>>;
}

impl<A: ?Sized + AsyncReadRent> AsyncReadRent for &mut A {
    #[inline]
    fn read<T: IoBufMut>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>> {
        (**self).read(buf)
    }

    #[inline]
    fn readv<T: IoVecBufMut>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>> {
        (**self).readv(buf)
    }
}

impl AsyncReadRent for &[u8] {
    fn read<T: IoBufMut>(&mut self, mut buf: T) -> impl Future<Output = BufResult<usize, T>> {
        let used = std::cmp::min(self.len(), buf.bytes_total());
        let (a, b) = self.split_at(used);
        unsafe {
            buf.write_ptr().copy_from_nonoverlapping(a.as_ptr(), used);
            buf.set_init(used);
        }
        *self = b;
        async move { (Ok(used), buf) }
    }

    fn readv<T: IoVecBufMut>(&mut self, mut buf: T) -> impl Future<Output = BufResult<usize, T>> {
        let n = match unsafe { RawBuf::new_from_iovec_mut(&mut buf) } {
            Some(mut raw_buf) => {
                let used = std::cmp::min(self.len(), raw_buf.bytes_total());
                let (a, b) = self.split_at(used);
                unsafe {
                    raw_buf
                        .write_ptr()
                        .copy_from_nonoverlapping(a.as_ptr(), used);
                    raw_buf.set_init(used);
                }
                *self = b;
                used
            }
            None => 0,
        };
        unsafe { buf.set_init(n) };
        async move { (Ok(n), buf) }
    }
}

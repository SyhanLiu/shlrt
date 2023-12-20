use std::future::Future;
use crate::buf::{IoBufMut, IoVecBufMut, SliceMut};
use crate::BufResult;
use crate::io::async_read_rent::AsyncReadRent;

pub trait AsyncReadRentExt {
    fn read_exact<T>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>>
        where T: IoBufMut + 'static;

    fn read_iovec_exact<T>(&mut self, buf: T) -> impl Future<Output = BufResult<usize, T>>
        where T: IoVecBufMut + 'static;
}

impl<A> AsyncReadRentExt for A where A: AsyncReadRent + ?Sized {
    fn read_exact<T>(&mut self, mut buf: T) -> impl Future<Output = BufResult<usize, T>> where T: IoBufMut + 'static {
        async {
            let len = buf.bytes_total();
            let mut read = 0;
            while read < len {
                let slice = unsafe {SliceMut::new_unchecked(buf, read, len)};
                let (result, slice) = self.read(slice).await;
                buf = slice.into_inner();
                match result {
                    Ok(0) => {
                        return (Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "failed to fill whole buffer")), buf);
                    }
                    Ok(n) => {
                        read += n;
                        unsafe {buf.set_init(read)};
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::Interrupted {
                            return (Err(e), buf);
                        }
                    }
                }
            }
            (Ok(read), buf)
        }
    }

    fn read_iovec_exact<T>(&mut self, mut buf: T) -> impl Future<Output = BufResult<usize, T>> where T: IoVecBufMut + 'static {
        async {
            let mut meta = crate::buf::write_vec_meta(&mut buf);
            let len = meta.len();
            let mut read = 0;

            while read < len {
                let (result, meta_tmp) = self.readv(meta).await;
                meta = meta_tmp;
                match result {
                    Ok(0) => {
                        return (Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "failed to fill whole buffer")), buf);
                    }
                    Ok(n) => {
                        read += n;
                        unsafe {buf.set_init(read)};
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::Interrupted {
                            return (Err(e), buf);
                        }
                    }
                }
            }
            (Ok(read), buf)
        }
    }
}
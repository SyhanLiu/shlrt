use std::io;
use std::path::Path;
use crate::driver::shared_fd::SharedFd;
use crate::fs::open_option::OpenOptions;
use std::fs::{File as StdFile};
use std::os::fd::{AsRawFd, IntoRawFd, RawFd};
use crate::buf::{IoBuf, IoBufMut};
use crate::driver::op::Op;

#[derive(Debug)]
pub struct File {
    fd: SharedFd,
}

impl File {
    pub async fn open(path: impl AsRef<Path>) -> io::Result<File> {
        OpenOptions::new().read(true).open(path).await
    }

    pub async fn create(path: impl AsRef<Path>) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
    }

    pub fn from_shared_fd(fd: SharedFd) -> File {
        File { fd }
    }

    pub fn from_std(std: StdFile) -> io::Result<File> {
        Ok(File {
            fd: SharedFd::new(std.into_raw_fd())?,
        })
    }

    pub async fn read_at<T: IoBufMut>(&self, buf: T, pos: u64) -> crate::BufResult<usize, T> {
        let op = Op::read_at(&self.fd, buf, pos).unwrap();
        op.read().await
    }

    pub async fn read_exact_at<T: IoBufMut>(&self, mut buf: T, pos: u64, ) -> crate::BufResult<(), T> {
        let len = buf.bytes_total();
        let mut read = 0;
        while read < len {
            let slice = unsafe { buf.slice_mut_unchecked(read..len) };
            let (res, slice) = self.read_at(slice, pos + read as u64).await;
            buf = slice.into_inner();
            match res {
                Ok(0) => {
                    return (
                        Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "failed to fill whole buffer",
                        )),
                        buf,
                    )
                }
                Ok(n) => {
                    read += n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return (Err(e), buf),
            };
        }

        (Ok(()), buf)
    }

    pub async fn write_at<T: IoBuf>(&self, buf: T, pos: u64) -> crate::BufResult<usize, T> {
        let op = Op::write_at(&self.fd, buf, pos).unwrap();
        op.write().await
    }

    pub async fn write_all_at<T: IoBuf>(&self, mut buf: T, pos: u64) -> crate::BufResult<(), T> {
        let len = buf.bytes_init();
        let mut written = 0;
        while written < len {
            let slice = unsafe { buf.slice_unchecked(written..len) };
            let (res, slice) = self.write_at(slice, pos + written as u64).await;
            buf = slice.into_inner();
            match res {
                Ok(0) => {
                    return (
                        Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )),
                        buf,
                    )
                }
                Ok(n) => written += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return (Err(e), buf),
            };
        }

        (Ok(()), buf)
    }

    pub async fn sync_all(&self) -> io::Result<()> {
        let op = Op::fsync(&self.fd).unwrap();
        let completion = op.await;

        completion.meta.result?;
        Ok(())
    }

    pub async fn sync_data(&self) -> io::Result<()> {
        let op = Op::datasync(&self.fd).unwrap();
        let completion = op.await;

        completion.meta.result?;
        Ok(())
    }

    pub async fn close(self) -> io::Result<()> {
        self.fd.close().await;
        Ok(())
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.raw_fd()
    }
}
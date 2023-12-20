use std::ffi::c_void;

/// Iovec 抽象，适配readv
pub unsafe trait IoVecBuf: Unpin + 'static {
    /// 返回iovec结构的指针
    /// struct iovec {
    ///     void  *iov_base;
    ///     size_t iov_len;
    /// };
    fn read_iovec_ptr(&self) -> *const libc::iovec;

    /// 返回iovec的数量
    fn read_iovec_len(&self) -> usize;
}

/// 中间结构，主要用于raw和iovecs之间的互相转换
#[derive(Clone)]
pub struct VecBuf {
    iovecs: Vec<libc::iovec>,
    raw: Vec<Vec<u8>>,
}

unsafe impl IoVecBuf for VecBuf {
    fn read_iovec_ptr(&self) -> *const libc::iovec {
        self.iovecs.read_iovec_ptr()
    }
    fn read_iovec_len(&self) -> usize {
        self.iovecs.read_iovec_len()
    }
}

unsafe impl IoVecBuf for Vec<libc::iovec> {
    fn read_iovec_ptr(&self) -> *const libc::iovec {
        self.as_ptr()
    }

    fn read_iovec_len(&self) -> usize {
        self.len()
    }
}

impl From<Vec<Vec<u8>>> for VecBuf {
    fn from(vv: Vec<Vec<u8>>) -> Self {
        let iovecs = vv
            .iter()
            .map(|v| libc::iovec {
                iov_base: v.as_ptr() as *mut c_void,
                iov_len: v.len(),
            })
            .collect();
        Self { iovecs, raw: vv }
    }
}

impl From<VecBuf> for Vec<Vec<u8>> {
    fn from(vb: VecBuf) -> Self {
        vb.raw
    }
}

/// 可变的iovec 抽象，适配writev
pub unsafe trait IoVecBufMut: Unpin + 'static {
    fn write_iovec_ptr(&mut self) -> *mut libc::iovec;

    fn write_iovec_len(&mut self) -> usize;

    unsafe fn set_init(&mut self, pos: usize);
}

unsafe impl IoVecBufMut for VecBuf {
    fn write_iovec_ptr(&mut self) -> *mut libc::iovec {
        self.read_iovec_ptr() as *mut libc::iovec
    }

    fn write_iovec_len(&mut self) -> usize {
        self.read_iovec_len()
    }

    /// 设置iovec的长度，也是设置Vec<u8>的长度，因为iovec的缓冲区就是Vec<u8>的data。
    unsafe fn set_init(&mut self, mut len: usize) {
        for (idx, iovec) in self.iovecs.iter_mut().enumerate() {
            if iovec.iov_len <= len {
                self.raw[idx].set_len(iovec.iov_len);
                len -= iovec.iov_len;
            } else {
                if len > 0 {
                    self.raw[idx].set_len(len);
                }
                break;
            }
        }
    }
}

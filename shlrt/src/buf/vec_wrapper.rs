use super::{IoVecBuf, IoVecBufMut};

pub(crate) struct IoVecMeta {
    data: Vec<libc::iovec>,
    offset: usize,
    len: usize,
}

/// 获取iovec数据
pub(crate) fn read_vec_meta<T: IoVecBuf>(iovec_buf: &T) -> IoVecMeta {
    let ptr = iovec_buf.read_iovec_ptr();
    let iovec_len = iovec_buf.read_iovec_len();
    let mut data = Vec::with_capacity(iovec_len);
    let mut len = 0;
    for i in 0..iovec_len {
        let iovec = unsafe { *ptr.add(i) };
        data.push(iovec);
        len += iovec.iov_len;
    }
    IoVecMeta {
        data,
        offset: 0,
        len,
    }
}

pub(crate) fn write_vec_meta<T: IoVecBufMut>(iovec_buf: &mut T) -> IoVecMeta {
    let ptr = iovec_buf.write_iovec_ptr();
    let iovec_len = iovec_buf.write_iovec_len();
    let mut data = Vec::with_capacity(iovec_len);
    let mut len = 0;
    for i in 0..iovec_len {
        let iovec = unsafe { *ptr.add(i) };
        data.push(iovec);
        len += iovec.iov_len;
    }
    IoVecMeta {
        data,
        offset: 0,
        len,
    }
}

impl IoVecMeta {
    /// 减去消耗的buf，遍历iovec数组直到减去used个字节
    pub(crate) fn consume(&mut self, mut used: usize) {
        if used == 0 {
            return;
        }
        let mut offset = self.offset;
        while let Some(iovec) = self.data.get_mut(offset) {
            match iovec.iov_len.cmp(&used) {
                std::cmp::Ordering::Less => {
                    used -= iovec.iov_len;
                    offset += 1;
                    continue;
                }
                std::cmp::Ordering::Equal => {
                    offset += 1;
                    self.offset = offset;
                    return;
                }
                std::cmp::Ordering::Greater => {
                    unsafe { let _ = iovec.iov_base.add(used); };
                    iovec.iov_len -= used;
                    self.offset = offset;
                    return;
                }
            }
        }
        panic!("try to consume more than owned")
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

unsafe impl IoVecBuf for IoVecMeta {
    fn read_iovec_ptr(&self) -> *const libc::iovec {
        unsafe { self.data.as_ptr().add(self.offset) }
    }

    fn read_iovec_len(&self) -> usize {
        self.data.len()
    }
}

unsafe impl IoVecBufMut for IoVecMeta {
    fn write_iovec_ptr(&mut self) -> *mut libc::iovec {
        unsafe { self.data.as_mut_ptr().add(self.offset) }
    }

    fn write_iovec_len(&mut self) -> usize {
        self.data.len()
    }

    unsafe fn set_init(&mut self, pos: usize) {
        self.consume(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::VecBuf;

    #[test]
    fn test_read_vec_meta() {
        let iovec = VecBuf::from(vec![vec![0; 10], vec![0; 20], vec![0; 30]]);
        let mut meta = read_vec_meta(&iovec);
        assert_eq!(meta.len(), 60);
        assert_eq!(meta.data.len(), 3);
        assert_eq!(meta.data[0].iov_len, 10);
        assert_eq!(meta.data[1].iov_len, 20);
        assert_eq!(meta.data[2].iov_len, 30);
        meta.consume(15);
        assert_eq!(meta.offset, 1);
        assert_eq!(meta.data[meta.offset].iov_len, 15);
    }

    #[test]
    fn test_write_vec_meta() {
        let mut iovec = VecBuf::from(vec![vec![0; 10], vec![0; 20], vec![0; 30]]);
        let mut meta = write_vec_meta(&mut iovec);
        assert_eq!(meta.len(), 60);
        assert_eq!(meta.data.len(), 3);
        assert_eq!(meta.data[0].iov_len, 10);
        assert_eq!(meta.data[1].iov_len, 20);
        assert_eq!(meta.data[2].iov_len, 30);
        meta.consume(15);
        assert_eq!(meta.offset, 1);
        assert_eq!(meta.data[meta.offset].iov_len, 15);
    }
}

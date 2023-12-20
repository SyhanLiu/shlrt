mod io_buf;
pub use io_buf::{IoBuf, IoBufMut};

mod io_vec_buf;
pub use io_vec_buf::{IoVecBuf, IoVecBufMut, VecBuf};

mod slice;
pub use slice::{IoVecWrapper, IoVecWrapperMut, Slice, SliceMut};

mod raw_buf;
pub use raw_buf::{RawBuf, RawBufIovec};

mod vec_wrapper;
pub(crate) use vec_wrapper::{read_vec_meta, write_vec_meta};

pub(crate) fn deref(buf: &impl IoBuf) -> &[u8] {
    /// 强转为切片引用
    unsafe {
        std::slice::from_raw_parts(buf.read_ptr(), buf.bytes_init())
    }
}

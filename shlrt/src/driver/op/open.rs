use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use crate::driver::op::Op;

struct Open {
    pub(crate) path: CString,
    flags: i32,
    mode: libc::mode_t,
}

impl Op<Open> {
    pub(crate) fn open<P: AsRef<Path>>(path: P, options: &OpenOptions) -> io::Result<Self> {
        let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
        let flags = libc::O_CLOEXEC
            | options.access_mode()?
            | options.creation_mode()?
            | (options.custom_flags & !libc::O_ACCMODE);
        let mode = options.mode;

        Self::submit_with(Open{path, flags, mode})
    }
}
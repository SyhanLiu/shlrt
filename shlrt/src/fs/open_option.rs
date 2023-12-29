use crate::fs::files::File;
use std::future::Future;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use crate::driver::Op;
use crate::driver::shared_fd::SharedFd;

macro_rules! open_options_setter {
    ($name:ident) => {
        pub fn $name(&mut self, $name: bool) -> &mut OpenOptions {
            self.$name = $name;
            self
        }
    };
}

#[derive(Debug, Clone)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    mode: libc::mode_t,
    custom_flags: libc::c_int,
}

impl OpenOptions {
    pub(crate) fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            mode: 0o666,
            custom_flags: 0,
        }
    }

    open_options_setter!(read);
    open_options_setter!(write);
    open_options_setter!(append);
    open_options_setter!(truncate);
    open_options_setter!(create);
    open_options_setter!(create_new);

    fn access_mode(&self) -> io::Result<libc::c_int> {
        match (self.read, self.write, self.append) {
            (true, false, false) => Ok(libc::O_RDONLY),
            (false, true, false) => Ok(libc::O_WRONLY),
            (true, true, false) => Ok(libc::O_RDWR),
            (false, _, true) => Ok(libc::O_WRONLY | libc::O_APPEND),
            (true, _, true) => Ok(libc::O_RDWR | libc::O_APPEND),
            (false, false, false) => Err(io::Error::from_raw_os_error(libc::EINVAL)),
        }
    }

    fn creation_mode(&self) -> io::Result<libc::c_int> {
        match (self.write, self.append) {
            (true, false) => {}
            (false, false) => {
                if self.truncate || self.create || self.create_new {
                    return Err(io::Error::from_raw_os_error(libc::EINVAL));
                }
            }
            (_, true) => {
                if self.truncate && !self.create_new {
                    return Err(io::Error::from_raw_os_error(libc::EINVAL));
                }
            }
        }

        Ok(match (self.create, self.truncate, self.create_new) {
            (false, false, false) => 0,
            (true, false, false) => libc::O_CREAT,
            (false, true, false) => libc::O_TRUNC,
            (true, true, false) => libc::O_CREAT | libc::O_TRUNC,
            (_, _, true) => libc::O_CREAT | libc::O_EXCL,
        })
    }

    pub fn open(&self, path: impl AsRef<Path>) -> impl Future<Output=io::Result<File>> {
        async {
            let op = Op::open(path.as_ref(), self)?;

            let completion = op.await;

            Ok(File::from_shared_fd(SharedFd::new_without_register(
                completion.meta.result? as _,
            )))
        }
    }
}

impl OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.mode = mode as libc::mode_t;
        self
    }

    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.custom_flags = flags as libc::c_int;
        self
    }
}
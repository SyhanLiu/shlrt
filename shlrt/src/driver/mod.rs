use std::mem::ManuallyDrop;
use io_uring;

mod op;
mod shared_fd;
mod uring;

pub trait Driver {

}

/// 定义TLS，每个线程一个io-uring
thread_local! {}
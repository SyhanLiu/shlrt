#![allow(stable_features)]
#![feature(io_error_more)]
#![feature(return_position_impl_trait_in_trait)]

pub mod buf;
mod driver;
mod io;
mod scheduler;
mod task;
mod utils;
mod builder;
mod macros;

pub type BufResult<T, B> = (std::io::Result<T>, B);

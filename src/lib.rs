#[macro_use(value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;
#[macro_use]
extern crate simple_error;
extern crate regex;

use std::error::Error;

pub mod charsets;
pub mod generators;
pub mod mask;
mod password_entropy;
pub mod runner;
pub mod stackbuf;
pub mod wordlists;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

type BoxResult<T> = Result<T, Box<dyn Error>>;

const BUFFER_SIZE: usize = 8192;
pub const MAX_WORD_SIZE: usize = 128;

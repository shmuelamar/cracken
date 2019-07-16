#[macro_use(value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;
extern crate regex;

pub mod charsets;
pub mod generators;
pub mod mask;
pub mod runner;
pub mod stackbuf;
pub mod wordlists;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

const BUFFER_SIZE: usize = 4096;
pub const MAX_WORD_SIZE: usize = 128;

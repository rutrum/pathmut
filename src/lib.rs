#![feature(path_file_prefix)]
#![feature(iter_intersperse)]

mod app;
mod component;

pub use app::build as build_app;

pub use component::*;

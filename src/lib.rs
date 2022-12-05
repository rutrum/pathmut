#![feature(path_file_prefix)]

mod app;
mod component;

pub use app::build as build_app;

pub use component::*;

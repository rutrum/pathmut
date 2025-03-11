#![feature(path_file_prefix)]
#![feature(iter_intersperse)]

mod app;
mod command;
mod component;

pub use app::build as build_app;
pub use app::get_command;

pub use command::*;
pub use component::*;

#[derive(Debug, Clone)]
pub enum Action<'a> {
    Get,
    Set(&'a [u8]),
    Delete,
    Replace(&'a [u8]),
}

#![feature(path_file_prefix)]
#![feature(iter_intersperse)]

mod app;
mod command;
mod component;

pub use app::build as build_app;
pub use app::get_command;

pub use command::Command;
pub use command::Question;
pub use component::*;

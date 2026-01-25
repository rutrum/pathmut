#![feature(iter_intersperse)]

mod app;
mod command;
pub mod path;

pub use app::build as build_app;
pub use app::get_command;

pub use command::*;

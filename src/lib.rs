#![feature(path_file_prefix)]
#![feature(iter_intersperse)]

mod app;
mod app_new; // will replace app in future
mod command;
mod component;

pub use app_new::build as build_app;
pub use app_new::get_command;

pub use command::Command;
pub use component::*;

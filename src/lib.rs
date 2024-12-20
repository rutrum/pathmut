#![feature(path_file_prefix)]
#![feature(iter_intersperse)]

mod app;
mod app_new; // will replace app in future
mod component;
mod command;

pub use app_new::build as build_app;

pub use component::*;
pub use command::Command;
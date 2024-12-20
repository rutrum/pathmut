use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;

use crate::command::Command as PathCommand;
use crate::component::Component;

pub fn build() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings.")
        .subcommands([
            get_command(),
            remove_command(),
            replace_command(),
        ])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
        .subcommand_value_name("COMPONENT|COMMAND")
        .subcommand_help_heading("Components/Commands")
}

// todo remove
fn component_arg() -> Arg {
    Arg::new("component")
        .required(true)
        .value_parser(value_parser!(Component))
}

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Append)
        .help("Path string to mutate.")
        .value_parser(value_parser!(PathBuf))
}

fn get_command() -> Command {
    Command::new("get")
        .about("Read a file component")
        .arg_required_else_help(true)
        .args([
            component_arg(),
            path_arg(),
        ])
}

fn remove_command() -> Command {
    Command::new("remove")
        .about("Remove a file component")
        .arg_required_else_help(true)
        .arg(component_arg())
}

fn replace_command() -> Command {
    Command::new("replace")
        .about("Replace a file component")
        .arg_required_else_help(true)
        .arg(component_arg())
}

mod component {
    use super::*;

    fn all() -> Vec<Command> {
        vec!(
            first(),
        )
    }

    fn first() -> Command {
        Command::new("first")
            .about("First component")
            .arg_required_else_help(true)
            .arg(path_arg())
    }
}
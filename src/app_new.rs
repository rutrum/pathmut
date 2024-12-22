use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;

use crate::command::Command as PathCommand;
use crate::component::arg_into_component;
use crate::component::Component;

pub fn build() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings.")
        .subcommands([get_command(), remove_command(), replace_command()])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
        .subcommand_value_name("COMPONENT|COMMAND")
        .subcommand_help_heading("Components/Commands")
}

// todo remove
fn component_arg() -> Arg {
    Arg::new("component")
        .required(true)
        .value_parser(arg_into_component)
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
        .args([component_arg(), path_arg()])
    //.subcommands(component::all())
}

fn remove_command() -> Command {
    Command::new("remove")
        .about("Remove a file component")
        .arg_required_else_help(true)
        .subcommands(component::all())
}

fn replace_command() -> Command {
    // todo: fix this, it works funny since arg component without str works
    Command::new("replace")
        .about("Replace a file component")
        .arg_required_else_help(true)
        .args([Arg::new("str")
            .required(true)
            .value_parser(value_parser!(String))])
        .subcommands(component::all())
}

mod component {
    use super::*;

    pub fn all() -> Vec<Command> {
        vec![extension(), stem(), prefix(), first()]
    }

    //    Extension,
    //    Stem,
    //    Prefix,
    //    Name,
    //    Parent,
    //    First,
    pub fn extension() -> Command {
        Command::new("ext")
            .about("File extension")
            .arg_required_else_help(true)
            .arg(path_arg())
    }

    pub fn stem() -> Command {
        Command::new("stem")
            .about("File stem")
            .arg_required_else_help(true)
            .arg(path_arg())
    }

    pub fn prefix() -> Command {
        Command::new("prefix")
            .about("File prefix")
            .arg_required_else_help(true)
            .arg(path_arg())
    }

    pub fn name() -> Command {
        Command::new("name")
            .about("File name")
            .arg_required_else_help(true)
            .arg(path_arg())
    }

    pub fn parent() -> Command {
        Command::new("parent")
            .about("Parent of the file or directory")
            .arg_required_else_help(true)
            .arg(path_arg())
    }

    pub fn first() -> Command {
        Command::new("first")
            .about("First component")
            .arg_required_else_help(true)
            .arg(path_arg())
    }
}

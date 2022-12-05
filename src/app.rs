use clap::{ArgGroup, value_parser, crate_version, ArgAction, Command, Arg};

use std::path::PathBuf;

pub fn build() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings.")
        .subcommands([
            ext_command(),
            stem_command(),
            prefix_command(),
            name_command(),
            parent_command(),
            first_command(),
        ])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
        .subcommand_value_name("COMPONENT|COMMAND")
        .subcommand_help_heading("Components/Commands")
}

fn ext_command() -> Command {
    Command::new("ext")
        .about("Read or update file extension")
        .args(component_args())
        .arg_required_else_help(true)
}

fn stem_command() -> Command {
    Command::new("stem")
        .about("Read or update file stem")
        .args(component_args())
        .arg_required_else_help(true)
}

fn prefix_command() -> Command {
    Command::new("prefix")
        .about("Read or update file prefix")
        .args(component_args())
        .arg_required_else_help(true)
}

fn name_command() -> Command {
    Command::new("name")
        .about("Read or update file name")
        .args(component_args())
        .arg_required_else_help(true)
}

fn parent_command() -> Command {
    Command::new("parent")
        .about("Read or update parent directory")
        .args(component_args())
        .arg_required_else_help(true)
}

fn first_command() -> Command {
    Command::new("first")
        .about("Read or update first component")
        .args(component_args())
        .arg_required_else_help(true)
}

fn component_args() -> [Arg; 3] {
    [ path_arg(), remove_arg(), replace_arg() ]
}

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Append)
        .help("Path string to mutate.")
        .value_parser(value_parser!(PathBuf))
}

fn remove_arg() -> Arg {
    Arg::new("remove")
        .short('r')
        .long("remove")
        .action(ArgAction::SetTrue)
        .help("Remove component from path")
        .conflicts_with("replace")
}

fn replace_arg() -> Arg {
    Arg::new("replace")
        .short('s')
        .long("replace")
        .action(ArgAction::Set)
        .help("Replace component in path")
        .conflicts_with("remove")
}


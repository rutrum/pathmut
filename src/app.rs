use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;

use crate::component::arg_into_component;

pub fn build() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings.")
        .subcommands([get_command(), remove_command(), replace_command()])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
        .subcommand_value_name("COMMAND or COMPONENT")
        .allow_external_subcommands(true)
        .after_help(components_help_section())
    // could add custom list of components here
}

fn components_help_section() -> &'static str {
    "\x1B[4;1mComponents:\x1B[0m\n\
    \x20 \x1B[1mext\x1B[0m      file extension\n\
    \x20 \x1B[1mstem\x1B[0m     file stem\n\
    \x20 \x1B[1mprefix\x1B[0m   file prefix\n\
    \x20 \x1B[1mname\x1B[0m     file name\n\
    \x20 \x1B[1mparent\x1B[0m   parent of the file or directory\n\
    \x20 \x1B[1;3mn\x1B[0m        ordinal of the nth component\n"
}

fn component_arg() -> Arg {
    // todo: figure out way to list possible values
    Arg::new("component")
        .required(true)
        .value_parser(arg_into_component)
        //.value_parser(value_parser!(Component))
        .help("Path component")
}

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Append)
        .help("Path string to mutate")
        .value_parser(value_parser!(PathBuf))
}

pub fn get_command() -> Command {
    Command::new("get")
        .about("Read a file component [default]")
        .arg_required_else_help(true)
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
    //.subcommands(component::all())
}

fn remove_command() -> Command {
    Command::new("delete")
        .about("Remove a file component")
        .arg_required_else_help(true)
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

fn replace_command() -> Command {
    // todo: fix this, it works funny since arg component without str works
    Command::new("replace")
        .about("Replace a file component")
        .arg_required_else_help(true)
        .args([Arg::new("str")
            .required(true)
            .value_parser(value_parser!(String))])
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

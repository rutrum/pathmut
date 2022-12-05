use clap::{value_parser, crate_version, ArgAction, Command, Arg};

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
}

fn ext_command() -> Command {
    Command::new("ext")
        .about("Prints file extension of file in the path.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
}

fn stem_command() -> Command {
    Command::new("stem")
        .about("Prints file stem of file in the path.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
}

fn prefix_command() -> Command {
    Command::new("prefix")
        .about("Prints file prefix of file in the path.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
}

fn name_command() -> Command {
    Command::new("name")
        .about("Prints the name of the file or directory at the path.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
}

fn parent_command() -> Command {
    Command::new("parent")
        .about("Prints the path without the final file or directory.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
}

fn first_command() -> Command {
    Command::new("first")
        .about("Prints the first component.")
        .arg(path_arg())
        .arg(remove_arg())
        .arg(replace_arg())
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
}

fn replace_arg() -> Arg {
    Arg::new("replace")
        .short('s')
        .long("replace")
        .action(ArgAction::Set)
        .help("Replace component in path")
}


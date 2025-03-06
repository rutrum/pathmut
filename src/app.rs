use clap::builder::ValueParser;
use clap::{crate_version, value_parser, Arg, ArgAction, Command};

use crate::command::PathKind;
use crate::command::Question;
use crate::component::arg_into_component;

pub fn build() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings")
        .args([normalize_arg(), parse_as_unix_arg(), parse_as_win_arg()])
        .subcommands([
            get_command(),
            remove_command(),
            replace_command(),
            set_command(),
            has_command(),
            is_command(),
            normalize_command(),
            convert_command(),
            //info_command(),
        ])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
        .subcommand_value_name("COMMAND or COMPONENT")
        .allow_external_subcommands(true)
        .after_help(components_help_section())
}

//fn info_command -> Command {
//    Command::new("info")
//}

fn normalize_arg() -> Arg {
    Arg::new("normalize")
        .global(true)
        .short('n')
        .long("normalize")
        .action(ArgAction::SetTrue)
        .help("Normalize the path first")
}

fn parse_as_win_arg() -> Arg {
    Arg::new("as-windows")
        .global(true)
        .short('w')
        .long("as-windows")
        .action(ArgAction::SetTrue)
        .conflicts_with("as-unix")
        .help("Parse paths as windows paths")
}

fn parse_as_unix_arg() -> Arg {
    Arg::new("as-unix")
        .global(true)
        .short('u')
        .long("as-unix")
        .action(ArgAction::SetTrue)
        .conflicts_with("as-windows")
        .help("Parse paths as unix paths")
}

fn components_help_section() -> &'static str {
    "\x1B[4;1mComponents:\x1B[0m\n\
    \x20 \x1B[1mext\x1B[0m      File extension\n\
    \x20 \x1B[1mstem\x1B[0m     File stem\n\
    \x20 \x1B[1mprefix\x1B[0m   File prefix\n\
    \x20 \x1B[1mname\x1B[0m     File name\n\
    \x20 \x1B[1mparent\x1B[0m   Parent of the file or directory\n\
    \x20 \x1B[1;3mn\x1B[0m        Ordinal of the nth component\n"
}

fn questions_help_section() -> &'static str {
    "\x1B[4;1mQuestions:\x1B[0m\n\
    \x20 \x1B[1mabsolute\x1B[0m\n\
    \x20 \x1B[1mrelative\x1B[0m\n\
    \x20 \x1B[1munix\x1B[0m\n\
    \x20 \x1B[1mwindows\x1B[0m\n"
}

fn component_arg() -> Arg {
    // todo: figure out way to list possible values
    Arg::new("component")
        .required(true)
        .value_parser(arg_into_component)
        .allow_negative_numbers(true)
        //.value_parser(value_parser!(Component))
        .help("Path component")
}

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Append)
        .help("Path strings to mutate")
        //.value_parser(value_parser!(TypedPathBuf))
        .value_parser(ValueParser::os_string())
}

fn question_arg() -> Arg {
    Arg::new("question")
        .required(true)
        .help("Question to ask")
        .value_parser(value_parser!(Question))
}

pub fn get_command() -> Command {
    Command::new("get")
        .about("Read a path component [default]")
        .arg_required_else_help(true)
        .args([normalize_arg(), component_arg(), path_arg()])
        .after_help(components_help_section())
}

pub fn has_command() -> Command {
    Command::new("has")
        .about("Check if a path component exists")
        .arg_required_else_help(true)
        .args(true_false_args())
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

fn remove_command() -> Command {
    Command::new("delete")
        .about("Remove a path component")
        .arg_required_else_help(true)
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

fn replace_command() -> Command {
    // todo: fix this, it works funny since arg component without str works
    Command::new("replace")
        .about("Replace an existing path component")
        .arg_required_else_help(true)
        .args([Arg::new("str")
            .required(true)
            .value_parser(ValueParser::os_string())])
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

fn set_command() -> Command {
    // todo: fix this, it works funny since arg component without str works
    Command::new("set")
        .about("Set a path component")
        .arg_required_else_help(true)
        .args([Arg::new("str")
            .required(true)
            .value_parser(ValueParser::os_string())])
        .args([component_arg(), path_arg()])
        .after_help(components_help_section())
}

fn true_false_args() -> [Arg; 3] {
    let any = Arg::new("any")
        .help("[default] True if one path succeeds")
        .long("any")
        .action(ArgAction::SetTrue);
    let all = Arg::new("all")
        .help("True only if all paths succeed")
        .long("all")
        .action(ArgAction::SetTrue)
        .conflicts_with("any");
    let print = Arg::new("print")
        .help("Print 'true' or 'false' to stdout instead of exit code")
        .short('p')
        .long("print")
        .action(ArgAction::SetTrue);
    [any, all, print]
}

fn is_command() -> Command {
    Command::new("is")
        .about("Ask questions about a file path")
        .arg_required_else_help(true)
        .args(true_false_args())
        .args([question_arg(), path_arg()])
        .after_help(questions_help_section())
}

fn normalize_command() -> Command {
    Command::new("normalize")
        .about("Normalize a file path")
        .arg_required_else_help(true)
        .arg(path_arg())
}

fn path_type_arg() -> Arg {
    Arg::new("type")
        .help("Type of path")
        .required(true)
        .value_parser(value_parser!(PathKind))
}

fn convert_command() -> Command {
    Command::new("convert")
        .about("Convert between unix and windows paths")
        .arg_required_else_help(true)
        .args([path_type_arg(), path_arg()])
}

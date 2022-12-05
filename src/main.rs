#![feature(path_file_prefix)]

use clap::{crate_version, value_parser, parser::ValuesRef, Arg, ArgMatches, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::env;
use atty;
use std::io::{self, Read};

enum Component {
    Extension,
    Stem,
    Prefix,
    Name,
    Parent,
    First,
}

impl TryFrom<&str> for Component {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use Component::*;
        let comp = match s {
            "ext" => Extension,
            "stem" => Stem,
            "prefix" => Prefix,
            "name" => Name,
            "parent" => Parent,
            "first" => First,
            _ => Err(())?,
        };
        Ok(comp)
    }
}

enum Action {
    Get,
    Remove,
    //Replace,
}

mod get {
    use super::*;

    pub fn ext(path: PathBuf) -> OsString {
        path.extension().unwrap_or_default().into()
    }

    pub fn stem(path: PathBuf) -> OsString {
        path.file_stem().unwrap_or_default().into()
    }

    pub fn prefix(path: PathBuf) -> OsString {
        path.file_prefix().unwrap_or_default().into()
    }

    pub fn name(path: PathBuf) -> OsString {
        path.file_name().unwrap_or_default().into()
    }

    pub fn parent(path: PathBuf) -> OsString {
        match path.parent() {
            Some(path) => path.into(),
            None => OsString::new(),
        }
    }

    pub fn first(path: PathBuf) -> OsString {
        match path.ancestors().last() {
            Some(path) => path.into(),
            None => OsString::new(),
        }
    }
}

mod remove {
    use super::*;

    pub fn ext(path: PathBuf) -> OsString {
        path.with_extension(OsStr::new("")).into()
    }

    /*
    pub fn stem(path: PathBuf) -> OsString {

    }

    pub fn prefix(path: PathBuf) -> OsString {
        
    }

    pub fn name(path: PathBuf) -> OsString {
        
    }

    pub fn parent(path: PathBuf) -> OsString {

    }

    pub fn first(path: PathBuf) -> OsString {
        
    }
    */
}

fn main() {
    let app = build_app();

    let mut args: Vec<String> = env::args_os().map(|x| x.into_string().unwrap()).collect();

    if atty::isnt(atty::Stream::Stdin) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        let mut v = Vec::new();
        handle.read_to_end(&mut v).unwrap();

        let s = String::from_utf8(v)
            .unwrap()
            .to_string();

        if !s.is_empty() {
            args.push(s);
        }
    }

    let matches = app.get_matches_from(args);

    if let Some((subcommand, matches)) = matches.subcommand() {
        if let Ok(component) = Component::try_from(subcommand) {
            let action = if matches.get_flag("remove") {
                Action::Remove
            } else {
                Action::Get
            };

            let paths = matches.get_many::<PathBuf>("path").unwrap();
            let result = do_component_action(component, action, paths);
            println!("{}", result);
        }
    }
}

fn do_component_action(comp: Component, action: Action, paths: ValuesRef<PathBuf>) -> String {
    use Component::*;
    use Action::*;

    match (action, comp) {
        (Get, Extension) => apply_to_paths(paths, get::ext),
        (Get, Stem) => apply_to_paths(paths, get::stem),
        (Get, Prefix) => apply_to_paths(paths, get::prefix),
        (Get, Name) => apply_to_paths(paths, get::name),
        (Get, Parent) => apply_to_paths(paths, get::parent),
        (Get, First) => apply_to_paths(paths, get::first),
        (Remove, Extension) => apply_to_paths(paths, remove::ext),
        _ => unreachable!(),
    }
}

fn apply_to_paths(paths: ValuesRef<PathBuf>, f: fn(PathBuf) -> OsString) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(path.to_path_buf());
        result.extend(new.to_str());
        result.push('\n');
    }
    result.trim().to_string()
}

fn build_app() -> Command {
    Command::new("pathmut")
        .version(crate_version!())
        .about("Mutates path strings.")
        .subcommands([
            ext_command(),
            stem_command(),
            prefix_command(),
            name_command(),
            parent_command(),
        ])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
}

fn ext_command() -> Command {
    Command::new("ext")
        .about("Prints file extension of file in the path.")
        .arg(path_arg())
        .arg(remove_arg())
}

fn stem_command() -> Command {
    Command::new("stem")
        .about("Prints file stem of file in the path.")
        .arg(path_arg())
}

fn prefix_command() -> Command {
    Command::new("prefix")
        .about("Prints file prefix of file in the path.")
        .arg(path_arg())
}

fn name_command() -> Command {
    Command::new("name")
        .about("Prints the name of the file or directory at the path.")
        .arg(path_arg())
}

fn parent_command() -> Command {
    Command::new("parent")
        .about("Prints the path without the final file or directory.")
        .arg(path_arg())
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

#[cfg(test)]
mod test {
    use assert_cmd::Command;
    use predicates::prelude::*;

    #[test]
    fn ext() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("txt\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext", "/my/path/file.tar.gz"])
            .assert()
            .success()
            .stdout("gz\n");
    }

    #[test]
    fn stem() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["stem", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("file\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["stem", "/my/path/file.tar.gz"])
            .assert()
            .success()
            .stdout("file.tar\n");
    }

    #[test]
    fn prefix() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["prefix", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("file\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["prefix", "/my/path/file.tar.gz"])
            .assert()
            .success()
            .stdout("file\n");
    }

    #[test]
    fn name() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["name", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("file.txt\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["name", "/my/path/dir"])
            .assert()
            .success()
            .stdout("dir\n");
    }

    #[test]
    fn ext_remove() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext", "--remove", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("/my/path/file\n");
    }

    #[test]
    fn parent() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["parent", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("/my/path\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["parent", "/my/path/dir"])
            .assert()
            .success()
            .stdout("/my/path\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["parent", "/"])
            .assert()
            .success()
            .stdout("\n");
    }

    #[test]
    fn from_stdin() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext"])
            .write_stdin("/my/path/file.txt")
            .assert()
            .success()
            .stdout("txt\n");
    }

    #[test]
    fn help_default() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .assert()
            .failure()
            .stderr(predicate::str::contains("Print help information"));
    }

    #[test]
    fn multiple_paths() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext", "file.txt", "another.png"])
            .assert()
            .success()
            .stdout("txt\npng\n");
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["stem", "file.txt", "another.png"])
            .assert()
            .success()
            .stdout("file\nanother\n");
    }

    /*
    #[test]
    fn piped_multiple_paths() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["ext"])
            .write_stdin("/my/path/file.txt\npicture.png")
            .assert()
            .success()
            .stdout("txt\npng\n");
    }
    */
}

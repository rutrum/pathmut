#![feature(path_file_prefix)]

use clap::{crate_version, value_parser, parser::ValuesRef, Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::env;
use atty;
use std::io::{self, Read};

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

    let result = if let Some(matches) = matches.subcommand_matches("ext") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |path| path.extension().unwrap().into())

    } else if let Some(matches) = matches.subcommand_matches("stem") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |path| path.file_stem().unwrap().into())

    } else if let Some(matches) = matches.subcommand_matches("prefix") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |path| path.file_prefix().unwrap().into())

    } else if let Some(matches) = matches.subcommand_matches("name") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |path| path.file_name().unwrap().into())

    } else if let Some(matches) = matches.subcommand_matches("parent") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |path| path.parent().map(|p| p.as_os_str()).unwrap_or(OsStr::new("")).into())

    } else if let Some(matches) = matches.subcommand_matches("rmext") {
        let paths = matches.get_many::<PathBuf>("path").unwrap();
        apply_to_paths(paths, |mut path| {
            path.set_extension(OsStr::new(""));
            path.as_os_str().into()
        })

    } else {
        unreachable!()
    };

    println!("{}", result);
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
            rmext_command(),
        ])
        .dont_delimit_trailing_values(true)
        .arg_required_else_help(true)
}

fn ext_command() -> Command {
    Command::new("ext")
        .about("Prints file extension of file in the path.")
        .arg(path_arg())
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

fn rmext_command() -> Command {
    Command::new("rmext")
        .about("Removes the extension from path.")
        .arg(path_arg())
}

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Append)
        .help("Path string to mutate.")
        .value_parser(value_parser!(PathBuf))
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
    fn rmext() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["rmext", "/my/path/file.txt"])
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

#![feature(path_file_prefix)]

use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use std::ffi::OsStr;
use std::path::PathBuf;

fn main() {
    let app = build_app();
    let matches = app.get_matches();

    let mut result: Option<&OsStr> = None;

    if let Some(matches) = matches.subcommand_matches("ext") {
        let path: &PathBuf = matches.get_one("path").unwrap();
        result = path.extension();
    } else if let Some(matches) = matches.subcommand_matches("stem") {
        let path: &PathBuf = matches.get_one("path").unwrap();
        result = path.file_stem();
    } else if let Some(matches) = matches.subcommand_matches("prefix") {
        let path: &PathBuf = matches.get_one("path").unwrap();
        result = path.file_prefix();
    } else if let Some(matches) = matches.subcommand_matches("name") {
        let path: &PathBuf = matches.get_one("path").unwrap();
        result = path.file_name();
    } else if let Some(matches) = matches.subcommand_matches("parent") {
        let path: &PathBuf = matches.get_one("path").unwrap();
        result = path.parent().map(|x| x.as_os_str());
    }

    if let Some(s) = result {
        println!("{}", s.to_str().unwrap());
    }
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

fn path_arg() -> Arg {
    Arg::new("path")
        .required(true)
        .action(ArgAction::Set)
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
            .stdout("");
    }
}

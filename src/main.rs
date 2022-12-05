use clap::parser::ValuesRef;
use std::ffi::{OsString};
use std::path::PathBuf;
use std::env;
use atty;
use std::io::{self, Read};

use pathmut::*;

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
        (Remove, Name) => apply_to_paths(paths, remove::name),
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
    fn name_remove() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["name", "--remove", "/my/path/file.txt"])
            .assert()
            .success()
            .stdout("/my/path/\n");
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

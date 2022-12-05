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
    use assert_cmd::assert::Assert;
    use predicates::prelude::*;

    fn pathmut(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(args)
            .assert()
    }

    mod component {
        use super::*;
        mod get {
            use super::*;

            #[test]
            fn ext() {
                pathmut(&["ext", "/my/path/file.txt"])
                    .success()
                    .stdout("txt\n");
                pathmut(&["ext", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("gz\n");
            }

            #[test]
            fn stem() {
                pathmut(&["stem", "/my/path/file.txt"])
                    .success()
                    .stdout("file\n");
                pathmut(&["stem", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("file.tar\n");
            }

            #[test]
            fn prefix() {
                pathmut(&["prefix", "/my/path/file.txt"])
                    .success()
                    .stdout("file\n");
                pathmut(&["prefix", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("file\n");
            }

            #[test]
            fn name() {
                pathmut(&["name", "/my/path/file.txt"])
                    .success()
                    .stdout("file.txt\n");
                pathmut(&["name", "/my/path/dir"])
                    .success()
                    .stdout("dir\n");
            }

            #[test]
            fn parent() {
                pathmut(&["parent", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path\n");
                pathmut(&["parent", "/my/path/dir"])
                    .success()
                    .stdout("/my/path\n");
                pathmut(&["parent", "/"])
                    .success()
                    .stdout("\n");
            }

            #[test]
            fn first() {
                pathmut(&["first", "/"])
                    .success()
                    .stdout("/\n");
                pathmut(&["first", "/my/path/file.txt"])
                    .success()
                    .stdout("/\n");
                pathmut(&["first", "my/path/file.txt"])
                    .success()
                    .stdout("my\n");
            }
        }

        mod remove {
            use super::*;

            #[test]
            fn ext() {
                pathmut(&["ext", "--remove", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/file\n");
            }

            #[test]
            fn name() {
                pathmut(&["name", "--remove", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/\n");
            }
        }
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
        pathmut(&[])
            .failure()
            .stderr(predicate::str::contains("Print help information"));
    }

    #[test]
    fn multiple_paths() {
        pathmut(&["ext", "file.txt", "another.png"])
            .success()
            .stdout("txt\npng\n");
        pathmut(&["stem", "file.txt", "another.png"])
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

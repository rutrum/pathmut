use clap::parser::ValuesRef;
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use pathmut::*;

fn main() {
    let app = build_app();
    let stdin = io::stdin();

    // manually fetch args, so it can be overwritten by piped input
    let mut args: Vec<String> = env::args_os().map(|x| x.into_string().unwrap()).collect();

    if !stdin.is_terminal() {
        // user is piping
        let mut v = Vec::new();
        let mut handle = stdin.lock(); // sometimes freezes
        handle.read_to_end(&mut v).unwrap();
        let s = String::from_utf8(v).unwrap();
        if !s.is_empty() {
            args.push(s);
        }
    }

    let matches = app.get_matches_from(args);

    if let Some((cmd, args)) = matches.subcommand() {
        // check if cmd is a command or component
        let cmd = Command::try_from(cmd).expect("required by clap");

        let component = args.get_one::<Component>("component").expect("required");
        let paths = args.get_many::<PathBuf>("path").expect("required");

        let action = match cmd {
            Command::Get => Action::Get,
            Command::Delete => Action::Remove,
            Command::Replace => Action::Replace(args.get_one::<String>("str").expect("required")),
        };

        let result = do_component_action(*component, action, paths);
        println!("{}", result);
    }
}

fn do_component_action(comp: Component, action: Action, paths: ValuesRef<PathBuf>) -> String {
    use Action::*;
    use Component::*;

    match (action, comp) {
        (Get, Extension) => apply_to_paths(paths, get::ext),
        (Get, Stem) => apply_to_paths(paths, get::stem),
        (Get, Prefix) => apply_to_paths(paths, get::prefix),
        (Get, Name) => apply_to_paths(paths, get::name),
        (Get, Parent) => apply_to_paths(paths, get::parent),
        (Get, Nth(n)) => apply_nth_to_paths(paths, n, get::nth),
        (Remove, Extension) => apply_to_paths(paths, remove::ext),
        (Remove, Stem) => apply_to_paths(paths, remove::stem),
        (Remove, Prefix) => apply_to_paths(paths, remove::prefix),
        (Remove, Name) => apply_to_paths(paths, remove::name),
        (Remove, Parent) => apply_to_paths(paths, remove::parent),
        (Remove, Nth(n)) => apply_nth_to_paths(paths, n, remove::nth),
        (Replace(s), Extension) => apply_to_paths_replace(paths, s, replace::ext),
        (Replace(s), Stem) => apply_to_paths_replace(paths, s, replace::stem),
        (Replace(s), Prefix) => apply_to_paths_replace(paths, s, replace::prefix),
        (Replace(s), Name) => apply_to_paths_replace(paths, s, replace::name),
        (Replace(s), Parent) => apply_to_paths_replace(paths, s, replace::parent),
        (Replace(s), Nth(n)) => apply_nth_to_paths_replace(paths, s, n, replace::nth),
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

fn apply_nth_to_paths(
    paths: ValuesRef<PathBuf>,
    n: usize,
    f: fn(usize, PathBuf) -> OsString,
) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(n, path.to_path_buf());
        result.extend(new.to_str());
        result.push('\n');
    }
    result.trim().to_string()
}

fn apply_to_paths_replace(
    paths: ValuesRef<PathBuf>,
    s: &str,
    f: fn(PathBuf, &str) -> OsString,
) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(path.to_path_buf(), s);
        result.extend(new.to_str());
        result.push('\n');
    }
    result.trim().to_string()
}

fn apply_nth_to_paths_replace(
    paths: ValuesRef<PathBuf>,
    s: &str,
    n: usize,
    f: fn(usize, PathBuf, &str) -> OsString,
) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(n, path.to_path_buf(), s);
        result.extend(new.to_str());
        result.push('\n');
    }
    result.trim().to_string()
}

#[cfg(test)]
mod test {
    use assert_cmd::assert::Assert;
    use assert_cmd::Command;
    use predicates::prelude::*;

    fn pathmut(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut").unwrap().args(args).assert()
    }

    mod component {
        use super::*;
        mod get {
            use super::*;

            #[test]
            fn ext() {
                pathmut(&["get", "ext", "/my/path/file.txt"])
                    .success()
                    .stdout("txt\n");
                pathmut(&["get", "ext", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("gz\n");
            }

            #[test]
            fn stem() {
                pathmut(&["get", "stem", "/my/path/file.txt"])
                    .success()
                    .stdout("file\n");
                pathmut(&["get", "stem", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("file.tar\n");
            }

            #[test]
            fn prefix() {
                pathmut(&["get", "prefix", "/my/path/file.txt"])
                    .success()
                    .stdout("file\n");
                pathmut(&["get", "prefix", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("file\n");
            }

            #[test]
            fn name() {
                pathmut(&["get", "name", "/my/path/file.txt"])
                    .success()
                    .stdout("file.txt\n");
                pathmut(&["get", "name", "/my/path/dir"])
                    .success()
                    .stdout("dir\n");
            }

            #[test]
            fn parent() {
                pathmut(&["get", "parent", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path\n");
                pathmut(&["get", "parent", "/my/path/dir"])
                    .success()
                    .stdout("/my/path\n");
                pathmut(&["get", "parent", "/"]).success().stdout("\n");
            }

            #[test]
            fn nth_0() {
                pathmut(&["get", "0", "/"]).success().stdout("/\n");
                pathmut(&["get", "0", "/my/path/file.txt"])
                    .success()
                    .stdout("/\n");
                pathmut(&["get", "0", "my/path/file.txt"])
                    .success()
                    .stdout("my\n");
            }

            #[test]
            fn nth_1() {
                pathmut(&["get", "1", "/"]).success().stdout("\n");
                pathmut(&["get", "1", "/my/path/file.txt"])
                    .success()
                    .stdout("my\n");
                pathmut(&["get", "1", "my/path/file.txt"])
                    .success()
                    .stdout("path\n");
            }
        }

        mod remove {
            use super::*;

            // todo: test aliases

            #[test]
            fn ext() {
                pathmut(&["remove", "ext", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/file\n");
            }

            #[test]
            fn stem() {
                pathmut(&["remove", "stem", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/txt\n");
                pathmut(&["remove", "stem", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("/my/path/gz\n");
            }

            #[test]
            fn prefix() {
                pathmut(&["remove", "prefix", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("/my/path/tar.gz\n");
                pathmut(&["remove", "prefix", "/my/path/file"])
                    .success()
                    .stdout("/my/path/\n");
                pathmut(&["remove", "prefix", "/my"])
                    .success()
                    .stdout("/\n");
                pathmut(&["remove", "prefix", "/"]).success().stdout("/\n");
            }

            #[test]
            fn name() {
                pathmut(&["remove", "name", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/\n");
            }

            #[test]
            fn parent() {
                pathmut(&["remove", "parent", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("file.tar.gz\n");
                pathmut(&["remove", "parent", "/my/path"])
                    .success()
                    .stdout("path\n");
                pathmut(&["remove", "parent", "/my/path/"])
                    .success()
                    .stdout("path\n");
            }

            #[test]
            fn nth_0() {
                pathmut(&["remove", "0", "/my/path/file.txt"])
                    .success()
                    .stdout("my/path/file.txt\n");
                pathmut(&["remove", "0", "my/path/file.txt"])
                    .success()
                    .stdout("path/file.txt\n");
                pathmut(&["remove", "0", "file.txt"]).success().stdout("\n");
            }
        }

        mod replace {
            use super::*;

            #[test]
            fn ext() {
                pathmut(&["replace", "sh", "ext", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/file.sh\n");
                pathmut(&["replace", "sh", "ext", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("/my/path/file.tar.sh\n");
            }

            #[test]
            fn stem() {
                pathmut(&["replace", "main", "stem", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/main.txt\n");
                pathmut(&["replace", "main", "stem", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("/my/path/main.gz\n");
            }

            #[test]
            fn prefix() {
                pathmut(&["replace", "main", "prefix", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/main.txt\n");
                pathmut(&["replace", "main", "prefix", "/my/path/file.tar.gz"])
                    .success()
                    .stdout("/my/path/main.tar.gz\n");
            }

            #[test]
            fn name() {
                pathmut(&["replace", "main", "name", "/my/path/file.txt"])
                    .success()
                    .stdout("/my/path/main\n");
                pathmut(&["replace", "main", "name", "/my/path/"])
                    .success()
                    .stdout("/my/main\n");
                pathmut(&["replace", "main", "name", "/my/path"])
                    .success()
                    .stdout("/my/main\n");
            }

            #[test]
            fn parent() {
                pathmut(&["replace", "new/dir", "parent", "/my/path/file.txt"])
                    .success()
                    .stdout("new/dir/file.txt\n");
                pathmut(&["replace", "/", "parent", "my/path/file.txt"])
                    .success()
                    .stdout("/file.txt\n");
                pathmut(&["replace", "new", "parent", "/my/path"])
                    .success()
                    .stdout("new/path\n");
            }

            #[test]
            fn nth_0() {
                pathmut(&["replace", "new/dir", "0", "/my/path/file.txt"])
                    .success()
                    .stdout("new/dir/my/path/file.txt\n");
                pathmut(&["replace", "new/dir", "0", "my/path/file.txt"])
                    .success()
                    .stdout("new/dir/path/file.txt\n");
                pathmut(&["replace", "/", "0", "my/path/file.txt"])
                    .success()
                    .stdout("/path/file.txt\n");
            }
        }
    }

    #[test]
    fn from_stdin() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(&["get", "ext"])
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
        pathmut(&["get", "ext", "file.txt", "another.png"])
            .success()
            .stdout("txt\npng\n");
        pathmut(&["get", "stem", "file.txt", "another.png"])
            .success()
            .stdout("file\nanother\n");
    }

    #[test]
    fn no_multiple_actions() {
        pathmut(&["get", "ext", "--remove", "--replace", "a", "file.txt"]).failure();
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

use clap::parser::ValuesRef;
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;
use std::str;
use typed_path::{TypedPath, TypedPathBuf};

use pathmut::*;

fn main() -> ExitCode {
    let app = build_app();
    let stdin = io::stdin();

    /*
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

    let matches = app.get_matches_from(args.clone());
    */
    let matches = app.get_matches();

    if let Some((cmd, cmd_args)) = matches.subcommand() {
        // check if cmd is a command or component
        if let Ok(cmd) = Command::try_from(cmd) {
            // if command is is
            if let Command::Is = cmd {
                let mut paths = cmd_args.get_many::<TypedPathBuf>("path").expect("required");
                let question = cmd_args.get_one::<Question>("question").expect("required");
                let all = cmd_args.get_flag("all");
                let print = cmd_args.get_flag("print");

                let answer = match (question, all) {
                    (Question::Absolute, true) => paths.all(|path| path.is_absolute()),
                    (Question::Absolute, false) => paths.any(|path| path.is_absolute()),
                    (Question::Relative, true) => paths.all(|path| path.is_relative()),
                    (Question::Relative, false) => paths.any(|path| path.is_relative()),
                };
                if print {
                    if answer {
                        println!("true");
                    } else {
                        println!("false");
                    }
                } else if !answer {
                    return ExitCode::FAILURE;
                }
            } else {
                let component = cmd_args
                    .get_one::<Component>("component")
                    .expect("required");

                // This requires manual labor
                let path = cmd_args.get_one::<OsString>("path").expect("required");

                let action = match cmd {
                    Command::Get => Action::Get,
                    Command::Delete => Action::Delete,
                    Command::Replace => {
                        Action::Replace(cmd_args.get_one::<String>("str").expect("required"))
                    }
                    Command::Set => {
                        Action::Set(cmd_args.get_one::<String>("str").expect("required"))
                    }
                    _ => unreachable!(),
                };

                let typed_path = TypedPath::derive(path.as_encoded_bytes());

                let result = component.action(action, &typed_path);
                println!("{}", String::from_utf8_lossy(&result));

                //let result = do_component_action(*component, action, paths);
            }
        } else {
            // assume subcommand is get
            //let matches = get_command().get_matches_from(args);
            /*
            let matches = get_command().get_matches();

            let component = matches.get_one::<Component>("component").expect("required");
            let paths = matches.get_one::<TypedPathBuf>("path").expect("required");
            let action = Action::Get;

            let result = do_component_action(*component, action, paths);
            println!("{}", result);
            */
        }
    }

    ExitCode::SUCCESS
}

fn do_component_action(comp: Component, action: Action, path: &TypedPath) -> Vec<u8> {
    use Action::*;
    use Component::*;

    match (action, comp) {
        //(Get, Extension) => apply_to_paths(paths, get::ext),
        (Get, Extension) => comp.get(path),
        _ => todo!(),
    }

    /*
    match (action, comp) {
        //(Get, Extension) => apply_to_paths(paths, get::ext),
        (Get, Extension) => comp.get(path),
        // TODO: rewrite all other functions to use the same function interface
        // well, I should make sure that this first example works at all
        (Get, Stem) => apply_to_paths(paths, get::stem),
        (Get, Prefix) => apply_to_paths(paths, get::prefix),
        (Get, Name) => apply_to_paths(paths, get::name),
        _ => todo!(),
        /*
        (Get, Parent) => apply_to_paths(paths, get::parent),
        (Get, Nth(n)) => apply_nth_to_paths(paths, n, get::nth),
        (Delete, Extension) => apply_to_paths(paths, delete::ext),
        (Delete, Stem) => apply_to_paths(paths, delete::stem),
        (Delete, Prefix) => apply_to_paths(paths, delete::prefix),
        (Delete, Name) => apply_to_paths(paths, delete::name),
        (Delete, Parent) => apply_to_paths(paths, delete::parent),
        (Delete, Nth(n)) => apply_nth_to_paths(paths, n, delete::nth),
        (Replace(s), Extension) => apply_to_paths_replace(paths, s, replace::ext),
        (Replace(s), Stem) => apply_to_paths_replace(paths, s, replace::stem),
        (Replace(s), Prefix) => apply_to_paths_replace(paths, s, replace::prefix),
        (Replace(s), Name) => apply_to_paths_replace(paths, s, replace::name),
        (Replace(s), Parent) => apply_to_paths_replace(paths, s, replace::parent),
        (Replace(s), Nth(n)) => apply_nth_to_paths_replace(paths, s, n, replace::nth),
        (Set(s), Extension) => apply_to_paths_replace(paths, s, set::ext),
        (Set(s), Stem) => apply_to_paths_replace(paths, s, set::stem),
        (Set(s), Prefix) => apply_to_paths_replace(paths, s, set::prefix),
        (Set(s), Name) => apply_to_paths_replace(paths, s, set::name),
        (Set(s), Parent) => apply_to_paths_replace(paths, s, set::parent),
        (Set(s), Nth(n)) => apply_nth_to_paths_replace(paths, s, n, set::nth),
        */
    }
    */
}

fn apply_to_paths(paths: ValuesRef<TypedPathBuf>, f: fn(&TypedPathBuf) -> &[u8]) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(path.into());
        result.extend(str::from_utf8(new));
        result.push('\n');
    }
    result.trim().to_string()
}

/*
fn apply_nth_to_paths(
    paths: ValuesRef<TypedPathBuf>,
    n: usize,
    f: fn(usize, TypedPathBuf) -> OsString,
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
    paths: ValuesRef<TypedPathBuf>,
    s: &str,
    f: fn(TypedPathBuf, &str) -> OsString,
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
    paths: ValuesRef<TypedPathBuf>,
    s: &str,
    n: usize,
    f: fn(usize, TypedPathBuf, &str) -> OsString,
) -> String {
    let mut result = String::new();
    for path in paths {
        let new = f(n, path.to_path_buf(), s);
        result.extend(new.to_str());
        result.push('\n');
    }
    result.trim().to_string()
}
    */

#[cfg(test)]
mod test {
    use assert_cmd::assert::Assert;
    use assert_cmd::Command;
    use predicates::prelude::*;

    fn pathmut(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut").unwrap().args(args).assert()
    }

    mod is {
        use super::*;

        #[test]
        fn relative() {
            pathmut(&["is", "relative", "/my/path/file.txt"]).failure();
            pathmut(&["is", "relative", "my/path/file.txt"]).success();
            pathmut(&["is", "relative", "file.txt"]).success();
        }

        #[test]
        fn relative_any() {
            pathmut(&["is", "relative", "--any", "my/path/file.txt", "my/path"]).success();
            pathmut(&["is", "relative", "--any", "/my/path/file.txt", "my/path"]).success();
            pathmut(&["is", "relative", "--any", "/my/path/file.txt", "/my/path"]).failure();
            pathmut(&["is", "relative", "--any", "my/path/file.txt"]).success();
            pathmut(&["is", "relative", "--any", "/my/path/file.txt"]).failure();
        }

        #[test]
        fn relative_all() {
            pathmut(&["is", "relative", "--all", "my/path/file.txt", "my/path"]).success();
            pathmut(&["is", "relative", "--all", "/my/path/file.txt", "my/path"]).failure();
            pathmut(&["is", "relative", "--all", "/my/path/file.txt", "/my/path"]).failure();
            pathmut(&["is", "relative", "--all", "my/path/file.txt"]).success();
            pathmut(&["is", "relative", "--all", "/my/path/file.txt"]).failure();
        }

        #[test]
        fn absolute() {
            pathmut(&["is", "absolute", "/my/path/file.txt"]).success();
            pathmut(&["is", "absolute", "my/path/file.txt"]).failure();
            pathmut(&["is", "absolute", "file.txt"]).failure();
        }

        #[test]
        fn absolute_any() {
            pathmut(&["is", "absolute", "--any", "my/path/file.txt", "my/path"]).failure();
            pathmut(&["is", "absolute", "--any", "/my/path/file.txt", "my/path"]).success();
            pathmut(&["is", "absolute", "--any", "/my/path/file.txt", "/my/path"]).success();
            pathmut(&["is", "absolute", "--any", "my/path/file.txt"]).failure();
            pathmut(&["is", "absolute", "--any", "/my/path/file.txt"]).success();
        }

        #[test]
        fn absolute_all() {
            pathmut(&["is", "absolute", "--all", "my/path/file.txt", "my/path"]).failure();
            pathmut(&["is", "absolute", "--all", "/my/path/file.txt", "my/path"]).failure();
            pathmut(&["is", "absolute", "--all", "/my/path/file.txt", "/my/path"]).success();
            pathmut(&["is", "absolute", "--all", "my/path/file.txt"]).failure();
            pathmut(&["is", "absolute", "--all", "/my/path/file.txt"]).success();
        }

        #[test]
        fn absolute_print() {
            pathmut(&["is", "absolute", "-p", "/my/path/file.txt"])
                .success()
                .stdout("true\n");
            pathmut(&["is", "absolute", "-p", "my/path/file.txt"])
                .success()
                .stdout("false\n");
            pathmut(&["is", "absolute", "--print", "file.txt"])
                .success()
                .stdout("false\n");
        }

        #[test]
        fn any_all_conflict() {
            pathmut(&["is", "--all", "--any", "absolute", "/path/to/file.txt"]).failure();
        }
    }

    mod default {
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
            pathmut(&["name", "/my/path/dir"]).success().stdout("dir\n");
        }

        #[test]
        fn parent() {
            pathmut(&["parent", "/my/path/file.txt"])
                .success()
                .stdout("/my/path\n");
            pathmut(&["parent", "/my/path/dir"])
                .success()
                .stdout("/my/path\n");
            pathmut(&["parent", "/"]).success().stdout("\n");
        }

        #[test]
        fn nth_0() {
            pathmut(&["0", "/"]).success().stdout("/\n");
            pathmut(&["0", "/my/path/file.txt"]).success().stdout("/\n");
            pathmut(&["0", "my/path/file.txt"]).success().stdout("my\n");
        }

        #[test]
        fn nth_1() {
            pathmut(&["1", "/"]).success().stdout("\n");
            pathmut(&["1", "/my/path/file.txt"])
                .success()
                .stdout("my\n");
            pathmut(&["1", "my/path/file.txt"])
                .success()
                .stdout("path\n");
        }
    }

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

    mod delete {
        use super::*;

        // todo: test aliases

        #[test]
        fn ext() {
            pathmut(&["delete", "ext", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/file\n");
        }

        #[test]
        fn stem() {
            pathmut(&["delete", "stem", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/txt\n");
            pathmut(&["delete", "stem", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/gz\n");
        }

        #[test]
        fn prefix() {
            pathmut(&["delete", "prefix", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/tar.gz\n");
            pathmut(&["delete", "prefix", "/my/path/file"])
                .success()
                .stdout("/my/path/\n");
            pathmut(&["delete", "prefix", "/my"])
                .success()
                .stdout("/\n");
            pathmut(&["delete", "prefix", "/"]).success().stdout("/\n");
        }

        #[test]
        fn name() {
            pathmut(&["delete", "name", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/\n");
        }

        #[test]
        fn parent() {
            pathmut(&["delete", "parent", "/my/path/file.tar.gz"])
                .success()
                .stdout("file.tar.gz\n");
            pathmut(&["delete", "parent", "/my/path"])
                .success()
                .stdout("path\n");
            pathmut(&["delete", "parent", "/my/path/"])
                .success()
                .stdout("path\n");
        }

        #[test]
        fn nth_0() {
            pathmut(&["delete", "0", "/my/path/file.txt"])
                .success()
                .stdout("my/path/file.txt\n");
            pathmut(&["delete", "0", "my/path/file.txt"])
                .success()
                .stdout("path/file.txt\n");
            pathmut(&["delete", "0", "file.txt"]).success().stdout("\n");
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
            pathmut(&["replace", "sh", "ext", "/my/path/file"])
                .success()
                .stdout("/my/path/file\n");
        }

        #[test]
        fn stem() {
            pathmut(&["replace", "main", "stem", "/my/path/file"])
                .success()
                .stdout("/my/path/main\n");
            pathmut(&["replace", "main", "stem", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/main.txt\n");
            pathmut(&["replace", "main", "stem", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/main.gz\n");
        }

        #[test]
        fn prefix() {
            pathmut(&["replace", "main", "prefix", "/my/path/file"])
                .success()
                .stdout("/my/path/main\n");
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

        #[test]
        fn nth_1() {
            pathmut(&["replace", "new/dir", "1", "/my/path/file.txt"])
                .success()
                .stdout("/new/dir/path/file.txt\n");
            pathmut(&["replace", "new/dir", "1", "my/path/file.txt"])
                .success()
                .stdout("my/new/dir/file.txt\n");
            pathmut(&["replace", "/", "1", "my/path/file.txt"])
                .success()
                .stdout("/file.txt\n");
        }
    }

    mod set {
        use super::*;

        #[test]
        fn ext() {
            pathmut(&["set", "sh", "ext", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/file.sh\n");
            pathmut(&["set", "sh", "ext", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/file.tar.sh\n");
            pathmut(&["set", "sh", "ext", "/my/path/file"])
                .success()
                .stdout("/my/path/file.sh\n");
        }

        #[test]
        fn stem() {
            pathmut(&["set", "main", "stem", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/main.txt\n");
            pathmut(&["set", "main", "stem", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/main.gz\n");
        }

        #[test]
        fn prefix() {
            pathmut(&["set", "main", "prefix", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/main.txt\n");
            pathmut(&["set", "main", "prefix", "/my/path/file.tar.gz"])
                .success()
                .stdout("/my/path/main.tar.gz\n");
        }

        #[test]
        fn name() {
            pathmut(&["set", "main", "name", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/main\n");
            pathmut(&["set", "main", "name", "/my/path/"])
                .success()
                .stdout("/my/main\n");
            pathmut(&["set", "main", "name", "/my/path"])
                .success()
                .stdout("/my/main\n");
        }

        #[test]
        fn parent() {
            pathmut(&["set", "new/dir", "parent", "/my/path/file.txt"])
                .success()
                .stdout("new/dir/file.txt\n");
            pathmut(&["set", "/", "parent", "my/path/file.txt"])
                .success()
                .stdout("/file.txt\n");
            pathmut(&["set", "new", "parent", "/my/path"])
                .success()
                .stdout("new/path\n");
            pathmut(&["set", "new", "parent", "file.txt"])
                .success()
                .stdout("new/file.txt\n");
        }

        #[test]
        fn nth_0() {
            pathmut(&["set", "new/dir", "0", "/my/path/file.txt"])
                .success()
                .stdout("new/dir/my/path/file.txt\n");
            pathmut(&["set", "new/dir", "0", "my/path/file.txt"])
                .success()
                .stdout("new/dir/path/file.txt\n");
            pathmut(&["set", "/", "0", "my/path/file.txt"])
                .success()
                .stdout("/path/file.txt\n");
        }

        #[test]
        fn nth_1() {
            pathmut(&["set", "new/dir", "1", "/my/path/file.txt"])
                .success()
                .stdout("/new/dir/path/file.txt\n");
            pathmut(&["set", "new/dir", "1", "my/path/file.txt"])
                .success()
                .stdout("my/new/dir/file.txt\n");
        }

        #[test]
        fn root() {
            pathmut(&["set", "/", "0", "my/path/file.txt"])
                .success()
                .stdout("/path/file.txt\n");
            pathmut(&["set", "/", "1", "my/path/file.txt"])
                .success()
                .stdout("/file.txt\n");
            pathmut(&["set", "/", "2", "my/path/file.txt"])
                .success()
                .stdout("/\n");
            pathmut(&["set", "/", "3", "my/path/file.txt"])
                .success()
                .stdout("/\n");
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
        pathmut(&[
            "set",
            "blah",
            "stem",
            "path/to/file.txt",
            "just/another.png",
        ])
        .success()
        .stdout("path/to/blah.txt\njust/blah.png\n");
    }

    #[test]
    fn windows() {
        pathmut(&["get", "ext", r"C:\Users\username\file.txt"])
            .success()
            .stdout("txt\n");
        pathmut(&["get", "stem", r"C:\Users\username\file.txt"])
            .success()
            .stdout("file\n");
        pathmut(&["get", "name", r"C:\Users\username\file.txt"])
            .success()
            .stdout("file.txt\n");
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

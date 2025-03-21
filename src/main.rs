use clap::ArgMatches;
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;
use typed_path::{
    PathType, TypedComponent, TypedPath, TypedPathBuf, UnixComponent, WindowsComponent,
};

use pathmut::*;

#[derive(Debug)]
enum ParseAs {
    Derive,
    Windows,
    Unix,
}

fn parse_paths(
    args: &ArgMatches,
    normalize: bool,
    parse_as: ParseAs,
) -> impl Iterator<Item = TypedPathBuf> + '_ {
    args.get_many::<OsString>("path")
        .expect("required")
        .map(|path| path.as_encoded_bytes())
        .map(move |path| match parse_as {
            ParseAs::Derive => TypedPath::derive(path),
            ParseAs::Windows => TypedPath::windows(path),
            ParseAs::Unix => TypedPath::unix(path),
        })
        .map(move |path| {
            if normalize {
                path.normalize()
            } else {
                path.to_path_buf()
            }
        })
}

/// Equality between TypedPathBuf that don't use .components(), which
/// does not yield . and ..
fn not_normal_path_eq(left: &TypedPathBuf, right: &TypedPathBuf) -> bool {
    use TypedPathBuf::*;
    match (left, right) {
        (Unix(left), Unix(right)) => left.as_bytes() == right.as_bytes(),
        (Windows(left), Windows(right)) => left.as_bytes() == right.as_bytes(),
        _ => false,
    }
}

fn main() -> ExitCode {
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

    let matches = app.get_matches_from(args.clone());

    let normalized_first = *matches.get_one::<bool>("normalize").unwrap();
    let parse_as = if *matches.get_one::<bool>("as-windows").unwrap() {
        ParseAs::Windows
    } else if *matches.get_one::<bool>("as-unix").unwrap() {
        ParseAs::Unix
    } else {
        ParseAs::Derive
    };

    // can I hoist the path work up here?  Can I parse paths upfront?

    //let matches = app.get_matches();

    if let Some((cmd, cmd_args)) = matches.subcommand() {
        // check if cmd is a command or component
        if let Ok(cmd) = Command::try_from(cmd) {
            // if command is is
            match cmd {
                Command::Depth => {
                    let paths = parse_paths(cmd_args, normalized_first, parse_as);

                    for path in paths {
                        println!(
                            "{}",
                            path.components()
                                .filter(|c| {
                                    if let TypedComponent::Windows(WindowsComponent::Prefix(_)) = c
                                    {
                                        return false;
                                    } else {
                                        return true;
                                    }
                                })
                                .count()
                                - 1
                        );
                    }
                }
                Command::Info => {
                    let paths = parse_paths(cmd_args, normalized_first, parse_as);

                    for path in paths {
                        println!("{}", path.to_string_lossy());
                        println!(
                            "      type: {}",
                            match path {
                                TypedPathBuf::Unix(_) => "unix",
                                TypedPathBuf::Windows(_) => "windows",
                            }
                        );
                        for (component, name) in [
                            (Component::Parent, "parent"),
                            (Component::Name, "name"),
                            (Component::Prefix, "prefix"),
                            (Component::Stem, "stem"),
                            (Component::Extension, "extension"),
                        ] {
                            println!(
                                "{name:>10}: {}",
                                String::from_utf8_lossy(&component.get(&path.to_path())),
                            );
                        }

                        let mut offset = 0;
                        for c in path.components() {
                            let s = match c {
                                TypedComponent::Unix(comp) => match comp {
                                    UnixComponent::RootDir => "/",
                                    UnixComponent::Normal(slice) => &String::from_utf8_lossy(slice),
                                    _ => todo!(),
                                },
                                _ => todo!(),
                            };

                            let mut padding = String::new();
                            for _ in 0..offset {
                                padding.push_str(" ");
                            }
                            println!("{padding}{s}");
                            offset += s.len();
                            if s != "/" {
                                offset += 1;
                            }
                        }
                    }
                }
                Command::Is => {
                    let mut paths = parse_paths(cmd_args, normalized_first, parse_as);

                    let question = cmd_args.get_one::<Question>("question").expect("required");
                    let all = cmd_args.get_flag("all");
                    let print = cmd_args.get_flag("print");

                    let answer = match (question, all) {
                        (Question::Absolute, true) => paths.all(|path| path.is_absolute()),
                        (Question::Absolute, false) => paths.any(|path| path.is_absolute()),
                        (Question::Relative, true) => paths.all(|path| path.is_relative()),
                        (Question::Relative, false) => paths.any(|path| path.is_relative()),
                        (Question::Unix, true) => paths.all(|path| path.is_unix()),
                        (Question::Unix, false) => paths.any(|path| path.is_unix()),
                        (Question::Windows, true) => paths.all(|path| path.is_windows()),
                        (Question::Windows, false) => paths.any(|path| path.is_windows()),
                        (Question::Normalized, true) => {
                            paths.all(|path| not_normal_path_eq(&path, &path.normalize()))
                        }
                        (Question::Normalized, false) => {
                            paths.any(|path| not_normal_path_eq(&path, &path.normalize()))
                        }
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
                }
                Command::Has => {
                    let mut paths = parse_paths(cmd_args, normalized_first, parse_as);

                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");
                    let all = cmd_args.get_flag("all");
                    let print = cmd_args.get_flag("print");

                    let answer = if all {
                        paths.all(|path| component.has(&path.to_path()))
                    } else {
                        paths.any(|path| component.has(&path.to_path()))
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
                }
                Command::Normalize => {
                    parse_paths(cmd_args, normalized_first, parse_as)
                        .map(|path| path.normalize())
                        .for_each(|path| println!("{}", path.to_string_lossy()));
                }
                Command::Convert => {
                    let path_type: PathType =
                        (*cmd_args.get_one::<PathKind>("type").expect("required")).into();

                    let paths = parse_paths(cmd_args, normalized_first, parse_as);

                    for path in paths {
                        let converted = match path_type {
                            PathType::Unix => path.with_unix_encoding(),
                            PathType::Windows => path.with_windows_encoding(),
                        };
                        println!("{}", converted.to_string_lossy());
                    }
                }
                Command::Get | Command::Delete | Command::Replace | Command::Set => {
                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");

                    // This requires manual labor

                    let action = match cmd {
                        Command::Get => Action::Get,
                        Command::Delete => Action::Delete,
                        Command::Replace => Action::Replace(
                            cmd_args
                                .get_one::<OsString>("str")
                                .expect("required")
                                .as_encoded_bytes(),
                        ),
                        Command::Set => Action::Set(
                            cmd_args
                                .get_one::<OsString>("str")
                                .expect("required")
                                .as_encoded_bytes(),
                        ),
                        _ => unreachable!(),
                    };

                    let results = parse_paths(cmd_args, normalized_first, parse_as)
                        .map(|path| component.action(&action, &path.to_path()));

                    for result in results {
                        println!("{}", String::from_utf8_lossy(&result));
                    }
                }
            }
        } else {
            // assume subcommand is get
            let matches = get_command().get_matches_from(args);

            let action = Action::Get;
            let component = matches.get_one::<Component>("component").expect("required");

            let results = parse_paths(&matches, normalized_first, parse_as)
                .map(|path| component.action(&action, &path.to_path()));

            for result in results {
                println!("{}", String::from_utf8_lossy(&result));
            }
        }
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod test {
    use assert_cmd::assert::Assert;
    use assert_cmd::Command;
    use predicates::prelude::*;

    fn pathmut(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut").unwrap().args(args).assert()
    }

    #[test]
    fn normalize_flag() {
        pathmut(&["-n", "get", "parent", "/path/to/../file.txt"])
            .success()
            .stdout("/path\n");
        pathmut(&["-n", "replace", "md", "ext", "/path/to/../file.txt"])
            .success()
            .stdout("/path/file.md\n");
        pathmut(&["--normalize", "parent", "/path/to/../file.txt"])
            .success()
            .stdout("/path\n");
        pathmut(&["-n", "convert", "win", "/path/to/../file.txt"])
            .success()
            .stdout("\\path\\file.txt\n");
    }

    #[test]
    fn parse_as_flags() {
        pathmut(&["-w", "get", "parent", "/path/to/file.txt"])
            .success()
            .stdout("/path/to\n");
        pathmut(&["-u", "get", "parent", "C:\\path\\to\\file.txt"])
            .success()
            .stdout("\n");
    }

    #[test]
    fn depth() {
        pathmut(&["depth", "/"]).success().stdout("0\n");
        pathmut(&["depth", "/path"]).success().stdout("1\n");
        pathmut(&["depth", "/path/to"]).success().stdout("2\n");
        pathmut(&["depth", "/path/to/file"]).success().stdout("3\n");
        pathmut(&["depth", "/path/to/file.md"])
            .success()
            .stdout("3\n");

        pathmut(&["depth", "path"]).success().stdout("0\n");
        pathmut(&["depth", "path/to"]).success().stdout("1\n");
        pathmut(&["depth", "path/to/file"]).success().stdout("2\n");

        pathmut(&["depth", "C:\\"]).success().stdout("0\n");
        pathmut(&["depth", "C:\\path"]).success().stdout("1\n");
        pathmut(&["depth", "C:\\path\\to"]).success().stdout("2\n");
        pathmut(&["depth", "C:\\path\\to\\file"])
            .success()
            .stdout("3\n");

        pathmut(&["depth", "C:path"]).success().stdout("0\n");
        pathmut(&["depth", "C:path\\to"]).success().stdout("1\n");
        pathmut(&["depth", "C:path\\to\\file"])
            .success()
            .stdout("2\n");
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

        #[test]
        fn windows() {
            pathmut(&["is", "windows", r"C:\my\path"]).success();
            pathmut(&["is", "unix", r"C:\my\path"]).failure();
            pathmut(&["is", "windows", r"\my\path"]).success();
            pathmut(&["is", "unix", r"\my\path"]).failure();

            // TODO: Make my own heuristic, I guess
            // pathmut(&["is", "windows", r"my\path"]).success();
            // pathmut(&["is", "unix", r"my\path"]).failure();
        }

        #[test]
        fn unix() {
            pathmut(&["is", "windows", r"/my/path"]).failure();
            pathmut(&["is", "unix", r"/my/path"]).success();
            pathmut(&["is", "windows", r"my/path"]).failure();
            pathmut(&["is", "unix", r"my/path"]).success();
        }

        #[test]
        fn normalized() {
            pathmut(&["is", "normalized", "/my/path"]).success();
            pathmut(&["is", "normalized", "/my/./path"]).failure();
            pathmut(&["is", "normalized", "/my/../path"]).failure();
            pathmut(&["is", "normalized", "/my//path"]).failure();
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
        fn nth_n1() {
            // can't use hyphens in subcommands
            pathmut(&["-1", "/"]).failure();
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

    mod has {
        use super::*;

        #[test]
        fn ext() {
            pathmut(&["has", "ext", "/my/path/file.txt"]).success();
            pathmut(&["has", "ext", "/my/path/file.tar.gz"]).success();
            pathmut(&["has", "ext", "/my/path/file"]).failure();
        }

        #[test]
        fn stem() {
            pathmut(&["has", "stem", "/my/path/file.txt"]).success();
            pathmut(&["has", "stem", "/my/path/file.tar.gz"]).success();
            pathmut(&["has", "stem", "/my/path"]).success();
            pathmut(&["has", "stem", "/"]).failure();
        }

        #[test]
        fn prefix() {
            pathmut(&["has", "prefix", "/my/path/file.txt"]).success();
            pathmut(&["has", "prefix", "/my/path/file.tar.gz"]).success();
            pathmut(&["has", "prefix", "/my/path"]).success();
            pathmut(&["has", "prefix", "/"]).failure();
        }

        #[test]
        fn name() {
            pathmut(&["has", "name", "/my/path/file.txt"]).success();
            pathmut(&["has", "name", "/my/path/dir"]).success();
            pathmut(&["has", "name", "/"]).failure();
        }

        #[test]
        fn parent() {
            pathmut(&["has", "parent", "/my/path/file.txt"]).success();
            pathmut(&["has", "parent", "/my/path/dir"]).success();
            pathmut(&["has", "parent", "/my"]).success();
            pathmut(&["has", "parent", "/"]).failure();
        }

        #[test]
        fn nth_n2() {
            pathmut(&["has", "-2", "/"]).failure();
            pathmut(&["has", "-2", "/my/path/file.txt"]).success();
        }

        #[test]
        fn nth_n1() {
            pathmut(&["has", "-1", "/"]).success();
            pathmut(&["has", "-1", "/my/path/file.txt"]).success();
        }

        #[test]
        fn nth_0() {
            pathmut(&["has", "0", "/"]).success();
            pathmut(&["has", "0", "/my/path/file.txt"]).success();
            pathmut(&["has", "0", "my/path/file.txt"]).success();
        }

        #[test]
        fn nth_1() {
            pathmut(&["has", "1", "/"]).failure();
            pathmut(&["has", "1", "/my/path/file.txt"]).success();
            pathmut(&["has", "1", "my/path/file.txt"]).success();
        }

        #[test]
        fn print() {
            pathmut(&["has", "ext", "-p", "/path/to.txt"])
                .success()
                .stdout("true\n");
            pathmut(&["has", "-p", "ext", "/path/to.txt"])
                .success()
                .stdout("true\n");
            pathmut(&["has", "-p", "ext", "/path/to"])
                .success()
                .stdout("false\n");
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
        fn nth_n1() {
            pathmut(&["get", "-1", "/"]).success().stdout("/\n");
            pathmut(&["get", "-1", "/my/path/file.txt"])
                .success()
                .stdout("file.txt\n");
            pathmut(&["get", "-1", "my/path/file.txt"])
                .success()
                .stdout("file.txt\n");
            pathmut(&["get", "-1", "my/path/file"])
                .success()
                .stdout("file\n");
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

        #[test]
        fn nth_last() {
            pathmut(&["get", "0", "/"]).success().stdout("/\n");
            pathmut(&["get", "3", "/my/path/file.txt"])
                .success()
                .stdout("file.txt\n");
            pathmut(&["get", "2", "my/path/file.txt"])
                .success()
                .stdout("file.txt\n");
            pathmut(&["get", "2", "my/path/file"])
                .success()
                .stdout("file\n");
        }

        #[test]
        fn nth_outside() {
            // index == 1 more than num components
            pathmut(&["get", "1", "/"]).success().stdout("\n");
            pathmut(&["get", "4", "/my/path/file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["get", "3", "my/path/file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["get", "3", "my/path/file"])
                .success()
                .stdout("\n");

            // index == - num components
            pathmut(&["get", "-2", "/"]).success().stdout("\n");
            pathmut(&["get", "-5", "/my/path/file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["get", "-4", "my/path/file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["get", "-4", "my/path/file"])
                .success()
                .stdout("\n");
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
                .stdout("/my/path\n");
            pathmut(&["delete", "prefix", "/my"])
                .success()
                .stdout("/\n");
            pathmut(&["delete", "prefix", "/"]).success().stdout("/\n");
        }

        #[test]
        fn name() {
            pathmut(&["delete", "name", "/my/path/file.txt"])
                .success()
                .stdout("/my/path\n");
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
        fn nth_n1() {
            pathmut(&["delete", "-1", "/my/path/file.txt"])
                .success()
                .stdout("/my/path\n");
            pathmut(&["delete", "-1", "my/path/file.txt"])
                .success()
                .stdout("my/path\n");
            pathmut(&["delete", "-1", "file.txt"])
                .success()
                .stdout("\n");
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
        fn nth_n1() {
            pathmut(&["replace", "new/dir", "-1", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/new/dir\n");

            // replacing a later component with root makes it root
            pathmut(&["replace", "/", "-1", "my/path/file.txt"])
                .success()
                .stdout("/\n");
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
        fn nth_n1() {
            pathmut(&["set", "new/dir", "-1", "/my/path/file.txt"])
                .success()
                .stdout("/my/path/new/dir\n");
            pathmut(&["set", "new/dir", "-1", "/my/path/file"])
                .success()
                .stdout("/my/path/new/dir\n");
            pathmut(&["set", "/", "-1", "my/path/file.txt"])
                .success()
                .stdout("/\n");
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

            // TODO: should this error or not?
            // this is two things: setting any thing >= num components to root is root
            // setting something at num_components feels like "insertion"
            pathmut(&["set", "/", "3", "my/path/file.txt"])
                .success()
                .stdout("/\n");
        }
    }

    #[test]
    fn normalize() {
        pathmut(&["normalize", "my/path/file.txt"])
            .success()
            .stdout("my/path/file.txt\n");
        pathmut(&["normalize", r"C:\my\path\file.txt"])
            .success()
            .stdout("C:\\my\\path\\file.txt\n");
        pathmut(&["normalize", "my/path/.."])
            .success()
            .stdout("my\n");
        pathmut(&["normalize", r"C:\my\path\.."])
            .success()
            .stdout("C:\\my\n");
        pathmut(&["normalize", "my/./path"])
            .success()
            .stdout("my/path\n");
        pathmut(&["normalize", r"C:\my\.\path"])
            .success()
            .stdout("C:\\my\\path\n");
    }

    #[test]
    fn normalize_not_default() {
        pathmut(&["get", "parent", "my/path/./dir/file.txt"])
            .success()
            .stdout("my/path/./dir\n");
        pathmut(&["get", "parent", "my/path/../file.txt"])
            .success()
            .stdout("my/path/..\n");
    }

    #[test]
    fn convert() {
        pathmut(&["convert", "unix", "my/path/file.txt"])
            .success()
            .stdout("my/path/file.txt\n");
        pathmut(&["convert", "win", "my/path/file.txt"])
            .success()
            .stdout("my\\path\\file.txt\n");
        pathmut(&["convert", "unix", "\\my\\path\\file.txt"])
            .success()
            .stdout("/my/path/file.txt\n");
        pathmut(&["convert", "win", "my\\path\\file.txt"])
            .success()
            .stdout("my\\path\\file.txt\n");
        pathmut(&["convert", "unix", "C:\\my\\path\\file.txt"])
            .success()
            .stdout("/my/path/file.txt\n");
    }

    #[test]
    fn from_stdin() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(["get", "ext"])
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
        pathmut(&["ext", "file.txt", "another.png"])
            .success()
            .stdout("txt\npng\n");
        pathmut(&["get", "stem", "file.txt", "another.png"])
            .success()
            .stdout("file\nanother\n");
        pathmut(&["stem", "file.txt", "another.png"])
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

use clap::ArgMatches;
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use pathmut::path::{Component, Path};
use pathmut::{build_app, get_command, Command, PathKind, Question};

#[derive(Debug, Clone, Copy)]
enum ParseAs {
    Derive,
    Windows,
    Unix,
    Url,
}

fn parse_paths(args: &ArgMatches, parse_as: ParseAs) -> impl Iterator<Item = Path> + '_ {
    args.get_many::<OsString>("path")
        .expect("required")
        .map(|path| path.to_string_lossy().to_string())
        .map(move |path| match parse_as {
            ParseAs::Derive => Path::parse(&path),
            ParseAs::Windows => Path::parse_as_windows(&path),
            ParseAs::Unix => Path::parse_as_unix(&path),
            ParseAs::Url => Path::parse_as_url(&path)
                .map(|(p, _)| p)
                .expect("valid url"),
        })
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

    let parse_as = if *matches.get_one::<bool>("as-windows").unwrap_or(&false) {
        ParseAs::Windows
    } else if *matches.get_one::<bool>("as-unix").unwrap_or(&false) {
        ParseAs::Unix
    } else if *matches.get_one::<bool>("as-url").unwrap_or(&false) {
        ParseAs::Url
    } else {
        ParseAs::Derive
    };

    // can I hoist the path work up here?  Can I parse paths upfront?

    //let matches = app.get_matches();

    if let Some((cmd, cmd_args)) = matches.subcommand() {
        // check if cmd is a command or component
        if let Ok(cmd) = Command::try_from(cmd) {
            match cmd {
                Command::Depth => {
                    for path in parse_paths(cmd_args, parse_as) {
                        println!("{}", path.depth());
                    }
                }
                Command::Info => {
                    for path in parse_paths(cmd_args, parse_as) {
                        let serialized = path.serialize();
                        println!("{}", serialized);
                        println!(
                            "      type: {}",
                            if path.is_url() {
                                "url"
                            } else if path.is_unix() {
                                "unix"
                            } else {
                                "windows"
                            }
                        );

                        // Show file components
                        for (component, name) in [
                            (Component::Name, "name"),
                            (Component::FilePrefix, "prefix"),
                            (Component::Stem, "stem"),
                            (Component::Extension, "extension"),
                        ] {
                            let value = path.get(component);
                            if !value.is_empty() {
                                println!("{name:>10}: {}", value);
                            }
                        }

                        // Show URL-specific components if it's a URL
                        if path.is_url() {
                            for (component, name) in [
                                (Component::Scheme, "scheme"),
                                (Component::Host, "host"),
                                (Component::Port, "port"),
                                (Component::Path, "path"),
                                (Component::Queries, "query"),
                                (Component::Fragment, "fragment"),
                            ] {
                                let value = path.get(component);
                                if !value.is_empty() {
                                    println!("{name:>10}: {}", value);
                                }
                            }
                        }

                        // Show Windows-specific components
                        if path.is_windows() {
                            let disk = path.get(Component::Disk);
                            if !disk.is_empty() {
                                println!("{:>10}: {}", "disk", disk);
                            }
                        }
                    }
                }
                Command::Is => {
                    let mut paths = parse_paths(cmd_args, parse_as);

                    let question = cmd_args.get_one::<Question>("question").expect("required");
                    let all = cmd_args.get_flag("all");
                    let print = cmd_args.get_flag("print");

                    let answer = match (question, all) {
                        (Question::Absolute, true) => paths.all(|path| {
                            path.serialize().starts_with('/')
                                || path.is_windows() && path.get(Component::Disk).len() > 0
                        }),
                        (Question::Absolute, false) => paths.any(|path| {
                            path.serialize().starts_with('/')
                                || path.is_windows() && path.get(Component::Disk).len() > 0
                        }),
                        (Question::Relative, true) => paths.all(|path| {
                            !path.serialize().starts_with('/')
                                && !(path.is_windows() && path.get(Component::Disk).len() > 0)
                        }),
                        (Question::Relative, false) => paths.any(|path| {
                            !path.serialize().starts_with('/')
                                && !(path.is_windows() && path.get(Component::Disk).len() > 0)
                        }),
                        (Question::Unix, true) => paths.all(|path| path.is_unix()),
                        (Question::Unix, false) => paths.any(|path| path.is_unix()),
                        (Question::Windows, true) => paths.all(|path| path.is_windows()),
                        (Question::Windows, false) => paths.any(|path| path.is_windows()),
                        (Question::Url, true) => paths.all(|path| path.is_url()),
                        (Question::Url, false) => paths.any(|path| path.is_url()),
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
                    let mut paths = parse_paths(cmd_args, parse_as);

                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");
                    let all = cmd_args.get_flag("all");
                    let print = cmd_args.get_flag("print");

                    let answer = if all {
                        paths.all(|path| path.has(*component))
                    } else {
                        paths.any(|path| path.has(*component))
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
                Command::Convert => {
                    let path_kind = cmd_args.get_one::<PathKind>("type").expect("required");

                    for path in parse_paths(cmd_args, parse_as) {
                        let converted = match path_kind {
                            PathKind::Unix => path.to_unix(),
                            PathKind::Windows => path.to_windows(),
                        };
                        println!("{}", converted.serialize());
                    }
                }
                Command::Get => {
                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");

                    for path in parse_paths(cmd_args, parse_as) {
                        println!("{}", path.get(*component));
                    }
                }
                Command::Delete => {
                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");

                    for mut path in parse_paths(cmd_args, parse_as) {
                        path.delete(*component);
                        println!("{}", path.serialize());
                    }
                }
                Command::Set => {
                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");
                    let value = cmd_args
                        .get_one::<OsString>("str")
                        .expect("required")
                        .to_string_lossy();

                    for mut path in parse_paths(cmd_args, parse_as) {
                        path.set(*component, &value);
                        println!("{}", path.serialize());
                    }
                }
                Command::Replace => {
                    let component = cmd_args
                        .get_one::<Component>("component")
                        .expect("required");
                    let value = cmd_args
                        .get_one::<OsString>("str")
                        .expect("required")
                        .to_string_lossy();

                    for mut path in parse_paths(cmd_args, parse_as) {
                        path.replace(*component, &value);
                        println!("{}", path.serialize());
                    }
                }
            }
        } else {
            // assume subcommand is get (default command)
            let matches = get_command().get_matches_from(args);

            let component = matches.get_one::<Component>("component").expect("required");

            for path in parse_paths(&matches, parse_as) {
                println!("{}", path.get(*component));
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
    fn parse_as_flags() {
        // -x forces unix parsing, -w forces windows
        pathmut(&["-w", "get", "ext", "/path/to/file.txt"])
            .success()
            .stdout("txt\n");
        pathmut(&["-x", "get", "ext", "C:\\path\\to\\file.txt"])
            .success()
            .stdout("txt\n");
    }

    #[test]
    fn depth() {
        // linux absolute
        pathmut(&["depth", "/"]).success().stdout("0\n");
        pathmut(&["depth", "/path"]).success().stdout("1\n");
        pathmut(&["depth", "/path/to"]).success().stdout("2\n");
        pathmut(&["depth", "/path/to/file"]).success().stdout("3\n");
        pathmut(&["depth", "/path/to/file.md"])
            .success()
            .stdout("3\n");

        // linux relative
        pathmut(&["depth", "path"]).success().stdout("0\n");
        pathmut(&["depth", "path/to"]).success().stdout("1\n");
        pathmut(&["depth", "path/to/file"]).success().stdout("2\n");

        // windows absolute
        pathmut(&["depth", "C:\\"]).success().stdout("0\n");
        pathmut(&["depth", "C:\\path"]).success().stdout("1\n");
        pathmut(&["depth", "C:\\path\\to"]).success().stdout("2\n");
        pathmut(&["depth", "C:\\path\\to\\file"])
            .success()
            .stdout("3\n");

        // windows relative
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
        fn disk() {
            pathmut(&["disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("C\n");
            pathmut(&["disk", "\\path\\to\\file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["disk", "d:\\path\\to\\file.txt"])
                .success()
                .stdout("D\n"); // FIXME: this performs capitalization on my behalf, which isn't what I want
            pathmut(&["disk", "/linux/path"]).success().stdout("\n");
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
        fn disk() {
            pathmut(&["has", "disk", "/path/to/file.txt"]).failure();
            pathmut(&["has", "disk", "C:\\path\\to\\file.txt"]).success();
            pathmut(&["has", "disk", "d:\\path\\to\\file.txt"]).success();
            pathmut(&["has", "disk", "\\path\\to\\file.txt"]).failure();
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
        fn disk() {
            pathmut(&["get", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("C\n");
            pathmut(&["get", "disk", "\\path\\to\\file.txt"])
                .success()
                .stdout("\n");
            pathmut(&["get", "disk", "d:\\path\\to\\file.txt"])
                .success()
                .stdout("D\n"); // FIXME: this performs capitalization on my behalf, which isn't what I want
            pathmut(&["get", "disk", "/linux/path"])
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
                .stdout("/my/path/.tar.gz\n");
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
                .stdout("/my/path\n");
        }

        #[test]
        fn disk() {
            pathmut(&["delete", "disk", "/path/to/file.txt"])
                .success()
                .stdout("/path/to/file.txt\n");
            pathmut(&["delete", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("\\path\\to\\file.txt\n");
            pathmut(&["delete", "disk", "d:\\path\\to\\file.txt"])
                .success()
                .stdout("\\path\\to\\file.txt\n");
            pathmut(&["delete", "disk", "\\path\\to\\file.txt"])
                .success()
                .stdout("\\path\\to\\file.txt\n");
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
                .stdout("/my/path/main.file\n");
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
                .stdout("/my/path/main.file\n");
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
        fn disk() {
            pathmut(&["replace", "C", "disk", "/path/to/file.txt"])
                .success()
                .stdout("/path/to/file.txt\n");
            pathmut(&["replace", "C", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("C:\\path\\to\\file.txt\n");
            pathmut(&["replace", "d", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("d:\\path\\to\\file.txt\n");
            pathmut(&["replace", "C", "disk", "\\path\\to\\file.txt"])
                .success()
                .stdout("\\path\\to\\file.txt\n");
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
        fn disk() {
            pathmut(&["set", "C", "disk", "/path/to/file.txt"])
                .success()
                .stdout("/path/to/file.txt\n");
            pathmut(&["set", "C", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("C:\\path\\to\\file.txt\n");
            pathmut(&["set", "d", "disk", "C:\\path\\to\\file.txt"])
                .success()
                .stdout("d:\\path\\to\\file.txt\n");
            pathmut(&["set", "C", "disk", "\\path\\to\\file.txt"])
                .success()
                .stdout("C:\\path\\to\\file.txt\n");
        }
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

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
    use rstest::rstest;

    fn pathmut(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut").unwrap().args(args).assert()
    }

    fn pathmut_stdout(args: &[&str]) -> String {
        let output = Command::cargo_bin("pathmut")
            .unwrap()
            .args(args)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    // =========================================================================
    // Combined get/has test - verifies get output AND has correctness
    // =========================================================================
    #[rstest]
    // Unix paths - with extension
    #[case("ext", "/my/path/file.txt", "txt")]
    #[case("stem", "/my/path/file.txt", "file")]
    #[case("prefix", "/my/path/file.txt", "file")]
    #[case("name", "/my/path/file.txt", "file.txt")]
    #[case("disk", "/my/path/file.txt", "")]
    // Unix paths - multiple extensions
    #[case("ext", "/my/path/file.tar.gz", "gz")]
    #[case("stem", "/my/path/file.tar.gz", "file.tar")]
    #[case("prefix", "/my/path/file.tar.gz", "file")]
    #[case("name", "/my/path/file.tar.gz", "file.tar.gz")]
    // Unix paths - no extension
    #[case("ext", "/my/path/file", "")]
    #[case("stem", "/my/path/file", "file")]
    #[case("prefix", "/my/path/file", "file")]
    #[case("name", "/my/path/file", "file")]
    // Unix root path
    #[case("ext", "/", "")]
    #[case("stem", "/", "")]
    #[case("prefix", "/", "")]
    #[case("name", "/", "")]
    // Windows paths
    #[case("ext", r"C:\path\to\file.txt", "txt")]
    #[case("stem", r"C:\path\to\file.txt", "file")]
    #[case("name", r"C:\path\to\file.txt", "file.txt")]
    #[case("disk", r"C:\path\to\file.txt", "C")]
    #[case("disk", r"d:\path\to\file.txt", "D")]
    #[case("disk", r"\path\to\file.txt", "")]
    // URLs - file components
    #[case("ext", "https://example.com/path/file.txt", "txt")]
    #[case("stem", "https://example.com/path/file.txt", "file")]
    #[case("name", "https://example.com/path/file.txt", "file.txt")]
    // URLs - url-specific components
    #[case("scheme", "https://example.com/path", "https")]
    #[case("host", "https://example.com/path", "example.com")]
    #[case("port", "https://example.com/path", "")]
    #[case("port", "https://example.com:8080/path", "8080")]
    #[case("query", "https://example.com?foo=bar", "?foo=bar")]
    #[case("query", "https://example.com/path", "")]
    #[case("frag", "https://example.com#section", "section")]
    #[case("frag", "https://example.com/path", "")]
    // URLs - auth components
    #[case("user", "https://user:pass@example.com", "user")]
    #[case("pass", "https://user:pass@example.com", "pass")]
    #[case("user", "https://example.com", "")]
    #[case("pass", "https://example.com", "")]
    fn test_get_has(#[case] component: &str, #[case] path: &str, #[case] expected: &str) {
        // Test get
        pathmut(&["get", component, path])
            .success()
            .stdout(format!("{expected}\n"));

        // Test has - should succeed iff expected is non-empty
        let has_result = pathmut(&["has", component, path]);
        if expected.is_empty() {
            has_result.failure();
        } else {
            has_result.success();
        }
    }

    // =========================================================================
    // Delete test
    // =========================================================================
    #[rstest]
    // Unix paths
    #[case("ext", "/my/path/file.txt", "/my/path/file")]
    #[case("stem", "/my/path/file.txt", "/my/path/txt")]
    #[case("stem", "/my/path/file.tar.gz", "/my/path/gz")]
    #[case("prefix", "/my/path/file.tar.gz", "/my/path/.tar.gz")]
    #[case("prefix", "/my/path/file", "/my/path/")]
    #[case("prefix", "/my", "/")]
    #[case("prefix", "/", "/")]
    #[case("name", "/my/path/file.txt", "/my/path")]
    // Windows paths
    #[case("ext", r"C:\path\to\file.txt", r"C:\path\to\file")]
    #[case("disk", r"C:\path\to\file.txt", r"\path\to\file.txt")]
    #[case("disk", r"d:\path\to\file.txt", r"\path\to\file.txt")]
    #[case("disk", r"\path\to\file.txt", r"\path\to\file.txt")]
    #[case("disk", "/path/to/file.txt", "/path/to/file.txt")]
    // URLs
    #[case(
        "query",
        "https://example.com/path?foo=bar",
        "https://example.com/path"
    )]
    #[case("frag", "https://example.com/path#section", "https://example.com/path")]
    #[case("port", "https://example.com:8080/path", "https://example.com/path")]
    fn test_delete(#[case] component: &str, #[case] path: &str, #[case] expected: &str) {
        pathmut(&["delete", component, path])
            .success()
            .stdout(format!("{expected}\n"));
    }

    // =========================================================================
    // Set test - also verifies replace behavior automatically
    // Replace should equal set when `has` is true, otherwise no-op
    // =========================================================================
    #[rstest]
    // Extension
    #[case("ext", "/path/file.txt", "NEW", "/path/file.NEW")]
    #[case("ext", "/path/file.tar.gz", "NEW", "/path/file.tar.NEW")]
    #[case("ext", "/path/file", "NEW", "/path/file.NEW")]
    // Stem
    #[case("stem", "/path/file.txt", "NEW", "/path/NEW.txt")]
    #[case("stem", "/path/file.tar.gz", "NEW", "/path/NEW.gz")]
    // Prefix
    #[case("prefix", "/path/file.txt", "NEW", "/path/NEW.txt")]
    #[case("prefix", "/path/file.tar.gz", "NEW", "/path/NEW.tar.gz")]
    // Name
    #[case("name", "/path/file.txt", "NEW", "/path/NEW")]
    #[case("name", "/my/path/", "NEW", "/my/NEW")]
    #[case("name", "/my/path", "NEW", "/my/NEW")]
    // Disk - Unix (no disk exists)
    #[case("disk", "/path/file.txt", "C", "/path/file.txt")]
    // Disk - Windows
    #[case("disk", r"C:\path\file.txt", "D", r"D:\path\file.txt")]
    #[case("disk", r"C:\path\file.txt", "d", r"d:\path\file.txt")]
    #[case("disk", r"\path\file.txt", "C", r"C:\path\file.txt")]
    // URL components
    #[case("scheme", "https://example.com", "ftp", "ftp://example.com")]
    #[case(
        "port",
        "https://example.com/path",
        "8080",
        "https://example.com:8080/path"
    )]
    #[case(
        "frag",
        "https://example.com/path",
        "top",
        "https://example.com/path#top"
    )]
    #[case(
        "query",
        "https://example.com/path",
        "a=b",
        "https://example.com/path?a=b"
    )]
    fn test_set(
        #[case] component: &str,
        #[case] path: &str,
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        // Test set
        pathmut(&["set", value, component, path])
            .success()
            .stdout(format!("{expected}\n"));

        // Test replace: if has returns success, replace should equal set; otherwise no-op
        let has_result = pathmut(&["has", component, path]);
        let replace_out = pathmut_stdout(&["replace", value, component, path]);
        if has_result.try_success().is_ok() {
            assert_eq!(
                replace_out,
                format!("{expected}\n"),
                "replace should equal set when component exists"
            );
        } else {
            assert_eq!(
                replace_out,
                format!("{path}\n"),
                "replace should be no-op when component doesn't exist"
            );
        }
    }

    // =========================================================================
    // Shorthand equivalence test - verifies `ext` == `get ext`
    // =========================================================================
    #[rstest]
    #[case("ext")]
    #[case("stem")]
    #[case("prefix")]
    #[case("name")]
    fn test_shorthand_equals_get(#[case] component: &str) {
        let paths = [
            "/my/path/file.txt",
            r"C:\path\file.txt",
            "https://example.com/file.txt",
        ];
        for path in paths {
            let get_out = pathmut_stdout(&["get", component, path]);
            let short_out = pathmut_stdout(&[component, path]);
            assert_eq!(
                get_out, short_out,
                "shorthand '{component}' should equal 'get {component}' for {path}"
            );
        }
    }

    // =========================================================================
    // Is command tests
    // =========================================================================
    #[rstest]
    #[case("relative", "/absolute/path", false)]
    #[case("relative", "relative/path", true)]
    #[case("relative", "file.txt", true)]
    #[case("absolute", "/absolute/path", true)]
    #[case("absolute", "relative/path", false)]
    #[case("absolute", "file.txt", false)]
    #[case("unix", "/unix/path", true)]
    #[case("unix", "my/path", true)]
    #[case("unix", r"C:\windows", false)]
    #[case("unix", r"\my\path", false)]
    #[case("windows", r"C:\windows", true)]
    #[case("windows", r"\my\path", true)]
    #[case("windows", "/unix/path", false)]
    #[case("windows", "my/path", false)]
    #[case("url", "https://example.com", true)]
    #[case("url", "/not/a/url", false)]
    fn test_is(#[case] question: &str, #[case] path: &str, #[case] expected: bool) {
        let result = pathmut(&["is", question, path]);
        if expected {
            result.success();
        } else {
            result.failure();
        }
    }

    #[test]
    fn test_is_print_flag() {
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
    fn test_is_any_all_flags() {
        // --any: success if ANY path matches
        pathmut(&["is", "absolute", "--any", "/abs", "rel"]).success();
        pathmut(&["is", "absolute", "--any", "rel1", "rel2"]).failure();
        pathmut(&["is", "relative", "--any", "/abs", "rel"]).success();
        pathmut(&["is", "relative", "--any", "/abs1", "/abs2"]).failure();

        // --all: success only if ALL paths match
        pathmut(&["is", "absolute", "--all", "/abs1", "/abs2"]).success();
        pathmut(&["is", "absolute", "--all", "/abs", "rel"]).failure();
        pathmut(&["is", "relative", "--all", "rel1", "rel2"]).success();
        pathmut(&["is", "relative", "--all", "/abs", "rel"]).failure();

        // conflict
        pathmut(&["is", "--all", "--any", "absolute", "/path"]).failure();
    }

    #[test]
    fn test_has_print_flag() {
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

    // =========================================================================
    // Standalone tests - unique behaviors
    // =========================================================================

    #[test]
    fn test_parse_as_flags() {
        // -x forces unix parsing, -w forces windows
        pathmut(&["-w", "get", "ext", "/path/to/file.txt"])
            .success()
            .stdout("txt\n");
        pathmut(&["-x", "get", "ext", "C:\\path\\to\\file.txt"])
            .success()
            .stdout("txt\n");
        // -u forces url parsing
        pathmut(&["-u", "get", "scheme", "https://example.com"])
            .success()
            .stdout("https\n");
    }

    #[test]
    fn test_depth() {
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

    #[test]
    fn test_convert() {
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
    fn test_from_stdin() {
        Command::cargo_bin("pathmut")
            .unwrap()
            .args(["get", "ext"])
            .write_stdin("/my/path/file.txt")
            .assert()
            .success()
            .stdout("txt\n");
    }

    #[test]
    fn test_help_default() {
        pathmut(&[])
            .failure()
            .stderr(predicate::str::contains("Print help"));
    }

    #[test]
    fn test_multiple_paths() {
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

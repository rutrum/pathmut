//! pathmut2 - Ordered operations CLI for path manipulation
//!
//! This is a reimplementation of pathmut using flag-based ordered operations
//! instead of subcommands. Operations are applied in the order they appear
//! on the command line.
//!
//! Example:
//!   pathmut2 --delete ext --set ext=md --print /path/file.txt

use clap::{builder::ValueParser, value_parser, Arg, ArgAction, ArgMatches, Command};
use pathmut::path::{Component, Path};
use pathmut::{PathKind, Question};
use serde::Serialize;
use std::env;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

// ============================================================================
// Operation enum - all possible ordered operations
// ============================================================================

#[derive(Debug, Clone)]
enum Operation {
    // Transformations (modify path state)
    Get(Component),
    Delete(Component),
    Set(Component, String),
    Replace(Component, String),
    Convert(PathKind),

    // Queries (affect exit code)
    Has(Component),
    Is(Question),

    // Output
    Print,
    Info,
    Depth,
}

// ============================================================================
// KeyValue parser for --set and --replace
// ============================================================================

#[derive(Debug, Clone)]
struct KeyValue {
    key: String,
    value: String,
}

impl KeyValue {
    fn parse(s: &str) -> Result<Self, String> {
        match s.split_once('=') {
            Some((k, v)) if !k.is_empty() => Ok(KeyValue {
                key: k.to_string(),
                value: v.to_string(),
            }),
            Some((_, _)) => Err(format!(
                "invalid format '{}': component name cannot be empty (expected COMPONENT=VALUE)",
                s
            )),
            None => Err(format!(
                "invalid format '{}': missing '=' (expected COMPONENT=VALUE)",
                s
            )),
        }
    }
}

// ============================================================================
// Info output structs (same as main.rs)
// ============================================================================

#[derive(Serialize)]
struct PathInfo {
    path: String,
    segments: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unix: Option<UnixInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    windows: Option<WindowsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<UrlInfoOutput>,
}

#[derive(Serialize)]
struct UnixInfo {
    root: bool,
}

#[derive(Serialize)]
struct WindowsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk: Option<String>,
    root: bool,
}

#[derive(Serialize)]
struct UrlInfoOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pass: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fragment: Option<String>,
}

impl PathInfo {
    fn from_path(path: &Path) -> Self {
        let path_str = path.serialize();
        let segments = path.segments();

        if path.is_url() {
            let url_info = path.url_info().unwrap();
            PathInfo {
                path: path_str,
                segments,
                unix: None,
                windows: None,
                url: Some(UrlInfoOutput {
                    scheme: url_info.scheme,
                    host: url_info.host,
                    port: url_info.port,
                    user: url_info.user,
                    pass: url_info.pass,
                    query: url_info.query,
                    fragment: url_info.fragment,
                }),
            }
        } else if path.is_windows() {
            let prefix = path.windows_prefix();
            let disk = {
                let d = path.get(Component::Disk);
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            };
            PathInfo {
                path: path_str,
                segments,
                unix: None,
                windows: Some(WindowsInfo {
                    prefix,
                    disk,
                    root: path.has_root(),
                }),
                url: None,
            }
        } else {
            PathInfo {
                path: path_str,
                segments,
                unix: Some(UnixInfo {
                    root: path.has_root(),
                }),
                windows: None,
                url: None,
            }
        }
    }
}

// ============================================================================
// CLI Definition
// ============================================================================

fn build_cli() -> Command {
    Command::new("pathmut2")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Mutate path strings with ordered operations")
        .arg_required_else_help(true)
        .after_help(
            "ORDERING:\n  \
             Operations are applied in the order they appear on the command line.\n\n\
             EXAMPLES:\n  \
             pathmut2 --get ext /path/file.txt\n  \
             pathmut2 --delete ext --set ext=md --print /path/file.txt\n  \
             pathmut2 --delete ext --has stem /path/file.txt\n  \
             pathmut2 --all --has ext --has stem file1.txt file2.txt\n\n\
             COMPONENTS:\n  \
             File: ext, stem, prefix, name\n  \
             Windows: disk, winprefix\n  \
             URL: scheme, host, port, path, query, frag, user, pass, origin, tld",
        )
        // Global flags for path parsing
        .arg(
            Arg::new("as-windows")
                .short('w')
                .long("as-windows")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["as-unix", "as-url"])
                .help("Parse paths as Windows paths"),
        )
        .arg(
            Arg::new("as-unix")
                .short('x')
                .long("as-unix")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["as-windows", "as-url"])
                .help("Parse paths as Unix paths"),
        )
        .arg(
            Arg::new("as-url")
                .short('u')
                .long("as-url")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["as-windows", "as-unix"])
                .help("Parse paths as URLs"),
        )
        // Query aggregation flags
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .conflicts_with("any")
                .help("All queries must pass (for --has/--is)"),
        )
        .arg(
            Arg::new("any")
                .long("any")
                .action(ArgAction::SetTrue)
                .conflicts_with("all")
                .help("Any query must pass [default]"),
        )
        // JSON output flag
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output --info as JSON instead of YAML"),
        )
        // Transformation operations (with short flags)
        .arg(
            Arg::new("get")
                .long("get")
                .short('g')
                .action(ArgAction::Append)
                .value_name("COMPONENT")
                .value_parser(|s: &str| Component::try_from(s))
                .help("Get and print a component value"),
        )
        .arg(
            Arg::new("delete")
                .long("delete")
                .short('d')
                .action(ArgAction::Append)
                .value_name("COMPONENT")
                .value_parser(|s: &str| Component::try_from(s))
                .help("Delete a component"),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .short('s')
                .action(ArgAction::Append)
                .value_name("COMPONENT=VALUE")
                .value_parser(ValueParser::new(|s: &str| KeyValue::parse(s)))
                .help("Set a component value"),
        )
        .arg(
            Arg::new("replace")
                .long("replace")
                .short('r')
                .action(ArgAction::Append)
                .value_name("COMPONENT=VALUE")
                .value_parser(ValueParser::new(|s: &str| KeyValue::parse(s)))
                .help("Replace an existing component"),
        )
        .arg(
            Arg::new("convert")
                .long("convert")
                .short('c')
                .action(ArgAction::Append)
                .value_name("TYPE")
                .value_parser(value_parser!(PathKind))
                .help("Convert path format (unix|win)"),
        )
        // Query operations (long flags only)
        .arg(
            Arg::new("has")
                .long("has")
                .action(ArgAction::Append)
                .value_name("COMPONENT")
                .value_parser(|s: &str| Component::try_from(s))
                .help("Check if component exists"),
        )
        .arg(
            Arg::new("is")
                .long("is")
                .action(ArgAction::Append)
                .value_name("QUESTION")
                .value_parser(value_parser!(Question))
                .help("Check path type (absolute|relative|unix|windows|url)"),
        )
        // Output operations (long flags only)
        // These use default_missing_value + require_equals so indices_of works properly
        // and they don't consume positional arguments.
        .arg(
            Arg::new("print")
                .long("print")
                .action(ArgAction::Append)
                .num_args(0..=1)
                .default_missing_value("_")
                .require_equals(true)
                .help("Print current path state"),
        )
        .arg(
            Arg::new("info")
                .long("info")
                .action(ArgAction::Append)
                .num_args(0..=1)
                .default_missing_value("_")
                .require_equals(true)
                .help("Print detailed path info (terminates processing)"),
        )
        .arg(
            Arg::new("depth")
                .long("depth")
                .action(ArgAction::Append)
                .num_args(0..=1)
                .default_missing_value("_")
                .require_equals(true)
                .help("Print path depth"),
        )
        // Positional: paths to transform
        .arg(
            Arg::new("path")
                .action(ArgAction::Append)
                .value_name("PATH")
                .help("Path strings to transform"),
        )
}

// ============================================================================
// Operation Extraction (using indices_of + sort)
// ============================================================================

fn extract_ordered_operations(matches: &ArgMatches) -> Vec<Operation> {
    let mut indexed_ops: Vec<(usize, Operation)> = Vec::new();

    // Helper for Component-based operations
    fn collect_component_ops<F>(
        matches: &ArgMatches,
        arg_name: &str,
        indexed_ops: &mut Vec<(usize, Operation)>,
        make_op: F,
    ) where
        F: Fn(Component) -> Operation,
    {
        if let (Some(values), Some(indices)) = (
            matches.get_many::<Component>(arg_name),
            matches.indices_of(arg_name),
        ) {
            indexed_ops.extend(indices.zip(values).map(|(i, c)| (i, make_op(*c))));
        }
    }

    // Helper for KeyValue-based operations
    fn collect_kv_ops<F>(
        matches: &ArgMatches,
        arg_name: &str,
        indexed_ops: &mut Vec<(usize, Operation)>,
        make_op: F,
    ) where
        F: Fn(Component, String) -> Operation,
    {
        if let (Some(values), Some(indices)) = (
            matches.get_many::<KeyValue>(arg_name),
            matches.indices_of(arg_name),
        ) {
            for (i, kv) in indices.zip(values) {
                if let Ok(component) = Component::try_from(kv.key.as_str()) {
                    indexed_ops.push((i, make_op(component, kv.value.clone())));
                }
            }
        }
    }

    // Helper for no-arg operations (print, info, depth)
    // These use default_missing_value="", so we need to get values to make indices_of work
    fn collect_noarg_ops(
        matches: &ArgMatches,
        arg_name: &str,
        indexed_ops: &mut Vec<(usize, Operation)>,
        op: Operation,
    ) {
        if let (Some(_values), Some(indices)) = (
            matches.get_many::<String>(arg_name),
            matches.indices_of(arg_name),
        ) {
            indexed_ops.extend(indices.map(|i| (i, op.clone())));
        }
    }

    // Collect all operation types
    collect_component_ops(matches, "get", &mut indexed_ops, Operation::Get);
    collect_component_ops(matches, "delete", &mut indexed_ops, Operation::Delete);
    collect_component_ops(matches, "has", &mut indexed_ops, Operation::Has);

    collect_kv_ops(matches, "set", &mut indexed_ops, Operation::Set);
    collect_kv_ops(matches, "replace", &mut indexed_ops, Operation::Replace);

    // Convert
    if let (Some(values), Some(indices)) = (
        matches.get_many::<PathKind>("convert"),
        matches.indices_of("convert"),
    ) {
        indexed_ops.extend(
            indices
                .zip(values)
                .map(|(i, k)| (i, Operation::Convert(*k))),
        );
    }

    // Is (Question)
    if let (Some(values), Some(indices)) =
        (matches.get_many::<Question>("is"), matches.indices_of("is"))
    {
        indexed_ops.extend(indices.zip(values).map(|(i, q)| (i, Operation::Is(*q))));
    }

    // No-arg operations
    collect_noarg_ops(matches, "print", &mut indexed_ops, Operation::Print);
    collect_noarg_ops(matches, "info", &mut indexed_ops, Operation::Info);
    collect_noarg_ops(matches, "depth", &mut indexed_ops, Operation::Depth);

    // Sort by index to get CLI order
    indexed_ops.sort_by_key(|(idx, _)| *idx);

    // Extract just the operations
    indexed_ops.into_iter().map(|(_, op)| op).collect()
}

// ============================================================================
// Path Parsing
// ============================================================================

#[derive(Debug, Clone, Copy)]
enum ParseAs {
    Derive,
    Windows,
    Unix,
    Url,
}

fn parse_path(path_str: &str, parse_as: ParseAs) -> Path {
    match parse_as {
        ParseAs::Derive => Path::parse(path_str),
        ParseAs::Windows => Path::parse_as_windows(path_str),
        ParseAs::Unix => Path::parse_as_unix(path_str),
        ParseAs::Url => Path::parse_as_url(path_str)
            .map(|(p, _)| p)
            .expect("valid url"),
    }
}

// ============================================================================
// Query Evaluation
// ============================================================================

fn evaluate_is(path: &Path, question: Question) -> bool {
    match question {
        Question::Absolute => {
            path.serialize().starts_with('/')
                || (path.is_windows() && !path.get(Component::Disk).is_empty())
        }
        Question::Relative => {
            !path.serialize().starts_with('/')
                && !(path.is_windows() && !path.get(Component::Disk).is_empty())
        }
        Question::Unix => path.is_unix(),
        Question::Windows => path.is_windows(),
        Question::Url => path.is_url(),
    }
}

// ============================================================================
// Main Execution
// ============================================================================

fn main() -> ExitCode {
    let cli = build_cli();
    let stdin = io::stdin();

    // Manually fetch args, so it can be overwritten by piped input
    let mut args: Vec<String> = env::args_os().map(|x| x.into_string().unwrap()).collect();

    if !stdin.is_terminal() {
        let mut v = Vec::new();
        let mut handle = stdin.lock();
        handle.read_to_end(&mut v).unwrap();
        let s = String::from_utf8(v).unwrap();
        if !s.is_empty() {
            args.push(s.trim().to_string());
        }
    }

    let matches = cli.get_matches_from(&args);

    // Parse global flags
    let parse_as = if *matches.get_one::<bool>("as-windows").unwrap_or(&false) {
        ParseAs::Windows
    } else if *matches.get_one::<bool>("as-unix").unwrap_or(&false) {
        ParseAs::Unix
    } else if *matches.get_one::<bool>("as-url").unwrap_or(&false) {
        ParseAs::Url
    } else {
        ParseAs::Derive
    };

    let use_all = *matches.get_one::<bool>("all").unwrap_or(&false);
    let use_json = *matches.get_one::<bool>("json").unwrap_or(&false);

    // Extract paths
    let path_strs: Vec<&String> = matches
        .get_many::<String>("path")
        .map(|v| v.collect())
        .unwrap_or_default();

    if path_strs.is_empty() {
        eprintln!("error: no paths provided");
        return ExitCode::FAILURE;
    }

    // Extract operations
    let operations = extract_ordered_operations(&matches);

    if operations.is_empty() {
        eprintln!("error: no operations provided (use --get, --set, --delete, etc.)");
        return ExitCode::FAILURE;
    }

    // Check if operations produce output or are query-only
    let has_output_op = operations.iter().any(|op| {
        matches!(
            op,
            Operation::Get(_) | Operation::Print | Operation::Info | Operation::Depth
        )
    });
    let has_query_op = operations
        .iter()
        .any(|op| matches!(op, Operation::Has(_) | Operation::Is(_)));

    // Auto-print if no output operations and no query operations
    let auto_print = !has_output_op && !has_query_op;

    // Track query results for exit code
    let mut query_results: Vec<bool> = Vec::new();

    // Process each path
    for path_str in path_strs {
        let mut path = parse_path(path_str, parse_as);
        let mut terminated = false;

        for op in &operations {
            match op {
                Operation::Get(component) => {
                    println!("{}", path.get(*component));
                }
                Operation::Delete(component) => {
                    path.delete(*component);
                }
                Operation::Set(component, value) => {
                    path.set(*component, value);
                }
                Operation::Replace(component, value) => {
                    path.replace(*component, value);
                }
                Operation::Convert(kind) => {
                    path = match kind {
                        PathKind::Unix => path.to_unix(),
                        PathKind::Windows => path.to_windows(),
                        PathKind::Url => path.to_url(),
                    };
                }
                Operation::Has(component) => {
                    let result = path.has(*component);
                    query_results.push(result);
                }
                Operation::Is(question) => {
                    let result = evaluate_is(&path, *question);
                    query_results.push(result);
                }
                Operation::Print => {
                    println!("{}", path.serialize());
                }
                Operation::Info => {
                    let info = PathInfo::from_path(&path);
                    if use_json {
                        println!("{}", serde_json::to_string_pretty(&info).unwrap());
                    } else {
                        print!("{}", serde_yaml::to_string(&info).unwrap());
                    }
                    // Info terminates processing for this path
                    terminated = true;
                    break;
                }
                Operation::Depth => {
                    println!("{}", path.depth());
                }
            }
        }

        // Auto-print if no output/query operations (and not terminated by --info)
        if auto_print && !terminated {
            println!("{}", path.serialize());
        }
    }

    // Determine exit code from query results
    if query_results.is_empty() {
        ExitCode::SUCCESS
    } else {
        let passed = if use_all {
            query_results.iter().all(|&r| r)
        } else {
            query_results.iter().any(|&r| r)
        };
        if passed {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
    use assert_cmd::assert::Assert;
    use assert_cmd::Command;
    use predicates::prelude::*;
    use rstest::rstest;

    fn pathmut2(args: &[&str]) -> Assert {
        Command::cargo_bin("pathmut2").unwrap().args(args).assert()
    }

    fn pathmut2_stdout(args: &[&str]) -> String {
        let output = Command::cargo_bin("pathmut2")
            .unwrap()
            .args(args)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    // =========================================================================
    // Basic --get tests
    // =========================================================================
    #[rstest]
    #[case("ext", "/my/path/file.txt", "txt")]
    #[case("stem", "/my/path/file.txt", "file")]
    #[case("name", "/my/path/file.txt", "file.txt")]
    #[case("ext", "/my/path/file.tar.gz", "gz")]
    #[case("stem", "/my/path/file.tar.gz", "file.tar")]
    #[case("ext", "https://example.com/file.txt", "txt")]
    #[case("scheme", "https://example.com/path", "https")]
    #[case("host", "https://example.com/path", "example.com")]
    fn get(#[case] component: &str, #[case] path: &str, #[case] expected: &str) {
        pathmut2(&["--get", component, path])
            .success()
            .stdout(format!("{expected}\n"));
    }

    // =========================================================================
    // Basic --delete + --print tests
    // =========================================================================
    #[rstest]
    #[case("ext", "/my/path/file.txt", "/my/path/file")]
    #[case("name", "/my/path/file.txt", "/my/path")]
    #[case("query", "https://example.com?foo=bar", "https://example.com")]
    #[case("port", "https://example.com:8080/path", "https://example.com/path")]
    fn delete(#[case] component: &str, #[case] path: &str, #[case] expected: &str) {
        pathmut2(&["--delete", component, "--print", path])
            .success()
            .stdout(format!("{expected}\n"));
    }

    // =========================================================================
    // Basic --set + --print tests
    // =========================================================================
    #[rstest]
    #[case("ext", "NEW", "/path/file.txt", "/path/file.NEW")]
    #[case("ext", "md", "/path/file", "/path/file.md")]
    #[case("scheme", "ftp", "https://example.com", "ftp://example.com")]
    fn set(
        #[case] component: &str,
        #[case] value: &str,
        #[case] path: &str,
        #[case] expected: &str,
    ) {
        pathmut2(&[
            "--set",
            &format!("{}={}", component, value),
            "--print",
            path,
        ])
        .success()
        .stdout(format!("{expected}\n"));
    }

    // =========================================================================
    // --replace tests
    // =========================================================================
    #[test]
    fn replace_existing() {
        pathmut2(&["--replace", "ext=NEW", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file.NEW\n");
    }

    #[test]
    fn replace_missing() {
        pathmut2(&["--replace", "ext=NEW", "--print", "/path/file"])
            .success()
            .stdout("/path/file\n");
    }

    // =========================================================================
    // Ordered operations tests
    // =========================================================================
    #[test]
    fn order_delete_then_print() {
        pathmut2(&["--delete", "ext", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file\n");
    }

    #[test]
    fn order_multiple_gets() {
        pathmut2(&[
            "--get",
            "ext",
            "--delete",
            "ext",
            "--get",
            "ext",
            "/path/file.txt",
        ])
        .success()
        .stdout("txt\n\n");
    }

    #[test]
    fn order_print_delete_print() {
        pathmut2(&["--print", "--delete", "ext", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file.txt\n/path/file\n");
    }

    #[test]
    fn order_set_then_get() {
        pathmut2(&["--set", "ext=md", "--get", "ext", "/path/file.txt"])
            .success()
            .stdout("md\n");
    }

    #[test]
    fn chain_multiple_transforms() {
        pathmut2(&[
            "--delete",
            "ext",
            "--set",
            "ext=md",
            "--print",
            "/path/file.txt",
        ])
        .success()
        .stdout("/path/file.md\n");
    }

    // =========================================================================
    // Query tests (--has, --is)
    // =========================================================================
    #[test]
    fn has_success() {
        pathmut2(&["--has", "ext", "/path/file.txt"]).success();
    }

    #[test]
    fn has_failure() {
        pathmut2(&["--has", "ext", "/path/file"]).failure();
    }

    #[test]
    fn has_any_mode() {
        // First --has passes, second fails -> success with --any (default)
        pathmut2(&[
            "--has",
            "ext",
            "--delete",
            "ext",
            "--has",
            "ext",
            "/path/file.txt",
        ])
        .success();
    }

    #[test]
    fn has_all_mode() {
        // With --all, both must pass -> failure
        pathmut2(&[
            "--all",
            "--has",
            "ext",
            "--delete",
            "ext",
            "--has",
            "ext",
            "/path/file.txt",
        ])
        .failure();
    }

    #[test]
    fn is_absolute() {
        pathmut2(&["--is", "absolute", "/path/file.txt"]).success();
    }

    #[test]
    fn is_relative() {
        pathmut2(&["--is", "absolute", "relative/path"]).failure();
    }

    #[test]
    fn is_url() {
        pathmut2(&["--is", "url", "https://example.com"]).success();
    }

    // =========================================================================
    // --convert tests
    // =========================================================================
    #[test]
    fn convert_to_unix() {
        pathmut2(&["--convert", "unix", "--print", r"\path\file.txt"])
            .success()
            .stdout("/path/file.txt\n");
    }

    #[test]
    fn convert_to_windows() {
        pathmut2(&["--convert", "win", "--print", "/path/file.txt"])
            .success()
            .stdout("\\path\\file.txt\n");
    }

    #[test]
    fn convert_then_get() {
        pathmut2(&["--convert", "unix", "--get", "ext", r"\path\file.txt"])
            .success()
            .stdout("txt\n");
    }

    #[test]
    fn convert_url_to_unix() {
        // URL to Unix should assume root and drop URL-specific info
        pathmut2(&["--convert", "unix", "https://example.com/path/to/file.txt"])
            .success()
            .stdout("/path/to/file.txt\n");
    }

    #[test]
    fn convert_url_to_windows() {
        // URL to Windows should assume root and drop URL-specific info
        pathmut2(&["--convert", "win", "https://example.com/path/to/file.txt"])
            .success()
            .stdout("\\path\\to\\file.txt\n");
    }

    #[test]
    fn convert_to_url() {
        pathmut2(&["--convert", "url", "/path/to/file.txt"])
            .success()
            .stdout("https:///path/to/file.txt\n");
    }

    // =========================================================================
    // --info and --depth tests
    // =========================================================================
    #[test]
    fn info_yaml() {
        pathmut2(&["--info", "/path/file.txt"])
            .success()
            .stdout(predicate::str::contains("segments:"));
    }

    #[test]
    fn info_json() {
        pathmut2(&["--json", "--info", "/path/file.txt"])
            .success()
            .stdout(predicate::str::contains("{"));
    }

    #[test]
    fn info_terminates() {
        // --info should terminate, subsequent ops should not run
        let output = pathmut2_stdout(&[
            "--delete",
            "ext",
            "--info",
            "--set",
            "ext=SHOULD_NOT_APPEAR",
            "/path/file.txt",
        ]);
        assert!(!output.contains("SHOULD_NOT_APPEAR"));
        assert!(output.contains("segments"));
    }

    #[test]
    fn depth() {
        pathmut2(&["--depth", "/path/to/file.txt"])
            .success()
            .stdout("3\n");
    }

    #[test]
    fn depth_after_delete() {
        pathmut2(&["--delete", "name", "--depth", "/path/to/file.txt"])
            .success()
            .stdout("2\n");
    }

    // =========================================================================
    // Multiple paths tests
    // =========================================================================
    #[test]
    fn multiple_paths_get() {
        pathmut2(&["--get", "ext", "file.txt", "other.md"])
            .success()
            .stdout("txt\nmd\n");
    }

    #[test]
    fn multiple_paths_transform() {
        pathmut2(&["--delete", "ext", "--print", "file.txt", "other.md"])
            .success()
            .stdout("file\nother\n");
    }

    #[test]
    fn multiple_paths_has_any() {
        pathmut2(&["--has", "ext", "file.txt", "noext"]).success();
    }

    #[test]
    fn multiple_paths_has_all() {
        pathmut2(&["--all", "--has", "ext", "file.txt", "noext"]).failure();
    }

    // =========================================================================
    // Validation tests
    // =========================================================================
    #[test]
    fn set_requires_equals() {
        pathmut2(&["--set", "ext", "--print", "file.txt"])
            .failure()
            .stderr(predicate::str::contains("="));
    }

    #[test]
    fn set_rejects_empty_component() {
        pathmut2(&["--set", "=value", "--print", "file.txt"]).failure();
    }

    #[test]
    fn no_args_shows_help() {
        pathmut2(&[])
            .failure()
            .stderr(predicate::str::contains("Usage:"));
    }

    #[test]
    fn no_paths_error() {
        pathmut2(&["--get", "ext"]).failure();
    }

    #[test]
    fn help() {
        pathmut2(&["--help"])
            .success()
            .stdout(predicate::str::contains("--get"));
    }

    // =========================================================================
    // Auto-print tests
    // =========================================================================
    #[test]
    fn auto_print_after_delete() {
        // No explicit --print, but should auto-print since it's a transform
        pathmut2(&["--delete", "ext", "/path/file.txt"])
            .success()
            .stdout("/path/file\n");
    }

    #[test]
    fn auto_print_after_set() {
        pathmut2(&["--set", "ext=md", "/path/file.txt"])
            .success()
            .stdout("/path/file.md\n");
    }

    #[test]
    fn auto_print_after_convert() {
        pathmut2(&["--convert", "win", "/path/file.txt"])
            .success()
            .stdout("\\path\\file.txt\n");
    }

    #[test]
    fn no_auto_print_with_get() {
        // --get produces output, so no auto-print
        pathmut2(&["--delete", "ext", "--get", "stem", "/path/file.txt"])
            .success()
            .stdout("file\n");
    }

    #[test]
    fn no_auto_print_with_query() {
        // --has is a query, so no auto-print (exit code is the result)
        pathmut2(&["--delete", "ext", "--has", "stem", "/path/file.txt"])
            .success()
            .stdout("");
    }

    #[test]
    fn auto_print_multiple_paths() {
        pathmut2(&["--delete", "ext", "file.txt", "other.md"])
            .success()
            .stdout("file\nother\n");
    }

    // =========================================================================
    // Stdin test
    // =========================================================================
    #[test]
    fn stdin() {
        Command::cargo_bin("pathmut2")
            .unwrap()
            .args(["--get", "ext"])
            .write_stdin("/my/path/file.txt")
            .assert()
            .success()
            .stdout("txt\n");
    }

    // =========================================================================
    // Short flag tests
    // =========================================================================
    #[test]
    fn short_flags() {
        pathmut2(&["-g", "ext", "/path/file.txt"])
            .success()
            .stdout("txt\n");

        pathmut2(&["-d", "ext", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file\n");

        pathmut2(&["-s", "ext=md", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file.md\n");

        pathmut2(&["-r", "ext=NEW", "--print", "/path/file.txt"])
            .success()
            .stdout("/path/file.NEW\n");

        pathmut2(&["-c", "win", "--print", "/path/file.txt"])
            .success()
            .stdout("\\path\\file.txt\n");
    }
}

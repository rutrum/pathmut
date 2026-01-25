//! Prototype: Using clap with index reconstruction for ordered operations
//!
//! This demonstrates how to handle:
//!   pathmut --delete ext --replace scheme=http --delete host "url://..."
//!
//! Where transformations are applied in the order they appear on the CLI.
//!
//! Run with: cargo run --example clap_ordered -- --help
//! Or:       cargo run --example clap_ordered -- --delete ext --replace scheme=http "https://example.com/file.txt"

use clap::{builder::ValueParser, Arg, ArgAction, ArgMatches, Command};

/// Represents a transformation operation
#[derive(Debug, Clone)]
enum Operation {
    Get(String),             // component
    Delete(String),          // component
    Set(String, String),     // component, value
    Replace(String, String), // component, value
}

/// A validated key=value pair parsed at CLI time
#[derive(Debug, Clone)]
struct KeyValue {
    key: String,
    value: String,
}

impl KeyValue {
    /// Parse a "key=value" string, requiring the equals sign
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

/// An operation paired with its CLI position index
#[derive(Debug)]
struct IndexedOp {
    index: usize,
    op: Operation,
}

fn build_cli() -> Command {
    Command::new("pathmut")
        .version("0.9.0")
        .about("Mutate path strings with ordered operations")
        .after_help(
            "ORDERING:\n  \
             Transformations are applied in the order they appear on the command line.\n\n\
             EXAMPLES:\n  \
             pathmut --delete ext --replace scheme=http \"https://example.com/file.txt\"\n  \
             pathmut --get ext --get stem \"path/to/file.tar.gz\"\n  \
             pathmut --set ext=md --delete scheme \"https://example.com/file.txt\"",
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
        // Transformation operations - each can appear multiple times
        .arg(
            Arg::new("get")
                .long("get")
                .short('g')
                .action(ArgAction::Append)
                .value_name("COMPONENT")
                .help("Get a component value (can be repeated)"),
        )
        .arg(
            Arg::new("delete")
                .long("delete")
                .short('d')
                .action(ArgAction::Append)
                .value_name("COMPONENT")
                .help("Delete a component (can be repeated)"),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .short('s')
                .action(ArgAction::Append)
                .value_name("COMPONENT=VALUE")
                .value_parser(ValueParser::new(|s: &str| KeyValue::parse(s)))
                .help("Set a component value (can be repeated)"),
        )
        .arg(
            Arg::new("replace")
                .long("replace")
                .short('r')
                .action(ArgAction::Append)
                .value_name("COMPONENT=VALUE")
                .value_parser(ValueParser::new(|s: &str| KeyValue::parse(s)))
                .help("Replace an existing component (can be repeated)"),
        )
        // Positional: paths to transform
        .arg(
            Arg::new("path")
                .action(ArgAction::Append)
                .value_name("PATH")
                .help("Path strings to transform"),
        )
}

/// Extract operations from ArgMatches, preserving their CLI order via indices.
///
/// Performance: O(n log n) where n = total number of operations.
/// - indices_of() is O(1) - clap stores indices alongside values internally
/// - get_many() is O(1) - returns an iterator over stored values
/// - The sort at the end is O(n log n)
fn extract_ordered_operations(matches: &ArgMatches) -> Vec<Operation> {
    let mut indexed_ops: Vec<IndexedOp> = Vec::new();

    // Helper to extract string-based operations (get, delete)
    fn collect_ops<F>(
        matches: &ArgMatches,
        arg_name: &str,
        indexed_ops: &mut Vec<IndexedOp>,
        make_op: F,
    ) where
        F: Fn(String) -> Operation,
    {
        if let Some(values) = matches.get_many::<String>(arg_name) {
            if let Some(indices) = matches.indices_of(arg_name) {
                for (value, index) in values.zip(indices) {
                    indexed_ops.push(IndexedOp {
                        index,
                        op: make_op(value.clone()),
                    });
                }
            }
        }
    }

    // Helper to extract KeyValue-based operations (set, replace)
    fn collect_kv_ops<F>(
        matches: &ArgMatches,
        arg_name: &str,
        indexed_ops: &mut Vec<IndexedOp>,
        make_op: F,
    ) where
        F: Fn(&KeyValue) -> Operation,
    {
        if let Some(values) = matches.get_many::<KeyValue>(arg_name) {
            if let Some(indices) = matches.indices_of(arg_name) {
                for (kv, index) in values.zip(indices) {
                    indexed_ops.push(IndexedOp {
                        index,
                        op: make_op(kv),
                    });
                }
            }
        }
    }

    // Collect all operation types
    collect_ops(matches, "get", &mut indexed_ops, Operation::Get);
    collect_ops(matches, "delete", &mut indexed_ops, Operation::Delete);

    // For set and replace, extract from pre-validated KeyValue
    collect_kv_ops(matches, "set", &mut indexed_ops, |kv| {
        Operation::Set(kv.key.clone(), kv.value.clone())
    });
    collect_kv_ops(matches, "replace", &mut indexed_ops, |kv| {
        Operation::Replace(kv.key.clone(), kv.value.clone())
    });

    // Sort by index to get CLI order
    indexed_ops.sort_by(|a, b| a.index.cmp(&b.index));

    // Extract just the operations
    indexed_ops.into_iter().map(|io| io.op).collect()
}

/// Demo: apply operations to a path string (placeholder implementation)
fn apply_operations(path: &str, operations: &[Operation]) -> String {
    // This is a simplified demo - in real code you'd use pathmut::Path
    let mut result = path.to_string();

    for op in operations {
        match op {
            Operation::Get(component) => {
                // In real impl: print path.get(component)
                println!("[GET {}]: (would extract from '{}')", component, result);
            }
            Operation::Delete(component) => {
                // In real impl: path.delete(component)
                println!("[DELETE {}]: transforming '{}'", component, result);
                result = format!("{}[deleted:{}]", result, component);
            }
            Operation::Set(component, value) => {
                println!("[SET {}={}]: transforming '{}'", component, value, result);
                result = format!("{}[set:{}={}]", result, component, value);
            }
            Operation::Replace(component, value) => {
                println!(
                    "[REPLACE {}={}]: transforming '{}'",
                    component, value, result
                );
                result = format!("{}[replace:{}={}]", result, component, value);
            }
        }
    }

    result
}

fn main() {
    let matches = build_cli().get_matches();

    // Extract paths
    let paths: Vec<&String> = matches
        .get_many::<String>("path")
        .map(|v| v.collect())
        .unwrap_or_default();

    if paths.is_empty() {
        eprintln!("No paths provided. Use --help for usage.");
        std::process::exit(1);
    }

    // Extract operations in CLI order
    let operations = extract_ordered_operations(&matches);

    if operations.is_empty() {
        eprintln!("No operations provided. Use --help for usage.");
        std::process::exit(1);
    }

    println!("=== Operations in CLI order ===");
    for (i, op) in operations.iter().enumerate() {
        println!("  {}: {:?}", i + 1, op);
    }
    println!();

    println!("=== Applying to paths ===");
    for path in paths {
        println!("Input: {}", path);
        let result = apply_operations(path, &operations);
        println!("Output: {}\n", result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_preservation() {
        // Simulate: --delete ext --replace scheme=http --delete host
        let matches = build_cli().get_matches_from([
            "pathmut",
            "--delete",
            "ext",
            "--replace",
            "scheme=http",
            "--delete",
            "host",
            "https://example.com/file.txt",
        ]);

        let ops = extract_ordered_operations(&matches);

        assert_eq!(ops.len(), 3);

        // Verify order is preserved
        assert!(matches!(ops[0], Operation::Delete(ref c) if c == "ext"));
        assert!(matches!(ops[1], Operation::Replace(ref c, ref v) if c == "scheme" && v == "http"));
        assert!(matches!(ops[2], Operation::Delete(ref c) if c == "host"));
    }

    #[test]
    fn test_interleaved_same_flag() {
        // Simulate: --delete ext --delete scheme --delete host
        let matches = build_cli().get_matches_from([
            "pathmut", "--delete", "ext", "--delete", "scheme", "--delete", "host", "path",
        ]);

        let ops = extract_ordered_operations(&matches);

        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], Operation::Delete(ref c) if c == "ext"));
        assert!(matches!(ops[1], Operation::Delete(ref c) if c == "scheme"));
        assert!(matches!(ops[2], Operation::Delete(ref c) if c == "host"));
    }

    #[test]
    fn test_complex_interleaving() {
        // Simulate: --get ext --delete ext --set ext=md --get ext
        let matches = build_cli().get_matches_from([
            "pathmut", "--get", "ext", "--delete", "ext", "--set", "ext=md", "--get", "ext",
            "file.txt",
        ]);

        let ops = extract_ordered_operations(&matches);

        assert_eq!(ops.len(), 4);
        assert!(matches!(ops[0], Operation::Get(ref c) if c == "ext"));
        assert!(matches!(ops[1], Operation::Delete(ref c) if c == "ext"));
        assert!(matches!(ops[2], Operation::Set(ref c, ref v) if c == "ext" && v == "md"));
        assert!(matches!(ops[3], Operation::Get(ref c) if c == "ext"));
    }

    #[test]
    fn test_short_flags() {
        // Simulate: -d ext -r scheme=http -s name=new
        let matches = build_cli().get_matches_from([
            "pathmut",
            "-d",
            "ext",
            "-r",
            "scheme=http",
            "-s",
            "name=new",
            "path",
        ]);

        let ops = extract_ordered_operations(&matches);

        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], Operation::Delete(ref c) if c == "ext"));
        assert!(matches!(ops[1], Operation::Replace(ref c, ref v) if c == "scheme" && v == "http"));
        assert!(matches!(ops[2], Operation::Set(ref c, ref v) if c == "name" && v == "new"));
    }

    #[test]
    fn test_validation_rejects_missing_equals() {
        let result = build_cli().try_get_matches_from(["pathmut", "--set", "ext", "path"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rejects_empty_component() {
        let result = build_cli().try_get_matches_from(["pathmut", "--set", "=value", "path"]);
        assert!(result.is_err());
    }
}

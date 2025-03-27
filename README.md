# `pathmut` is a path string manipulation utility

This was made because I'm tired of resorting to `basename`, `dirname`, `cut -d. -f1` or whatever other random core util to just extract some information from paths.

Important to know this utility only works with path _strings_ and doesn't touch the file system whatsoever.

```
$ pathmut
Mutate path strings

Usage: pathmut [OPTIONS] [COMMAND or COMPONENT]

Commands:
  get        Read a path component [default]
  delete     Remove a path component
  replace    Replace an existing path component
  set        Set a path component
  has        Check if a path component exists
  is         Ask questions about a file path
  normalize  Normalize a file path
  convert    Convert between unix and windows paths
  depth      Number of components before the last component.
  help       Print this message or the help of the given subcommand(s)

Options:
  -n, --normalize   Normalize the path first
  -u, --as-unix     Parse paths as unix paths
  -w, --as-windows  Parse paths as windows paths
  -h, --help        Print help information
  -V, --version     Print version information

Components:
  ext      File extension
  stem     File stem
  prefix   File prefix
  name     File name
  parent   Parent of the file or directory
  disk     Disk of a windows path
  n        Ordinal of the nth component
```

## Installation

For now, you have to build from source.  The package is available on [crates.io](https://crates.io/crates/pathmut) and can be installed with `cargo`.
```
cargo install pathmut
```
You can also build using `nix` if you have flakes enabled.
```
nix shell github:rutrum/pathmut
```

## Future Development

I'd like this to be the all in one tool for manipulating path strings.  This tool should never touch or look at the actual filesystem, which means it shouldn't be able to answer if a path exists, if something is a file, create files, etc.  

I should be able to ask questions about path strings, like the following:

* Is the path valid, if it did exist?
* What is the depth of the path?

I should be able to work with multiple paths:

* Does path A begin with path B?
* Does path A end with path B?
* Is path B contained with path A?
* Join path A and path B together.

Why stop at paths?  A command line utility built to mutate URIs like "scheme://user:pass@sub.domain.com/route?query=param#anchor" would have an almost identical API, and would even overlap with some path functionality.  In the future, this library will expand to include URIs as well.

## Changelog

### 0.7.0

* Normalize flag `-n` that can apply normalization to the paths before any command
* Flags to force parsing paths as windows `-w` or as unix `-u` paths
* Add `is normal` command which checks if a path string is normalized
* Add `depth` command which counts the number of components before the last
* Add `disk` component for Windows paths with disk prefix

### 0.6.0

* Specify nth component using negative indicies to count backwards, `-1` for last component, etc.
* Add `normalize` command that removes `..` and `.` from path appropriately
* Add `convert` command to convert between Windows and unix paths

### 0.5.0

* Rename `replace` to `set`
* Add `replace` command, which does not change paths when the component does not exist
* Windows and unix path support, independent of native OS
* Add `is` command which tests if paths are relative, absolute, windows, or unix
    * `--print` flag to print true/false instead of changing exit code
    * `--any` and `--all` flags for testing multiple paths
* Add `has` command for checking if a path component exists

### 0.4.0

* Remove `--remove` and `--replace` args with `delete` and `replace` commands
* Add `get` command (default behavior)
* Components are now arguments to `get`, `delete`, and `replace`
* Specify nth component by using number, `0` for first component, etc.
* Remove `first` component

### 0.3.0

* Add `first` command
* Add `--remove` and `--replace` arguments for every command
* Remove `rmext` command

### 0.2.0

* Show help by default
* Read from stdin, all at once
* Allow passing multiple file paths as args (not stdin)
* Add `rmext` command

### 0.1.0

* Add `ext`, `stem`, `prefix`, `name`, `parent` commands.

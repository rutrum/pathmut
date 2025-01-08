# `pathmut` is a path string manipulation utility

This was made because I'm tired of resorting to `basename`, `dirname`, `cut -d. -f1` or whatever other random core util to just extract some information from path names.

Important to know this utility only works with path _strings_ and doesn't touch the file system whatsoever.

```
$ pathmut
Mutates path strings.

Usage: pathmut [COMMAND or COMPONENT]

Commands:
  get      Read a file component [default]
  delete   Remove a file component
  replace  Replace a file component
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information

Components:
  ext      file extension
  stem     file stem
  prefix   file prefix
  name     file name
  parent   parent of the file or directory
  n        ordinal of the nth component
```

## Installation

For now, you have to build from source.  The package is available on [crates.io](https://crates.io/crates/pathmut).
```
cargo install pathmut
```

## Documentation

This utility is really just a command line wrapper for [Rust's Path struct API](https://doc.rust-lang.org/stable/std/path/struct.Path.html).  Precise details on exactly how these commands work are found there.

## Future Development

I'd like this to be the all in one tool for manipulating path strings.  This tool should never touch or look at the actual filesystem, which means it shouldn't be able to answer if a path exists, if something is a file, create files, etc.  

I should be able to ask questions about path strings, like the following:

* Is the path absolute or relative?
* Is this a windows path or a unix path?
* Is the path valid, if it did exist?
* What is the depth of the path?

I should be able to manipulate paths as a whole:

* I should canonicalize a path (remove redundant `../` and `./`)

I should be able to work with multiple paths:

* Does path A begin with path B?
* Does path A end with path B?
* Is path B contained with path A?
* Join path A and path B together.

## Changelog

### v0.5.0

* Rename `replace` to `set`
* Add `replace` command, which does not change paths when the component does not exist
* Add `is` command which tests if paths are relative or absolute
    * `--print` flag to print true/false instead of changing exit code
    * `--any` and `--all` flags for testing multiple paths

### v0.4.0

* Remove `--remove` and `--replace` args with `delete` and `replace` commands
* Add `get` command (default behavior)
* Components are now arguments to `get`, `delete`, and `replace`
* Specify nth component by using number, `0` for first component, etc.
* Remove `first` component

### v0.3.0

* Add `first` command
* Add `--remove` and `--replace` arguments for every command
* Remove `rmext` command

### v0.2.0

* Show help by default
* Read from stdin, all at once
* Allow passing multiple file paths as args (not stdin)
* Add `rmext` command

### v0.1.0

* Add `ext`, `stem`, `prefix`, `name`, `parent` commands.

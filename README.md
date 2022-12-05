# `pathmut` is a path string manipulation utility

This was made because I'm tired of resorting to `basename`, `dirname`, `cut -d. -f1` or whatever other random core util to just extract some information from path names.

Important to know this utility only works with path _strings_ and doesn't touch the file system whatsoever.

```
$ pathmut
Mutates path strings.

Usage: pathmut [COMPONENT|COMMAND]

Components/Commands:
  ext     Read or update file extension
  stem    Read or update file stem
  prefix  Read or update file prefix
  name    Read or update file name
  parent  Read or update parent directory
  first   Read or update first component
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

## Documentation

This utility is really just a command line wrapper for [Rust's Path struct API](https://doc.rust-lang.org/stable/std/path/struct.Path.html).  Precise details on exactly how these commands work are found there.

## Changelog

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

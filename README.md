# `pathmut` is a path string manipulation utility

This was made because I'm tired of resorting to `basename`, `dirname`, `cut -d. -f1` or whatever other random core util to just extract some information from path names.

Important to know this utility only works with path _strings_ and doesn't touch the file system whatsoever.

```
$ pathmut --help
Mutates path strings.

Usage: pathmut [COMMAND]

Commands:
  ext     Prints file extension of file in the path.
  stem    Prints file stem of file in the path.
  prefix  Prints file prefix of file in the path.
  name    Prints the name of the file or directory at the path.
  parent  Prints the path without the final file or directory.
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

## Documentation

This utility is really just a command line wrapper for [Rust's Path struct API](https://doc.rust-lang.org/stable/std/path/struct.Path.html).  Precise details on exactly how these commands work are found there.

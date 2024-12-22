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

## Future Development

I'd like this to be the all in one tool for manipulating path strings.  This tool should never touch or look at the actual filesystem, which means it shouldn't be able to answer if a path exists, if something is a file, create files, etc.  

I should be able to ask questions about path strings, like the following:

* Is the path absolute or relative?
* Is this a windows path or a unix path?
* Is the path valid, if it did exist?
* What is the depth of the path?

I should be able to extract parts of a path:

* Extract the second part of the path.

I should be able to manipulate paths as a whole:

* I should canonicalize a path (remove redundant `../` and `./`)

I should be able to work with multiple paths:

* Does path A begin with path B?
* Does path A end with path B?
* Is path B contained with path A?
* Join path A and path B together.

All the above are questions I'd like answered within this utility, but I'm still not sure if the current API 

```
pathmut <component> [options] <path>
```

is the best for extending functionality past the extraction, removal, and replacement features.  For example, if I wanted to have the utility return a successful exit code if the path is absolute, then I would I want to write `pathmut is absolute <path>` or something else?  Should `is` be a subcommand all on its own?  Many more questions can be raised about implementing these features.

The best path forward might be writing a variety of expressions and see which ones feel the most natural to write and easiest to read and organize.

## Changelog

### v0.4.0

* Remove `--remove` and `--replace` args with `remove` and `replace` commands
* Add `get` command (default behavior)
* Components are now arguments to `get`, `remove`, and `replace`
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

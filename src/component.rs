use std::iter;
use typed_path::{PathType, TypedPath, TypedPathBuf};

// use clap::{builder::PossibleValue, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    Extension,
    Stem,
    Prefix,
    Name,
    Parent,
    // Root
    // The windows prefix
    Nth(usize),
}

// may not need this because of ValueEnum
impl TryFrom<&str> for Component {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use Component::*;
        let comp = match s {
            "ext" => Extension,
            "stem" => Stem,
            "prefix" => Prefix,
            "name" => Name,
            "parent" => Parent,
            _ => Nth(s.parse::<usize>().map_err(|_| ())?),
        };
        Ok(comp)
    }
}

pub fn arg_into_component(s: &str) -> Result<Component, String> {
    use Component::*;
    if let Ok(n) = s.parse::<usize>() {
        Ok(Nth(n))
    } else {
        let component = match s {
            "ext" => Extension,
            "stem" => Stem,
            "prefix" => Prefix,
            "name" => Name,
            "parent" => Parent,
            _ => Err("invalid component")?,
        };
        Ok(component)
    }
}

// todo: make my own typed value parser
//struct ComponentParser;
//impl clap::builder::TypedValueParser for ComponentParser {
//    type Value = Component;
//
//    fn parse_ref(
//        &self,
//        cmd: &clap::Command,
//        arg: Option<&clap::Arg>,
//        value: &std::ffi::OsStr,
//    ) -> Result<Self::Value, clap::Error> {
//    }
//}

// TODO: rewrite the following to use [u8] instead of OsStr

trait FilePrefix {
    fn file_prefix(&self) -> Option<&[u8]>;
}

impl FilePrefix for TypedPath<'_> {
    // Referencing std::path::Path::file_prefix
    // https://doc.rust-lang.org/stable/src/std/path.rs.html#2648-2650
    fn file_prefix(&self) -> Option<&[u8]> {
        self.file_name()
            .map(split_file_at_dot)
            .and_then(|(before, _after)| Some(before))
    }
}

fn split_file_at_dot(file: &[u8]) -> (&[u8], Option<&[u8]>) {
    // Referencing std::path::split_file_at_dot
    // https://doc.rust-lang.org/stable/src/std/path.rs.html#340
    let slice = file;
    if slice == b".." {
        return (file, None);
    }

    let i = match slice[1..].iter().position(|b| *b == b'.') {
        Some(i) => i + 1,
        None => return (file, None),
    };
    let before = &slice[..i];
    let after = &slice[i + 1..];
    (before, Some(after))
}

impl Component {
    pub fn action(self, action: Action, path: &TypedPath) -> Vec<u8> {
        match action {
            Action::Get => self.get(path),
            Action::Set(s) => self.set(path, s),
            Action::Replace(s) => self.replace(path, s),
            Action::Delete => self.delete(path),
        }
    }

    pub fn get(self, path: &TypedPath) -> Vec<u8> {
        use Component::*;
        match self {
            Extension => path.extension().unwrap_or_default().into(),
            Stem => path.file_stem().unwrap_or_default().into(),
            Prefix => path.file_prefix().unwrap_or_default().into(),
            Name => path.file_name().unwrap_or_default().into(),
            Parent => path
                .parent()
                .map(|p| p.as_bytes().to_vec())
                .unwrap_or_default(),
            Nth(n) => path
                .components()
                .nth(n)
                .map(|c| c.as_bytes().to_vec())
                .unwrap_or_default(),
        }
    }

    pub fn has(self, path: &TypedPath) -> bool {
        self.get(path).len() > 0
    }

    pub fn set(self, path: &TypedPath, value: &[u8]) -> Vec<u8> {
        use Component::*;
        match self {
            Extension => path.with_extension(value).into_vec(),
            Stem => {
                if let Some(ext) = path.extension() {
                    let name = [value, b".", ext].concat();
                    path.with_file_name(name).into_vec()
                } else {
                    path.with_file_name(value).into_vec()
                }
            }
            Prefix => {
                let after: &[u8] = path
                    .file_name()
                    .map(split_file_at_dot)
                    .and_then(|(_, after)| after)
                    .unwrap_or_default();

                if let Some(parent) = path.parent() {
                    let name = if after.len() > 0 {
                        [value, b".", after].concat()
                    } else {
                        value.to_vec()
                    };
                    parent.join(name).into_vec()
                } else {
                    let new_path = if path.is_unix() {
                        TypedPath::new(value, PathType::Unix)
                    } else {
                        TypedPath::new(value, PathType::Windows)
                    };
                    new_path.join(after).into_vec()
                }
            }
            Name => path.with_file_name(value).into_vec(),
            Parent => {
                let new_parent = match path {
                    TypedPath::Unix(_) => TypedPath::new(value, PathType::Unix),
                    TypedPath::Windows(_) => TypedPath::new(value, PathType::Windows),
                };
                new_parent
                    .join(path.file_name().unwrap_or_default())
                    .into_vec()
            }
            Nth(n) => {
                // what if path is root?
                // todo

                // what if n == number of components?
                let num_components = path.components().count();
                if num_components == n {
                    return path.join(value).into_vec();
                }

                // what if n > number of components?
                // todo

                path.components()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == n {
                            TypedPathBuf::from(value)
                        } else {
                            TypedPathBuf::from(c.as_bytes())
                        }
                    })
                    .reduce(|a, b| a.join(b))
                    .map(|p| p.into_vec())
                    .unwrap_or_default()
            }
        }
    }

    pub fn replace(self, path: &TypedPath, value: &[u8]) -> Vec<u8> {
        if self.has(path) {
            self.set(path, value)
        } else {
            path.to_path_buf().into_vec()
        }
    }

    pub fn delete(&self, path: &TypedPath) -> Vec<u8> {
        use Component::*;
        match self {
            Stem => {
                if let Some(ext) = path.extension() {
                    path.with_file_name(ext).into_vec()
                } else {
                    path.with_file_name("").into_vec()
                }
            }
            Prefix => {
                // revisit, this feels like hard coded, not edge case
                if path == &TypedPath::derive("/") {
                    return path.to_path_buf().into_vec();
                }

                let after: &[u8] = path
                    .file_name()
                    .map(split_file_at_dot)
                    .and_then(|(_, after)| after)
                    .unwrap_or_default();

                if let Some(parent) = path.parent() {
                    parent.join(after).into_vec()
                } else {
                    let new_path = if path.is_unix() {
                        TypedPath::new(after, PathType::Unix)
                    } else {
                        TypedPath::new(after, PathType::Windows)
                    };
                    new_path.to_path_buf().into_vec()
                }
            }
            Name => path.with_file_name("").into_vec(),
            _ => self.replace(path, b""),
        }
    }
}

pub enum Action<'a> {
    Get,
    Set(&'a [u8]),
    Delete,
    Replace(&'a [u8]),
}

pub mod set {
    use std::ffi::{OsStr, OsString};
    use std::iter;
    use std::path::{Path, PathBuf};

    pub fn ext(path: PathBuf, s: &str) -> OsString {
        path.with_extension(OsStr::new(s)).into()
    }

    pub fn stem(path: PathBuf, s: &str) -> OsString {
        if let Some(ext) = path.extension() {
            let name = format!("{}.{}", s, ext.to_str().unwrap());
            path.with_file_name(name).into()
        } else {
            path.with_file_name(s).into()
        }
    }

    pub fn prefix(path: PathBuf, s: &str) -> OsString {
        if let Some(name) = path.file_name() {
            if let Some(prefix) = path.file_prefix() {
                iter::once(s);
                let after_prefix = iter::once(s)
                    .chain(
                        name.to_str()
                            .unwrap()
                            .split('.')
                            .skip_while(|&s| s == prefix.to_str().unwrap()),
                    )
                    .intersperse(".")
                    .collect::<String>();
                path.with_file_name(after_prefix).into()
            } else {
                path.into() // unreachable?
            }
        } else {
            path.into()
        }
    }

    pub fn name(path: PathBuf, s: &str) -> OsString {
        path.with_file_name(s).into()
    }

    pub fn parent(path: PathBuf, s: &str) -> OsString {
        PathBuf::from(s)
            .join(path.file_name().unwrap_or_default())
            .into()
    }

    pub fn nth(n: usize, path: PathBuf, s: &str) -> OsString {
        // what if path is root?
        // todo

        // what if n == number of components?
        let num_components = path.components().count();
        if num_components == n {
            return path.join(s).into();
        }

        // what if n > number of components?
        // todo

        path.components()
            .enumerate()
            .map(|(i, c)| {
                if i == n {
                    PathBuf::from(s)
                } else {
                    c.as_os_str().into()
                }
            })
            .collect::<PathBuf>()
            .into()
    }
}

/// Replace means its only set if it already existed
pub mod replace {
    use super::set;
    use std::ffi::{OsStr, OsString};
    use std::iter;
    use std::path::{Path, PathBuf};

    pub fn ext(path: PathBuf, s: &str) -> OsString {
        match path.extension() {
            Some(_) => set::ext(path, s),
            None => path.into(),
        }
    }

    pub fn stem(path: PathBuf, s: &str) -> OsString {
        match path.file_prefix() {
            Some(_) => set::stem(path, s),
            None => path.into(),
        }
    }

    pub fn prefix(path: PathBuf, s: &str) -> OsString {
        match path.file_prefix() {
            Some(_) => set::prefix(path, s),
            None => path.into(),
        }
    }
    pub fn name(path: PathBuf, s: &str) -> OsString {
        match path.file_name() {
            Some(_) => set::name(path, s),
            None => path.into(),
        }
    }
    pub fn parent(path: PathBuf, s: &str) -> OsString {
        match path.parent() {
            Some(_) => set::parent(path, s),
            None => path.into(),
        }
    }
    pub fn nth(n: usize, path: PathBuf, s: &str) -> OsString {
        let back = path
            .components()
            .take(n)
            .map(|c| c.as_os_str())
            .collect::<PathBuf>();
        let front = path
            .components()
            .skip(n + 1)
            .map(|c| c.as_os_str())
            .collect::<PathBuf>();
        back.join(Path::new(s)).join(front).into()
    }

    pub mod delete {
        use super::*;

        pub fn ext(path: PathBuf) -> OsString {
            path.with_extension(OsStr::new("")).into()
        }

        pub fn stem(path: PathBuf) -> OsString {
            if let Some(ext) = path.extension() {
                path.with_file_name(ext).into()
            } else {
                path.with_file_name(OsStr::new("")).into()
            }
        }

        pub fn prefix(path: PathBuf) -> OsString {
            if let Some(name) = path.file_name() {
                if let Some(prefix) = path.file_prefix() {
                    let after_prefix = name
                        .to_str()
                        .unwrap()
                        .split('.')
                        .skip_while(|&s| s == prefix.to_str().unwrap())
                        .intersperse(".")
                        .collect::<String>();
                    path.with_file_name(after_prefix).into()
                } else {
                    path.into() // unreachable?
                }
            } else {
                path.into()
            }
        }

        pub fn name(path: PathBuf) -> OsString {
            path.with_file_name(OsStr::new("")).into()
        }

        pub fn parent(path: PathBuf) -> OsString {
            path.file_name().unwrap_or_default().into()
        }

        pub fn nth(n: usize, path: PathBuf) -> OsString {
            path.components()
                .enumerate()
                .filter_map(|(i, c)| {
                    if i == n {
                        None
                    } else {
                        Some(c.as_os_str().into())
                    }
                })
                .collect::<Vec<OsString>>()
                .join(OsStr::new("/"))
        }
    }
}

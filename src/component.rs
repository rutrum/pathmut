use std::ffi::{OsStr, OsString};
use std::iter;
use std::path::PathBuf;

use clap::{builder::PossibleValue, ValueEnum};

// Warning: this may get more complicated, allowing "part[i]"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    Extension,
    Stem,
    Prefix,
    Name,
    Parent,
    First,
    // Root
    // the windows root

    // maybe this won't work
    Nth(usize),
}

// TODO: remove
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
            "first" => First,
            _ => Err(())?,
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
            "first" => First,
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

// to make clap automatically parse into a component
impl ValueEnum for Component {
    fn value_variants<'a>() -> &'a [Self] {
        use Component::*;
        &[Extension, Stem, Prefix, Name, Parent, First, Nth(5)]
    }
    fn to_possible_value(&self) -> Option<PossibleValue> {
        use Component::*;
        let s = match self {
            Extension => "ext",
            Stem => "stem",
            Prefix => "prefix",
            Name => "name",
            Parent => "parent",
            First => "first",
            Nth(_) => "nth",
        };
        Some(PossibleValue::new(s))
    }
}

pub enum Action<'a> {
    Get,
    Remove,
    Replace(&'a str),
}

pub mod get {
    use super::*;

    pub fn ext(path: PathBuf) -> OsString {
        path.extension().unwrap_or_default().into()
    }

    pub fn stem(path: PathBuf) -> OsString {
        path.file_stem().unwrap_or_default().into()
    }

    pub fn prefix(path: PathBuf) -> OsString {
        path.file_prefix().unwrap_or_default().into()
    }

    pub fn name(path: PathBuf) -> OsString {
        path.file_name().unwrap_or_default().into()
    }

    pub fn parent(path: PathBuf) -> OsString {
        match path.parent() {
            Some(path) => path.into(),
            None => OsString::new(),
        }
    }

    pub fn first(path: PathBuf) -> OsString {
        match path
            .ancestors()
            .filter(|&x| !x.as_os_str().is_empty())
            .last()
        {
            Some(path) => path.into(),
            None => OsString::new(),
        }
    }

    pub fn nth(n: usize, path: PathBuf) -> OsString {
        path.components()
            .nth(n)
            .map(|c| c.as_os_str())
            .unwrap_or_default()
            .into()
    }
}

pub mod remove {
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

    pub fn first(path: PathBuf) -> OsString {
        let mut iter = path.components();
        iter.next();
        iter.as_path().into()
    }

    pub fn nth(n: usize, path: PathBuf) -> OsString {
        path.into()
    }
}

pub mod replace {
    use super::*;

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

    pub fn first(path: PathBuf, s: &str) -> OsString {
        let mut iter = path.components();
        iter.next();
        let after_first = iter.as_path();
        PathBuf::from(s).join(after_first).into()
    }

    pub fn nth(n: usize, path: PathBuf, s: &str) -> OsString {
        path.into()
    }
}

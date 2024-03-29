use std::ffi::{OsStr, OsString};
use std::iter;
use std::path::PathBuf;

pub enum Component {
    Extension,
    Stem,
    Prefix,
    Name,
    Parent,
    First,
}

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
}

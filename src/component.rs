use std::ffi::{OsString, OsStr};
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

pub enum Action {
    Get,
    Remove,
    //Replace,
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
        match path.ancestors().last() {
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

    /*
    pub fn stem(path: PathBuf) -> OsString {

    }

    pub fn prefix(path: PathBuf) -> OsString {
        
    }

    pub fn name(path: PathBuf) -> OsString {
        
    }

    pub fn parent(path: PathBuf) -> OsString {

    }

    pub fn first(path: PathBuf) -> OsString {
        
    }
    */
}



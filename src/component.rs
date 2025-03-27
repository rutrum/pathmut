use crate::Action;
use std::iter;
use typed_path::{
    Components, Path, PathType, TypedPath, TypedPathBuf, WindowsComponent, WindowsComponents,
    WindowsEncoding, WindowsPath, WindowsPrefix, WindowsPrefixComponent,
};

// use clap::{builder::PossibleValue, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    Extension,
    Stem,
    Prefix,
    Name,
    Parent,
    // windows stuff
    // https://doc.rust-lang.org/stable/std/path/enum.Prefix.html
    Disk,
    Nth(isize),
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
            "disk" => Disk,
            _ => Nth(s.parse::<isize>().map_err(|_| ())?),
        };
        Ok(comp)
    }
}

pub fn arg_into_component(s: &str) -> Result<Component, String> {
    use Component::*;
    if let Ok(n) = s.parse::<isize>() {
        Ok(Nth(n))
    } else {
        let component = match s {
            "ext" => Extension,
            "stem" => Stem,
            "prefix" => Prefix,
            "name" => Name,
            "parent" => Parent,
            "disk" => Disk,
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

trait FilePrefix {
    // TODO: consider if this is the right name,
    // since it conflicts with WindowsPrefix
    // is this even a useful component?
    fn file_prefix(&self) -> Option<&[u8]>;
}

impl FilePrefix for TypedPath<'_> {
    // Referencing std::path::Path::file_prefix
    // https://doc.rust-lang.org/stable/src/std/path.rs.html#2648-2650
    fn file_prefix(&self) -> Option<&[u8]> {
        self.file_name()
            .map(split_file_at_dot)
            .map(|(before, _after)| before)
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
    pub fn action(self, action: &Action, path: &TypedPath) -> Vec<u8> {
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
            Disk => match path {
                TypedPath::Unix(_) => "".into(),
                TypedPath::Windows(w) => match w.components().next() {
                    Some(WindowsComponent::Prefix(prefix)) => match prefix.kind() {
                        WindowsPrefix::Disk(disk) => [disk].into(),
                        _ => "".into(),
                    },
                    _ => "".into(),
                },
            },
            Nth(n) => {
                let num_components: usize = path.components().count();
                let index: usize = if n >= 0 {
                    let positive: usize = n.try_into().unwrap();
                    positive
                } else {
                    let positive: usize = (-n).try_into().unwrap();
                    if positive > num_components {
                        // index is behind first component
                        return Vec::new();
                    }
                    num_components - positive
                };
                path.components()
                    .nth(index)
                    .map(|c| c.as_bytes().to_vec())
                    .unwrap_or_default()
            }
        }
    }

    pub fn has(self, path: &TypedPath) -> bool {
        !self.get(path).is_empty()
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
                    let name = if !after.is_empty() {
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
            Disk => match path {
                TypedPath::Unix(_) => path.to_path_buf().into_vec(),
                TypedPath::Windows(w) => {
                    let mut original = w.components();
                    let mut new = original.clone();
                    let has_prefix = match new.next() {
                        Some(WindowsComponent::Prefix(prefix)) => match prefix.kind() {
                            WindowsPrefix::Disk(_) => true,
                            _ => false,
                        },
                        _ => false,
                    };

                    let no_disk: &typed_path::Path<WindowsEncoding> = if has_prefix {
                        original.next(); // remove prefix
                                         //original.next(); // remove root
                        original.as_path()
                    } else {
                        original.as_path()
                    };

                    if value.len() == 0 {
                        return original
                            .as_path::<WindowsEncoding>()
                            .to_path_buf()
                            .into_vec();
                    }

                    // TEST: what happens if disk is more one char?
                    // what if 0 chars

                    // this is so garbage
                    let disk_str = format!(r"{}:", String::from_utf8(vec![value[0]]).unwrap());
                    let disk_path = WindowsPath::new(&disk_str);
                    let mut new_path = disk_path.to_path_buf();
                    new_path.push(no_disk);

                    //println!("{:?}", path);
                    //println!("{:?}", path.to_string_lossy());
                    //println!("no disk: {:?}", no_disk);
                    //println!("{:?}", no_disk.to_string());
                    //println!("just disk: {:?}", disk_path);
                    //println!("{:?}", disk_path.to_string());
                    //println!("{:?}", new_path);
                    //println!("{:?}", new_path.to_string());

                    new_path.into()
                }
            },
            Nth(n) => {
                // what if path is root?
                // todo

                let num_components: usize = path.components().count();
                let index: usize = if n >= 0 {
                    let positive: usize = n.try_into().unwrap();
                    positive
                } else {
                    let positive: usize = (-n).try_into().unwrap();
                    if positive > num_components {
                        // index is behind first component
                        return Vec::new();
                    }
                    num_components - positive
                };

                // what if n == number of components?
                let num_components = path.components().count();
                if num_components == index {
                    return path.join(value).into_vec();
                }

                // what if n > number of components?
                // todo

                path.components()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == index {
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
        //println!("{:?} {:?}", path, value);
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

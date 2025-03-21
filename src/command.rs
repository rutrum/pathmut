use clap::builder::PossibleValue;
use clap::ValueEnum;
use typed_path::PathType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Get,
    Set,
    Replace,
    Delete,
    Has,
    Is,
    Normalize,
    Convert,
    Info,
    Depth,
}

impl TryFrom<&str> for Command {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use Command::*;
        let command = match s {
            "get" => Get,
            "set" => Set,
            "replace" => Replace,
            "delete" => Delete,
            "has" => Has,
            "is" => Is,
            "normalize" => Normalize,
            "convert" => Convert,
            "info" => Info,
            "depth" => Depth,
            _ => Err(())?,
        };
        Ok(command)
    }
}

// is this necessary?
impl ValueEnum for Command {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Command::Get,
            Command::Set,
            Command::Replace,
            Command::Delete,
            Command::Has,
            Command::Is,
            Command::Normalize,
            Command::Convert,
            Command::Info,
            Command::Depth,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        use Command::*;
        let s = match self {
            Get => "get",
            Set => "set",
            Replace => "replace",
            Delete => "delete",
            Has => "has",
            Is => "is",
            Normalize => "normalize",
            Convert => "convert",
            Info => "info",
            Depth => "depth",
        };
        Some(PossibleValue::new(s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    Absolute,
    Relative,
    Unix,
    Windows,
    Normalized,
}

impl ValueEnum for Question {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Question::Absolute,
            Question::Relative,
            Question::Unix,
            Question::Windows,
            Question::Normalized,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        use Question::*;
        let s = match self {
            Absolute => "absolute",
            Relative => "relative",
            Unix => "unix",
            Windows => "windows",
            Normalized => "normalized",
        };
        Some(PossibleValue::new(s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    Unix,
    Windows,
}

impl Into<PathType> for PathKind {
    fn into(self) -> PathType {
        match self {
            Self::Unix => PathType::Unix,
            Self::Windows => PathType::Windows,
        }
    }
}

impl ValueEnum for PathKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[PathKind::Unix, PathKind::Windows]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let s = match self {
            Self::Unix => "unix",
            Self::Windows => "win",
        };
        Some(PossibleValue::new(s))
    }
}

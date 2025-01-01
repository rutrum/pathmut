use clap::builder::PossibleValue;
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Get,
    Set,
    Replace,
    Delete,
    Is,
    // CONVERT,
    // NORMALIZE
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
            "is" => Is,
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
            Command::Is,
            // Command::Convert,
            // Command::Normalize,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        use Command::*;
        let s = match self {
            Get => "get",
            Set => "set",
            Replace => "replace",
            Delete => "delete",
            Is => "is",
        };
        Some(PossibleValue::new(s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    ABSOLUTE,
    RELATIVE,
    // LINUX,
    // WINDOWS,
}

impl ValueEnum for Question {
    fn value_variants<'a>() -> &'a [Self] {
        &[Question::ABSOLUTE, Question::RELATIVE]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        use Question::*;
        let s = match self {
            ABSOLUTE => "absolute",
            RELATIVE => "relative",
        };
        Some(PossibleValue::new(s))
    }
}

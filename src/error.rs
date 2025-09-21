#![macro_use]

use std::cmp::{min};
use std::collections::HashSet;
use std::fmt::{Display};
use std::io::{Error};
use std::path::{PathBuf};

use colored::*;

use crate::config::*;
use crate::preprocess::*;
use crate::warnings::*;

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (
        std::io::Error::new(std::io::ErrorKind::Other, format!($($arg)*))
    )
}

pub trait ErrorExt<T> {
    fn prepend_error<M: AsRef<[u8]> + Display>(self, msg: M) -> Result<T, Error>;
    fn print_error(self, exit: bool) -> ();
}
impl<T> ErrorExt<T> for Result<T, Error> {
    fn prepend_error<M: AsRef<[u8]> + Display>(self, msg: M) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(error!("{}\n{}", msg, e))
        }
    }

    fn print_error(self, exit: bool) {
        if let Err(error) = self {
            eprintln!("{}: {}", "error".red().bold(), error);

            if exit {
                print_warning_summary();
                std::process::exit(1);
            }
        }
    }
}

pub trait PreprocessParseErrorExt<T> {
    fn format_error(self, origin: &Option<PathBuf>, input: &str) -> Result<T, Error>;
}
impl<T> PreprocessParseErrorExt<T> for Result<T, preprocess_grammar::ParseError> {
    fn format_error(self, origin: &Option<PathBuf>, input: &str) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(pe) => {
                let line_origin = pe.line - 1;
                let file_origin = match origin {
                    Some(path) => format!("{}:", path.to_str().unwrap().to_string()),
                    None => "".to_string()
                };

                let line = input.lines().nth(pe.line - 1).unwrap_or("");

                Err(format_parse_error(line, file_origin, line_origin, pe.column, pe.expected))
            }
        }
    }
}

pub trait ConfigParseErrorExt<T> {
    fn format_error(self, info: &PreprocessInfo, input: &str) -> Result<T, Error>;
}
impl<T> ConfigParseErrorExt<T> for Result<T, config_grammar::ParseError> {
    fn format_error(self, info: &PreprocessInfo, input: &str) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(pe) => {
                let line_origin = info.line_origins[min(pe.line, info.line_origins.len()) - 1].0 as usize;
                let file_origin = match &info.line_origins[min(pe.line, info.line_origins.len()) - 1].1 {
                    Some(path) => format!("{}:", path.to_str().unwrap().to_string()),
                    None => "".to_string()
                };

                let line = input.lines().nth(pe.line - 1).unwrap_or("");

                Err(format_parse_error(line, file_origin, line_origin, pe.column, pe.expected))
            }
        }
    }
}

fn format_parse_error(line: &str, file: String, line_number: usize, column_number: usize, expected: HashSet<&'static str>) -> Error {
    let trimmed = line.trim_start();
    let expected_list: Vec<String> = expected.iter().cloned().map(|x| format!("{:?}", x)).collect();

    error!("In line {}{}:\n\n  {}\n  {}{}\n\nUnexpected token \"{}\", expected: {}",
        file,
        line_number,
        trimmed,
        " ".to_string().repeat(column_number - 1 - (line.len() - trimmed.len())),
        "^".red().bold(),
        line.chars().map(|x| x.to_string()).nth(column_number - 1).unwrap_or_else(|| "\\n".to_string()),
        expected_list.join(", "))
}

pub fn warning<M: AsRef<[u8]> + Display>(msg: M, name: Option<&'static str>, location: (Option<M>,Option<u32>)) {
    // Check if warning should be shown
    if let Some(name) = name {
        if !raise_warning(name) {
            return; // Warning is muted or exceeded maximum
        }
    }

    let loc_str = if location.0.is_some() && location.1.is_some() {
        format!("In file {}:{}: ", location.0.unwrap(), location.1.unwrap())
    } else if location.0.is_some() {
        format!("In file {}: ", location.0.unwrap())
    } else if location.1.is_some() {
        format!("In line {}: ", location.1.unwrap())
    } else {
        "".to_string()
    };

    let name_str = match name {
        Some(name) => format!(" [{}]", name),
        None => "".to_string()
    };

    eprintln!("{}{}: {}{}", loc_str, "warning".yellow().bold(), msg, name_str);
}

pub fn warning_suppressed(name: Option<&'static str>) -> bool {
    if let Some(name) = name {
        is_warning_muted(name) || has_exceeded_maximum(name)
    } else {
        false
    }
}

pub fn print_warning_summary() {
    let summary = get_warning_summary();

    for (name, _raised, excess) in summary {
        if excess > 0 {
            if excess > 1 {
                warning(format!("{} warnings of type \"{}\" were suppressed to prevent spam. Use \"-w {}\" to disable these warnings entirely.",
                    excess, name, name), None, (None::<String>, None));
            } else {
                warning(format!("{} warning of type \"{}\" was suppressed to prevent spam. Use \"-w {}\" to disable these warnings entirely.",
                    excess, name, name), None, (None::<String>, None));
            }
        }
    }
}

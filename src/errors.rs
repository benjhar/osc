use std::{char, fmt::Display};

#[derive(Debug)]
pub enum Error {
    Utf8(String),
    DataLength(usize, usize),
    NoData(usize),
    UnrecognisedTypeTag(char),
    Alignment(usize, usize),
    Malformed(String),
    Socket(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Utf8(s) => f.write_fmt(format_args!("{} not valid utf-8", s)),
            DataLength(expected, received) => f.write_fmt(format_args!(
                "Expected {} elements, received {}",
                expected, received
            )),
            NoData(expected) => f.write_fmt(format_args!(
                "Expected {} elememts, received none",
                expected
            )),
            UnrecognisedTypeTag(tag) => f.write_fmt(format_args!("Unrecognised type tag: {}", tag)),
            Alignment(length, expected_alignment) => f.write_fmt(format_args!(
                "Data not {}-byte aligned. Received length {}",
                expected_alignment, length
            )),
            Malformed(s) => f.write_fmt(format_args!("{} malformed", s)),
            Socket(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

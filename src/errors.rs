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
    BlobSize(i32),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::{
            Alignment, BlobSize, DataLength, Malformed, NoData, Socket, UnrecognisedTypeTag, Utf8,
        };
        match self {
            Utf8(s) => f.write_fmt(format_args!("{s} not valid utf-8")),
            DataLength(expected, received) => f.write_fmt(format_args!(
                "Expected {expected} elements, received {received}",
            )),
            NoData(expected) => {
                f.write_fmt(format_args!("Expected {expected} elememts, received none",))
            }
            UnrecognisedTypeTag(tag) => f.write_fmt(format_args!("Unrecognised type tag: {tag}")),
            Alignment(length, expected_alignment) => f.write_fmt(format_args!(
                "Data not {expected_alignment}-byte aligned. Received length {length}",
            )),
            Malformed(s) => f.write_fmt(format_args!("{s} malformed")),
            Socket(e) => e.fmt(f),
            BlobSize(size) => f.write_fmt(format_args!(
                "Blob size invalid, found {size}, expected size >= 0 && size % 4 == 0"
            )),
        }
    }
}

impl std::error::Error for Error {}

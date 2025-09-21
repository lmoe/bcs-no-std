// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
#![no_std]

extern crate alloc;
use alloc::string::String;
use core::fmt;
use core::fmt::{Display, Formatter};
use serde::{de, ser};
use serde::de::StdError;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Eof,
    ExceededMaxLen(usize),
    ExceededContainerDepthLimit(&'static str),
    ExpectedBoolean,
    ExpectedMapKey,
    ExpectedMapValue,
    NonCanonicalMap,
    ExpectedOption,
    Custom(String),
    MissingLen,
    NotSupported(&'static str),
    RemainingInput(u64),
    Utf8,
    NonCanonicalUleb128Encoding,
    IntegerOverflowDuringUleb128Decoding,
    BufferFull,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Eof => write!(f, "unexpected end of input"),
            Error::ExceededMaxLen(len) => write!(f, "exceeded max sequence length: {}", len),
            Error::ExceededContainerDepthLimit(name) => {
                write!(f, "exceeded max container depth while entering: {}", name)
            }
            Error::ExpectedBoolean => write!(f, "expected boolean"),
            Error::ExpectedMapKey => write!(f, "expected map key"),
            Error::ExpectedMapValue => write!(f, "expected map value"),
            Error::NonCanonicalMap => {
                write!(f, "keys of serialized maps must be unique and in increasing order")
            }
            Error::ExpectedOption => write!(f, "expected option type"),
            Error::Custom(msg) => write!(f, "{}", msg),
            Error::MissingLen => write!(f, "sequence missing length"),
            Error::NotSupported(feature) => write!(f, "not supported: {}", feature),
            Error::RemainingInput(size) => write!(f, "remaining input"),
            Error::Utf8 => write!(f, "malformed utf8"),
            Error::NonCanonicalUleb128Encoding => {
                write!(f, "ULEB128 encoding was not minimal in size")
            }
            Error::IntegerOverflowDuringUleb128Decoding => {
                write!(f, "ULEB128-encoded integer did not fit in the target size")
            }
            Error::BufferFull => write!(f, "output buffer is full"),
        }
    }
}

impl StdError for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        use alloc::string::ToString;
        Error::Custom(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        use alloc::string::ToString;
        Error::Custom(msg.to_string())
    }
}
//! Support for working with different codepages / encodings.

use crate::error::{DecodeError, EncodeError};
use std::borrow::Cow;
use std::fmt::Debug;

/// Trait for reading strings from the database files.
///
/// If the `yore` feature isn't on, this is implemented only by [`UnicodeLossy`] and [`Unicode`].
///
/// If the `yore` feature is on, this is implemented by all [`yore::CodePage`].
///
/// Note: This trait might be extended with an `encode` function in the future.
pub trait Encoding: EncodingClone {
    /// Decode encoding into UTF-8 string. If codepoints can't be represented, an error is returned.
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError>;

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError>;
}

pub trait EncodingClone {
    fn clone_box(&self) -> Box<dyn Encoding>;
}

impl<T> EncodingClone for T
where
    T: 'static + Encoding + Clone,
{
    fn clone_box(&self) -> Box<dyn Encoding> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Encoding> {
    fn clone(&self) -> Box<dyn Encoding> {
        self.clone_box()
    }
}

/// This unit struct can be used as an [`Encoding`] to try to decode characters as Unicode,
/// falling back to the replacement character for unknown codepoints.
#[derive(Copy, Clone, Debug)]
pub struct UnicodeLossy;

/// This unit struct can be used as an [`Encoding`] to try to decode a string as Unicode,
/// and returning an error if unknown codepoints are encountered.
#[derive(Copy, Clone, Debug)]
pub struct Unicode;

/// This unit struct can be used as an [`Encoding`] to try to decode characters as ASCII,
/// and returning an error if non-ascii codepoints are encountered.
#[derive(Copy, Clone, Debug)]
pub struct Ascii;

/// Tries to decode as Unicode, replaces unknown codepoints with the replacement character.
impl Encoding for UnicodeLossy {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        Ok(String::from_utf8_lossy(bytes))
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        Ok(s.as_bytes().into())
    }
}

/// Tries to decode as Unicode, if unrepresentable characters are found, an [`Err`] is returned.
impl Encoding for Unicode {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        String::from_utf8(bytes.to_vec())
            .map(Cow::Owned)
            .map_err(DecodeError::FromUtf8)
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        Ok(s.as_bytes().into())
    }
}

/// Tries to decode as ASCII, if unrepresentable characters are found, an [`Err`] is returned.
impl Encoding for Ascii {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        if bytes.is_ascii() {
            // Since all ascii code points are compatible with utf-8
            // it is ok to unwrap here.
            Ok(String::from_utf8(bytes.to_vec()).unwrap().into())
        } else {
            Err(DecodeError::NotAscii)
        }
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        Ok(s.as_bytes().into())
    }
}

#[derive(Clone)]
pub(crate) struct DynEncoding {
    inner: Box<dyn Encoding>,
}

impl DynEncoding {
    pub(crate) fn new<E: Encoding + 'static>(encoding: E) -> Self {
        Self {
            inner: Box::new(encoding) as Box<dyn Encoding>,
        }
    }
}

impl Encoding for DynEncoding {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        self.inner.decode(bytes)
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        self.inner.encode(s)
    }
}

#[cfg(feature = "yore")]
impl<T> Encoding for T
where
    T: 'static + yore::CodePage + Clone,
{
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        self.decode(bytes).map_err(Into::into)
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        self.encode(s).map_err(Into::into)
    }
}

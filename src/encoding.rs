//! Support for working with different codepages / encodings.

use crate::error::{DecodeError, EncodeError};
use std::borrow::Cow;
use std::fmt::Debug;

pub trait AsCodePageMark {
    fn code_page_mark(&self) -> crate::CodePageMark;
}

macro_rules! impl_as_code_page_mark {
    ($($t:ty => $cp:path),* $(,)?) => {
        $(
            impl AsCodePageMark for $t {
                fn code_page_mark(&self) -> crate::CodePageMark {
                    $cp
                }
            }
        )*
    };
}

impl_as_code_page_mark!(
  Ascii => crate::CodePageMark::Utf8,
  UnicodeLossy => crate::CodePageMark::Utf8,
  Unicode => crate::CodePageMark::Utf8,
);

#[cfg(feature = "yore")]
impl_as_code_page_mark!(
    yore::code_pages::CP437 => crate::CodePageMark::CP437,
    yore::code_pages::CP850 => crate::CodePageMark::CP850,
    yore::code_pages::CP1252 => crate::CodePageMark::CP1252,
    yore::code_pages::CP852 => crate::CodePageMark::CP852,
    yore::code_pages::CP866 => crate::CodePageMark::CP866,
    yore::code_pages::CP865 => crate::CodePageMark::CP865,
    yore::code_pages::CP861 => crate::CodePageMark::CP861,
    yore::code_pages::CP874 => crate::CodePageMark::CP874,
    yore::code_pages::CP1255 => crate::CodePageMark::CP1255,
    yore::code_pages::CP1256 => crate::CodePageMark::CP1256,
    yore::code_pages::CP1250 => crate::CodePageMark::CP1250,
    yore::code_pages::CP1251 => crate::CodePageMark::CP1251,
    yore::code_pages::CP1254 => crate::CodePageMark::CP1254,
    yore::code_pages::CP1253 => crate::CodePageMark::CP1253,
);

/// Trait for reading strings from the database files.
///
/// If the `yore` feature isn't on, this is implemented only by [`UnicodeLossy`] and [`Unicode`].
///
/// If the `yore` feature is on, this is implemented by all [`yore::CodePage`].
///
/// Note: This trait might be extended with an `encode` function in the future.
pub trait Encoding: EncodingClone + AsCodePageMark + Send {
    /// Decode encoding into UTF-8 string. If codepoints can't be represented, an error is returned.
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError>;

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError>;
}

/// Trait to be able to clone a `Box<dyn Encoding>`
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
        // header can be ASCIIZ, terminated by \0, so only read up to this character
        let zero_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let str_bytes = &bytes[0..zero_pos];

        if str_bytes.is_ascii() {
            // Since all ascii code points are compatible with utf-8
            // it is ok to unwrap here.
            Ok(String::from_utf8(str_bytes.to_vec()).unwrap().into())
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

impl AsCodePageMark for DynEncoding {
    fn code_page_mark(&self) -> crate::CodePageMark {
        self.inner.code_page_mark()
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
    T: 'static + yore::CodePage + Clone + AsCodePageMark + Send,
{
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        self.decode(bytes).map_err(Into::into)
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        self.encode(s).map_err(Into::into)
    }
}

#[cfg(feature = "yore")]
#[derive(Copy, Clone)]
pub struct LossyCodePage<CP>(pub CP);

#[cfg(feature = "yore")]
impl<CP> AsCodePageMark for LossyCodePage<CP>
where
    CP: AsCodePageMark,
{
    fn code_page_mark(&self) -> crate::CodePageMark {
        self.0.code_page_mark()
    }
}

#[cfg(feature = "yore")]
impl<CP> Encoding for LossyCodePage<CP>
where
    CP: 'static + yore::CodePage + Clone + AsCodePageMark + Send,
{
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        Ok(self.0.decode_lossy(bytes))
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        Ok(self.0.encode_lossy(s, b'?'))
    }
}

//! Support for working with different codepages / encodings.

use crate::error::DecodeError;
use std::borrow::Cow;

/// Trait for reading strings from the database files.
///
/// If the `yore` feature isn't on, this is implemented only by [`UnicodeLossy`] and [`Unicode`].
///
/// If the `yore` feature is on, this is implemented by all [`yore::CodePage`].
///
/// Note: This trait might be extended with an `encode` function in the future.
pub trait Encoding {
    /// Decode encoding into UTF-8 string. If codepoints can't be represented, an error is returned.
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError>;
}

/// This unit struct can be used as an [`Encoding`] to try to decode characters as Unicode,
/// falling back to the replacement character for unknown codepoints.
pub struct UnicodeLossy;

/// This unit struct can be used as an [`Encoding`] to try to decode a string as Unicode,
/// and returning an error if unknown codepoints are encountered.
pub struct Unicode;

/// Tries to decode as Unicode, replaces unknown codepoints with the replacement character.
impl Encoding for UnicodeLossy {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        Ok(String::from_utf8_lossy(bytes))
    }
}

/// Tries to decode as Unicode, if unrepresentable characters are found, an [`Err`] is returned.
impl Encoding for Unicode {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        String::from_utf8(bytes.to_vec())
            .map(Cow::Owned)
            .map_err(DecodeError::FromUtf8)
    }
}

#[cfg(feature = "yore")]
impl<T> Encoding for T
where
    T: yore::CodePage,
{
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        self.decode(bytes).map_err(Into::into)
    }
}

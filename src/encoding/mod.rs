//! Support for working with different codepages / encodings.

use crate::error::{DecodeError, EncodeError};
use std::borrow::Cow;
use std::fmt::Debug;

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

#[cfg(feature = "encoding_rs")]
mod encoding_rs;
#[cfg(feature = "yore")]
mod yore;

#[cfg(feature = "yore")]
pub use yore::LossyCodePage;

#[cfg(feature = "encoding_rs")]
pub use encoding_rs::EncodingRs;

pub trait AsCodePageMark {
    fn code_page_mark(&self) -> crate::CodePageMark;
}

impl_as_code_page_mark!(
  Ascii => crate::CodePageMark::Utf8,
  UnicodeLossy => crate::CodePageMark::Utf8,
  Unicode => crate::CodePageMark::Utf8,
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
pub struct DynEncoding {
    inner: Box<dyn Encoding>,
}

impl DynEncoding {
    pub(crate) fn new<E: Encoding + 'static>(encoding: E) -> Self {
        Self {
            inner: Box::new(encoding) as Box<dyn Encoding>,
        }
    }

    #[cfg(any(feature = "yore", feature = "encoding_rs"))]
    pub fn from_name(name: &str) -> Option<Self> {
        let upper = name
            .trim()
            .trim_start_matches('\u{feff}')
            .to_ascii_uppercase();

        #[cfg(feature = "yore")]
        {
            use crate::CodePageMark;
            let encoding = match upper.as_str() {
                "UTF-8" | "65001" => CodePageMark::Utf8,
                "932" | "CP932" | "SHIFT_JIS" | "SJIS" => CodePageMark::CP932,
                "936" | "CP936" | "GBK" => CodePageMark::CP936,
                "949" | "CP949" | "EUC-KR" => CodePageMark::CP949,
                "latin1" => CodePageMark::CP1252,
                "866" | "CP866" => CodePageMark::CP866,
                "874" | "CP874" => CodePageMark::CP874,
                "1255" | "CP1255" => CodePageMark::CP1255,
                "1256" | "CP1256" => CodePageMark::CP1256,
                "1250" | "CP1250" => CodePageMark::CP1250,
                "1251" | "CP1251" | "ANSI 1251" => CodePageMark::CP1251,
                "1252" | "CP1252" => CodePageMark::CP1252,
                "1254" | "CP1254" => CodePageMark::CP1254,
                "1253" | "CP1253" => CodePageMark::CP1253,
                _ => CodePageMark::Undefined,
            }
            .to_encoding();

            #[cfg(feature = "encoding_rs")]
            {
                if encoding.is_some() {
                    return encoding;
                }
            }
            #[cfg(not(feature = "encoding_rs"))]
            {
                encoding
            }
        }

        // This is a best effort as a CPG file specification doesn't seem to exist
        #[cfg(feature = "encoding_rs")]
        match upper.as_str() {
            "UTF-8" | "65001" => Some(::encoding_rs::UTF_8),
            "932" | "CP932" | "SHIFT_JIS" | "SJIS" => Some(::encoding_rs::SHIFT_JIS),
            "936" | "CP936" | "GBK" => Some(::encoding_rs::GBK),
            "949" | "CP949" | "EUC-KR" => Some(::encoding_rs::EUC_KR),
            "BIG5" | "BIG-5" => Some(::encoding_rs::BIG5),
            "latin1" => Some(::encoding_rs::WINDOWS_1252), // Windows-1252 is a superset of latin1
            // For consistency with https://github.com/tmontaigu/Some(crate::dbase-rs/blob/master/src/::encoding/::encoding_rs.rs
            // I found almost no actual .cpg files on GitHub.
            "866" | "CP866" => Some(::encoding_rs::IBM866),
            "874" | "CP874" => Some(::encoding_rs::WINDOWS_874),
            "1255" | "CP1255" => Some(::encoding_rs::WINDOWS_1255),
            "1256" | "CP1256" => Some(::encoding_rs::WINDOWS_1256),
            "1250" | "CP1250" => Some(::encoding_rs::WINDOWS_1250),
            "1251" | "CP1251" | "ANSI 1251" => Some(::encoding_rs::WINDOWS_1251),
            "1252" | "CP1252" => Some(::encoding_rs::WINDOWS_1252),
            "1254" | "CP1254" => Some(::encoding_rs::WINDOWS_1254),
            "1253" | "CP1253" => Some(::encoding_rs::WINDOWS_1253),
            // It seems ISO-8859-* ::encodings can be stored as 8859* or 8859-*
            // - https://github.com/OSGeo/gdal/blob/12582d42366b101f75079dc832e34e4144cce62f/ogr/ogrsf_frmts/shape/ogrshapelayer.cpp#L517C38-L523
            // - https://github.com/qgis/QGIS/blob/master/tests/testdata/shapefile/iso-8859-1.cpg
            "ISO-8859-1" | "8859-1" | "88591" => Some(::encoding_rs::WINDOWS_1252),
            "ISO-8859-2" | "8859-2" | "88592" => Some(::encoding_rs::ISO_8859_2),
            "ISO-8859-3" | "8859-3" | "88593" => Some(::encoding_rs::ISO_8859_3),
            "ISO-8859-4" | "8859-4" | "88594" => Some(::encoding_rs::ISO_8859_4),
            "ISO-8859-5" | "8859-5" | "88595" => Some(::encoding_rs::ISO_8859_5),
            "ISO-8859-6" | "8859-6" | "88596" => Some(::encoding_rs::ISO_8859_6),
            "ISO-8859-7" | "8859-7" | "88597" => Some(::encoding_rs::ISO_8859_7),
            "ISO-8859-8" | "8859-8" | "88598" => Some(::encoding_rs::ISO_8859_8),
            "ISO-8859-9" | "8859-9" | "88599" => Some(::encoding_rs::WINDOWS_1254),
            "ISO-8859-10" | "8859-10" | "885910" => Some(::encoding_rs::ISO_8859_10),
            "ISO-8859-13" | "8859-13" | "885913" => Some(::encoding_rs::ISO_8859_13),
            "ISO-8859-14" | "8859-14" | "885914" => Some(::encoding_rs::ISO_8859_14),
            "ISO-8859-15" | "8859-15" | "885915" => Some(::encoding_rs::ISO_8859_15),
            "ISO-8859-16" | "8859-16" | "885916" => Some(::encoding_rs::ISO_8859_16),
            _ => None,
        }
        .map(|e| Self::new(encoding_rs::EncodingRs::from(e)))
    }
}

impl From<Ascii> for DynEncoding {
    fn from(value: Ascii) -> Self {
        Self::new(value)
    }
}

impl From<Unicode> for DynEncoding {
    fn from(value: Unicode) -> Self {
        Self::new(value)
    }
}

impl From<UnicodeLossy> for DynEncoding {
    fn from(value: UnicodeLossy) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "encoding_rs")]
impl From<encoding_rs::EncodingRs> for DynEncoding {
    fn from(value: encoding_rs::EncodingRs) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "yore")]
impl<CP> From<LossyCodePage<CP>> for DynEncoding
where
    CP: 'static + ::yore::CodePage + Clone + AsCodePageMark + Send,
{
    fn from(value: LossyCodePage<CP>) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "yore")]
impl<CP> From<CP> for DynEncoding
where
    CP: 'static + ::yore::CodePage + Clone + AsCodePageMark + Send,
{
    fn from(value: CP) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Debug for DynEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynEncoding")
            .field(&self.inner.code_page_mark())
            .finish()
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

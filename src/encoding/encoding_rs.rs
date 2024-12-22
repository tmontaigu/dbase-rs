use super::{AsCodePageMark, DecodeError, EncodeError, Encoding};

use std::borrow::Cow;

#[derive(Copy, Clone)]
pub struct EncodingRs(&'static encoding_rs::Encoding);

impl From<&'static encoding_rs::Encoding> for EncodingRs {
    fn from(item: &'static encoding_rs::Encoding) -> Self {
        EncodingRs(item)
    }
}

impl AsCodePageMark for EncodingRs {
    fn code_page_mark(&self) -> crate::CodePageMark {
        let code_page = codepage::from_encoding(self.0).unwrap();
        match code_page {
            1252 => crate::CodePageMark::CP1252,
            866 => crate::CodePageMark::CP866,
            874 => crate::CodePageMark::CP874,
            1255 => crate::CodePageMark::CP1255,
            1256 => crate::CodePageMark::CP1256,
            1250 => crate::CodePageMark::CP1250,
            1251 => crate::CodePageMark::CP1251,
            1254 => crate::CodePageMark::CP1254,
            1253 => crate::CodePageMark::CP1253,
            65001 => crate::CodePageMark::Utf8,
            950 => crate::CodePageMark::CP950,
            949 => crate::CodePageMark::CP949,
            936 => crate::CodePageMark::CP936,
            932 => crate::CodePageMark::CP932,
            _ => crate::CodePageMark::Utf8,
        }
    }
}

impl Encoding for EncodingRs {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        Ok(self.0.decode(bytes).0)
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        Ok(self.0.encode(s).0)
    }
}

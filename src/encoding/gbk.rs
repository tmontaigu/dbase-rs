use super::{AsCodePageMark, DecodeError, EncodeError, Encoding};
use encoding_rs::GBK;
use std::borrow::Cow;

#[derive(Copy, Clone)]
pub struct GbkEncoding;

impl AsCodePageMark for GbkEncoding {
    fn code_page_mark(&self) -> crate::CodePageMark {
        crate::CodePageMark::CP936
    }
}

impl Encoding for GbkEncoding {
    fn decode<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, str>, DecodeError> {
        let (cow, _, had_errors) = GBK.decode(bytes);
        if had_errors {
            Err(DecodeError::Message("GBK decode error".to_string()))
        } else {
            Ok(cow)
        }
    }

    fn encode<'a>(&self, s: &'a str) -> Result<Cow<'a, [u8]>, EncodeError> {
        let (cow, _, had_errors) = GBK.encode(s);
        if had_errors {
            Err(EncodeError::Message("GBK encode error".to_string()))
        } else {
            Ok(cow)
        }
    }
}

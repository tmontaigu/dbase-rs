use std::borrow::Cow;

use super::{AsCodePageMark, DecodeError, EncodeError, Encoding};

impl_as_code_page_mark!(
    yore::code_pages::CP437 => crate::CodePageMark::CP437,
    yore::code_pages::CP850 => crate::CodePageMark::CP850,
    yore::code_pages::CP1252 => crate::CodePageMark::CP1252,
    yore::code_pages::CP852 => crate::CodePageMark::CP852,
    yore::code_pages::CP866 => crate::CodePageMark::CP866,
    yore::code_pages::CP865 => crate::CodePageMark::CP865,
    yore::code_pages::CP861 => crate::CodePageMark::CP861,
    yore::code_pages::CP874 => crate::CodePageMark::CP874,
    yore::code_pages::CP936 => crate::CodePageMark::CP936,
    yore::code_pages::CP1255 => crate::CodePageMark::CP1255,
    yore::code_pages::CP1256 => crate::CodePageMark::CP1256,
    yore::code_pages::CP1250 => crate::CodePageMark::CP1250,
    yore::code_pages::CP1251 => crate::CodePageMark::CP1251,
    yore::code_pages::CP1254 => crate::CodePageMark::CP1254,
    yore::code_pages::CP1253 => crate::CodePageMark::CP1253,
);

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

#[derive(Copy, Clone)]
pub struct LossyCodePage<CP>(pub CP);

impl<CP> AsCodePageMark for LossyCodePage<CP>
where
    CP: AsCodePageMark,
{
    fn code_page_mark(&self) -> crate::CodePageMark {
        self.0.code_page_mark()
    }
}

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

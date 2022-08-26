use crate::{CodePageMark, FieldConversionError, FieldInfo};
use std::string::FromUtf8Error;

#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Wrapper of `std::io::Error` to forward any reading/writing error
    IoError(std::io::Error),
    /// Wrapper to forward errors whe trying to parse a float from the file
    ParseFloatError(std::num::ParseFloatError),
    /// Wrapper to forward errors whe trying to parse an integer value from the file
    ParseIntError(std::num::ParseIntError),
    /// The Field as an invalid FieldType
    InvalidFieldType(char),
    /// Happens when at least one field is a Memo type
    /// and the that additional memo file could not be found / was not given
    MissingMemoFile,
    /// Something went wrong when we tried to open the associated memo file
    ErrorOpeningMemoFile(std::io::Error),
    /// The conversion from a FieldValue to another type could not be made
    BadConversion(FieldConversionError),
    /// End of the record, there are no more fields
    EndOfRecord,
    /// Not all the fields declared to the writer were given
    NotEnoughFields,
    /// More fields than expected were given to the writer
    TooManyFields,
    /// The type of the value for the field is not compatible with the
    /// dbase field's type
    IncompatibleType,
    /// The Code Page is not supported
    UnsupportedCodePage(CodePageMark),
    /// A string from the database could not be decoded
    StringDecodeError(DecodeError),
    /// A string from the database could not be encoded
    StringEncodeError(EncodeError),
    Message(String),
}

/// The error type for this crate
pub struct Error {
    pub(crate) record_num: usize,
    pub(crate) field: Option<FieldInfo>,
    pub(crate) kind: ErrorKind,
}

impl Error {
    pub(crate) fn new(field_error: FieldIOError, current_record: usize) -> Self {
        Self {
            record_num: current_record,
            field: field_error.field,
            kind: field_error.kind,
        }
    }

    pub(crate) fn io_error(error: std::io::Error, current_record: usize) -> Self {
        Self {
            record_num: current_record,
            field: None,
            kind: ErrorKind::IoError(error),
        }
    }

    /// Returns the kind of error that happened
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Returns the index of record index for which the error occurred
    ///
    /// 0 may be the first record or an error that occurred before
    /// handling the first record (eg: an error reading the header)
    pub fn record_num(&self) -> usize {
        self.record_num
    }

    /// Returns the information of the record field for which the error occurred
    pub fn field(&self) -> &Option<FieldInfo> {
        &self.field
    }
}

#[derive(Debug)]
pub struct FieldIOError {
    pub(crate) field: Option<FieldInfo>,
    pub(crate) kind: ErrorKind,
}

impl FieldIOError {
    pub fn new(kind: ErrorKind, field: Option<FieldInfo>) -> Self {
        Self { field, kind }
    }

    pub(crate) fn end_of_record() -> Self {
        Self {
            field: None,
            kind: ErrorKind::EndOfRecord,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(e: std::io::Error) -> Self {
        ErrorKind::IoError(e)
    }
}

impl From<std::num::ParseFloatError> for ErrorKind {
    fn from(p: std::num::ParseFloatError) -> Self {
        ErrorKind::ParseFloatError(p)
    }
}

impl From<std::num::ParseIntError> for ErrorKind {
    fn from(p: std::num::ParseIntError) -> Self {
        ErrorKind::ParseIntError(p)
    }
}

impl From<FieldConversionError> for ErrorKind {
    fn from(e: FieldConversionError) -> Self {
        ErrorKind::BadConversion(e)
    }
}

impl From<DecodeError> for ErrorKind {
    fn from(e: DecodeError) -> Self {
        ErrorKind::StringDecodeError(e)
    }
}

impl From<EncodeError> for ErrorKind {
    fn from(e: EncodeError) -> Self {
        ErrorKind::StringEncodeError(e)
    }
}

impl From<FieldConversionError> for FieldIOError {
    fn from(e: FieldConversionError) -> Self {
        FieldIOError::new(ErrorKind::BadConversion(e), None)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_info) = &self.field {
            write!(
                f,
                "ReadingError {{ record_num: {}, kind: {:?}, {} }}",
                self.record_num, self.kind, field_info
            )
        } else {
            write!(
                f,
                "ReadingError {{ record_num: {}, kind: {:?} }}",
                self.record_num, self.kind
            )
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for FieldIOError {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::IoError(_) => "An I/O error happened",
            ErrorKind::ParseFloatError(_) => "Float value could not be obtained",
            ErrorKind::ParseIntError(_) => "Float value could not be obtained",
            ErrorKind::InvalidFieldType(_) => "The FieldType code is note a valid one",
            ErrorKind::MissingMemoFile => "The memo file could not be found",
            ErrorKind::ErrorOpeningMemoFile(_) => {
                "An error occurred when trying to open the memo file"
            }
            ErrorKind::BadConversion(_) => "The convertion cannot be made",
            ErrorKind::EndOfRecord => "End of record reached, no more fields left",
            ErrorKind::NotEnoughFields => {
                "The writer did not expected that many fields for the record"
            }
            ErrorKind::TooManyFields => "The writer expected to write more fields for the record",
            ErrorKind::IncompatibleType => "The types are not compatible",
            ErrorKind::StringDecodeError(_) => "A string from the database could not be decoded",
            ErrorKind::StringEncodeError(_) => "A string from the database could not be encoded",
            ErrorKind::UnsupportedCodePage(_) => "The code page is not supported",
            ErrorKind::Message(ref msg) => msg,
        }
    }
}

impl std::fmt::Display for FieldIOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_info) = &self.field {
            write!(
                f,
                "FieldIOError {{ kind: {:?}, {} }}",
                self.kind, field_info
            )
        } else {
            write!(f, "ReadingError {{ kind: {:?} }}", self.kind)
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
    Message(String),
    FromUtf8(FromUtf8Error),
    NotAscii,
    #[cfg(feature = "yore")]
    Yore(yore::DecodeError),
}

impl From<String> for DecodeError {
    fn from(msg: String) -> Self {
        Self::Message(msg)
    }
}

#[cfg(feature = "yore")]
impl From<yore::DecodeError> for DecodeError {
    fn from(e: yore::DecodeError) -> Self {
        DecodeError::Yore(e)
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    Message(String),
    #[cfg(feature = "yore")]
    Yore(yore::EncodeError),
}

impl From<String> for EncodeError {
    fn from(msg: String) -> Self {
        Self::Message(msg)
    }
}

#[cfg(feature = "yore")]
impl From<yore::EncodeError> for EncodeError {
    fn from(e: yore::EncodeError) -> Self {
        EncodeError::Yore(e)
    }
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for EncodeError {}

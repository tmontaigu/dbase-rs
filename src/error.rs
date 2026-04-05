use crate::{
    CodePageMark, FieldConversionError, FieldIndex, FieldInfo, FieldType, RecordIndex,
    field::types::TimeError,
};
use std::string::FromUtf8Error;

#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Wrapper of `std::io::Error` to forward any reading/writing error
    IoError(std::io::Error),
    /// Wrapper to forward errors when trying to parse a float from the file
    ParseFloatError(std::num::ParseFloatError),
    /// Wrapper to forward errors when trying to parse an integer value from the file
    ParseIntError(std::num::ParseIntError),
    /// A date field could not be parsed
    InvalidDate(crate::field::types::DateParseError),
    /// The time is invalid
    InvalidTime(TimeError),
    /// The field has an invalid FieldType
    InvalidFieldType(char),
    /// Happens when at least one field is a Memo type
    /// and the additional memo file could not be found / was not given
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
    /// The file is globally invalid, it may not even be a proper dbase file
    InvalidFile(&'static str),
    Message(String),
}

#[derive(Debug, Clone)]
pub(crate) struct FieldContext {
    pub(crate) index: FieldIndex,
    pub(crate) name: String,
    pub(crate) kind: FieldType,
}

#[derive(Debug, Clone)]
pub(crate) enum RecordField {
    DeletionFlag,
    Field(FieldContext),
}

#[derive(Debug, Clone)]
pub(crate) struct RecordContext {
    index: RecordIndex,
    field: Option<RecordField>,
}

#[derive(Debug)]
pub struct FieldError {
    pub(crate) kind: ErrorKind,
    pub(crate) context: Option<FieldContext>,
}

impl FieldError {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl FieldError {
    pub(crate) fn from_info(
        index: FieldIndex,
        info: &FieldInfo,
        err: impl Into<ErrorKind>,
    ) -> Self {
        Self {
            kind: err.into(),
            context: Some(FieldContext {
                index,
                name: info.name.clone(),
                kind: info.field_type(),
            }),
        }
    }

    pub(crate) const fn end_of_record() -> Self {
        Self {
            kind: ErrorKind::EndOfRecord,
            context: None,
        }
    }

    pub(crate) fn without_context(kind: impl Into<ErrorKind>) -> Self {
        Self {
            kind: kind.into(),
            context: None,
        }
    }
}

impl std::error::Error for FieldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.context {
            Some(FieldContext { index, name, kind }) => {
                write!(
                    f,
                    "An error occurred at field #{} ('{name}', {kind}): {}",
                    index.0, self.kind
                )
            }
            None => write!(f, "An error occurred for a field: {}", self.kind),
        }
    }
}

/// The error type for this crate
#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) record_context: Option<RecordContext>,
}

impl Error {
    /// Shortcut for when the error happens before parsing the fields
    /// i.e when parsing header
    pub(crate) fn header(kind: impl Into<ErrorKind>) -> Self {
        Self {
            kind: kind.into(),
            record_context: None,
        }
    }

    /// Shortcut for when the error happens at the 'global' record level
    /// for example when reading the whole record into a buffer, seeking to the
    /// beginning of the record, etc
    pub(crate) fn record(record_index: RecordIndex, kind: impl Into<ErrorKind>) -> Self {
        Self {
            kind: kind.into(),
            record_context: Some(RecordContext {
                index: record_index,
                field: None,
            }),
        }
    }

    pub(crate) fn deletion_flag(record_index: RecordIndex, kind: impl Into<ErrorKind>) -> Self {
        Self {
            kind: kind.into(),
            record_context: Some(RecordContext {
                index: record_index,
                field: Some(RecordField::DeletionFlag),
            }),
        }
    }

    /// Most complete error, for when it happened when reading a field
    pub(crate) fn field(
        record_index: RecordIndex,
        field_ctx: FieldContext,
        kind: impl Into<ErrorKind>,
    ) -> Self {
        Self {
            kind: kind.into(),
            record_context: Some(RecordContext {
                index: record_index,
                field: Some(RecordField::Field(field_ctx)),
            }),
        }
    }

    pub(crate) fn from_field_error(index: RecordIndex, error: FieldError) -> Self {
        let FieldError { kind, context } = error;
        match context {
            Some(c) => Self::field(index, c, kind),
            None => Self::record(index, kind),
        }
    }

    /// Returns the kind of error that happened
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Returns record index for which the error occurred
    ///
    /// None means the error happened outside of reading/writing a record, mainly
    /// when reading the header
    pub fn record_index(&self) -> Option<RecordIndex> {
        self.record_context.as_ref().map(|ctx| ctx.index)
    }

    /// Returns the field index for which the error occurred
    ///
    /// None means the error happened outside of reading/writing a field
    pub fn field_index(&self) -> Option<FieldIndex> {
        self.record_context.as_ref().and_then(|ctx| {
            ctx.field.as_ref().and_then(|f| {
                if let RecordField::Field(ctx) = f {
                    Some(ctx.index)
                } else {
                    None
                }
            })
        })
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

impl From<crate::field::types::DateParseError> for ErrorKind {
    fn from(e: crate::field::types::DateParseError) -> Self {
        ErrorKind::InvalidDate(e)
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

impl From<TimeError> for ErrorKind {
    fn from(e: TimeError) -> Self {
        Self::InvalidTime(e)
    }
}

impl ErrorKind {
    pub(crate) fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorKind::IoError(e) => Some(e),
            ErrorKind::ParseFloatError(e) => Some(e),
            ErrorKind::ParseIntError(e) => Some(e),
            ErrorKind::InvalidDate(e) => Some(e),
            ErrorKind::InvalidTime(e) => Some(e),
            ErrorKind::ErrorOpeningMemoFile(e) => Some(e),
            ErrorKind::BadConversion(e) => Some(e),
            ErrorKind::StringDecodeError(e) => Some(e),
            ErrorKind::StringEncodeError(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ctx) = &self.record_context {
            match &ctx.field {
                Some(record_field) => match record_field {
                    RecordField::DeletionFlag => {
                        write!(
                            f,
                            "An error occurred in record {} at deletion flag: {}",
                            ctx.index.0, self.kind
                        )
                    }
                    RecordField::Field(FieldContext { index, name, kind }) => {
                        write!(
                            f,
                            "An error occurred in record {} at field #{} ('{name}', {kind}): {}",
                            ctx.index.0, index.0, self.kind
                        )
                    }
                },
                None => {
                    write!(
                        f,
                        "An error occurred in record {}: {}",
                        ctx.index.0, self.kind
                    )
                }
            }
        } else {
            write!(f, "Error {{ kind: {} }}", self.kind)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::IoError(err) => write!(f, "An I/O error happened: {err}"),
            ErrorKind::ParseFloatError(err) => {
                write!(f, "Float value could not be obtained: {err}")
            }
            ErrorKind::ParseIntError(err) => {
                write!(f, "Integer value could not be obtained: {err}")
            }
            ErrorKind::InvalidDate(err) => {
                write!(f, "Invalid date: {err}")
            }
            ErrorKind::InvalidTime(time_error) => {
                write!(f, "Invalid time: {time_error}")
            }
            ErrorKind::InvalidFieldType(c) => {
                write!(f, "The FieldType code '{c}' is not a valid one")
            }
            ErrorKind::MissingMemoFile => write!(f, "The memo file could not be found"),
            ErrorKind::ErrorOpeningMemoFile(err) => {
                write!(
                    f,
                    "An error occurred when trying to open the memo file: {err}"
                )
            }
            ErrorKind::BadConversion(err) => write!(f, "The conversion cannot be made: {err}"),
            ErrorKind::EndOfRecord => write!(f, "End of record reached, no more fields left"),
            ErrorKind::NotEnoughFields => {
                write!(
                    f,
                    "The writer did not expected that many fields for the record"
                )
            }
            ErrorKind::TooManyFields => {
                write!(f, "The writer expected to write more fields for the record")
            }
            ErrorKind::IncompatibleType => write!(f, "The types are not compatible"),
            ErrorKind::StringDecodeError(err) => {
                write!(f, "A string from the database could not be decoded: {err}")
            }
            ErrorKind::StringEncodeError(err) => {
                write!(f, "A string from the database could not be encoded: {err}")
            }
            ErrorKind::UnsupportedCodePage(code) => {
                write!(f, "The code page '{code:?}' is not supported")
            }
            ErrorKind::InvalidFile(details) => write!(f, "The file is invalid: {details}"),
            ErrorKind::Message(msg) => write!(f, "{msg}"),
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
        match self {
            DecodeError::Message(msg) => write!(f, "{msg}"),
            DecodeError::FromUtf8(e) => write!(f, "{e}"),
            DecodeError::NotAscii => write!(f, "string contains non-ASCII bytes"),
            #[cfg(feature = "yore")]
            DecodeError::Yore(e) => write!(f, "{e}"),
        }
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
        match self {
            EncodeError::Message(msg) => write!(f, "{msg}"),
            #[cfg(feature = "yore")]
            EncodeError::Yore(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EncodeError {}

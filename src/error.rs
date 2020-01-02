use {FieldConversionError, FieldInfo};

#[derive(Debug)]
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
    NotEnoughFields,
    TooManyFields,
    /// The type of the value for the field is not compatible with the
    /// dbase field's type
    IncompatibleType,
    Message(String),
}

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
    pub(crate) fn new(kind: ErrorKind, field: Option<FieldInfo>) -> Self {
        Self { field, kind }
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

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::EndOfRecord => write!(
                f,
                "ReadingError {{ record_num: {},  kind: EndOfRecord }}",
                self.record_num
            ),
            kind => write!(
                f,
                "ReadingError {{ record_num: {}, kind: {:?}, {:?} }}",
                self.record_num, kind, self.field
            ),
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

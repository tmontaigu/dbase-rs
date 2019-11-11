//! dbase is rust library meant to read and write
//!
//! # Reading
//!
//! To Read the whole file at once you should use the [read](fn.read.html) function.
//!
//! Once you have access to the records, you will have to `match` against the real
//! [FieldValue](enum.FieldValue.html)
//!
//! # Examples
//!
//! ```
//! use dbase::FieldValue;
//! let records = dbase::read("tests/data/line.dbf").unwrap();
//! for record in records {
//!     for (name, value) in record {
//!         println!("{} -> {:?}", name, value);
//!         match value {
//!             FieldValue::Character(Some(string)) => println!("Got string: {}", string),
//!             FieldValue::Numeric(value) => println!("Got numeric value of  {:?}", value),
//!             _ => {}
//!         }
//!     }
//!}
//! ```
//!
//! You can also create a [Reader](reading/Reader.struct.html) and iterate over the records.
//!
//! ```
//! let mut reader = dbase::Reader::from_path("tests/data/line.dbf").unwrap();
//! for record_result in reader.iter_records() {
//!     let record = record_result.unwrap();
//!     for (name, value) in record {
//!         println!("name: {}, value: {:?}", name, value);
//!     }
//! }
//!
//! ```
//!

//https://dbfviewer.com/dbf-file-structure/

extern crate byteorder;

mod header;
mod reading;
mod record;
mod writing;


use std::io::{Read, Seek};

pub use reading::{read, Reader, Record};
pub use reading::FieldIterator;
pub use record::field::{FieldValue, Date, DateTime};
use record::FieldConversionError;
pub use record::FieldFlags;
pub use record::RecordFieldInfo;
pub use writing::{TableWriter, TableWriterBuilder};
pub use writing::WritableRecord;

/// Errors that may happen when reading a .dbf
#[derive(Debug)]
pub enum Error {
    /// Wrapper of `std::io::Error` to forward any reading/writing error
    IoError(std::io::Error),
    /// Wrapper to forward errors whe trying to parse a float from the file
    ParseFloatError(std::num::ParseFloatError),
    /// Wrapper to forward errors whe trying to parse an integer value from the file
    ParseIntError(std::num::ParseIntError),
    /// The Field as an invalid FieldType
    InvalidFieldType(char),
    InvalidDate,
    FieldLengthTooLong,
    FieldNameTooLong,
    /// Happens when at least one field is a Memo type
    /// and the that additional memo file could not be found / was not given
    MissingMemoFile,
    ErrorOpeningMemoFile(std::io::Error),
    BadConversion(FieldConversionError),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(p: std::num::ParseFloatError) -> Self {
        Error::ParseFloatError(p)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(p: std::num::ParseIntError) -> Self {
        Error::ParseIntError(p)
    }
}

impl From<FieldConversionError> for Error {
    fn from(e: FieldConversionError) -> Self {
        Error::BadConversion(e)
    }
}

/// Trait to be implemented by structs that represent records read from a
/// dBase file.
///
/// The field iterator gives access to methods that allow to read fields value
/// or skip them.
/// It is not required that the user reads / skips all the fields in a record,
/// in other words: it is not required to consume the iterator.
pub trait ReadableRecord: Sized {
    fn read_using<'a, 'b, T, I>(field_iterator: &mut FieldIterator<'a, 'b, T, I>) -> Result<Self, Error>
        where T: Read + Seek,
              I: Iterator<Item=&'b RecordFieldInfo>;
}
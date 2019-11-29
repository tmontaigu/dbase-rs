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
#[cfg(feature = "serde")]
extern crate serde;

mod header;
mod reading;
mod record;
mod writing;

#[cfg(feature = "serde")]
mod de;
#[cfg(feature = "serde")]
mod ser;

pub use reading::{read, Reader, Record, FieldIterator, ReadableRecord};
pub use record::field::{FieldValue, Date, DateTime};
pub use record::{FieldInfo, FieldName, FieldFlags, FieldConversionError};
pub use writing::{TableWriter, TableWriterBuilder, WritableRecord, FieldWriter};
use std::fmt::{Display, Formatter};
use record::field::FieldType;

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
    FieldNameTooLong,
    /// Happens when at least one field is a Memo type
    /// and the that additional memo file could not be found / was not given
    MissingMemoFile,
    ErrorOpeningMemoFile(std::io::Error),
    BadConversion(FieldConversionError),
    EndOfRecord,
    MissingFields, // FIXME find something better ?
    BadFieldType{expected: FieldType, got: FieldType, field_name: String},
    Message(String),
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "serde")]
impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Message(ref msg) => { msg }
            Error::IoError(_) => { "A std::io::Error occurred" }
            Error::ParseFloatError(_) => { "Failed to parse a float" }
            Error::ParseIntError(_) => { "Failed to parse an int" }
            Error::InvalidFieldType(_) => { "The field type is invalid" }
            Error::InvalidDate => { "The date is invalid" }
            Error::FieldNameTooLong => { "The Field name is too long to fit" }
            Error::MissingMemoFile => { "A memo file was expected but could not be found" }
            Error::ErrorOpeningMemoFile(_) => { "An error occurred when trying to open the memo file" }
            Error::BadConversion(_) => { "BadConversion" }
            Error::EndOfRecord => { "EndOfRecord" }
            Error::MissingFields => { "Missing at least one field" }
            Error::BadFieldType { expected: e, got: g, field_name: n } => {
                stringify!("For field named '{}', expected field_type: {}, but was give: {}", n, e, g)
            }
        }
    }
}

#[macro_export]
macro_rules! dbase_record {
    (
        $(#[derive($($derives:meta),*)])?
        struct $name:ident {
            $( $field_name:ident: $field_type:ty),+
            $(,)?
        }
    ) => {

        $(#[derive($($derives),*)])?
        struct $name {
            $($field_name: $field_type),+
        }

        impl ReadableRecord for $name {
                fn read_using<T>(field_iterator: &mut FieldIterator<T>) -> Result<Self, Error>
                    where T: Read + Seek
                {
                  Ok(Self {
                    $(
                        $field_name: field_iterator
                            .read_next_field_as::<$field_type>()
                            .ok_or(Error::EndOfRecord)??
                            .value
                    ),+
                  })
              }
        }

       impl WritableRecord for $name {
             fn write_using<'a, W: Write>(&self, field_writer: &mut FieldWriter<'a, W>) -> Result<(), Error> {
                $(
                    field_writer.write_next_field_value(&self.$field_name)?;
                )+
                Ok(())
             }
        }

    };
}

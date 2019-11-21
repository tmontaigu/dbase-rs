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
extern crate serde;

mod header;
mod reading;
mod record;
mod writing;
mod de;

pub use reading::{read, Reader, Record, FieldIterator, ReadableRecord};
pub use record::field::{FieldValue, Date, DateTime};
pub use record::{FieldInfo, FieldName, FieldFlags, FieldConversionError};
pub use writing::{TableWriter, TableWriterBuilder, WritableRecord, FieldValueCollector};

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
                fn read_using<'a, 'b, T, I>(field_iterator: &mut FieldIterator<'a, 'b, T, I>) -> Result<Self, Error>
                    where T: Read + Seek,
                          I: Iterator<Item=&'b FieldInfo> + 'b {
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
             fn values_for_fields(self, _field_names: &[&str], values: &mut dbase::FieldValueCollector) {
                $(
                    values.push(FieldValue::from(self.$field_name));
                )+
             }
        }
    };
}

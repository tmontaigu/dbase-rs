//! Module with all structs & functions charged of writing .dbf file content
use std::fs::File;
use std::io::{BufWriter, Cursor, Write, Seek, SeekFrom};
use std::path::Path;

use byteorder::WriteBytesExt;

use crate::header::Header;
use crate::reading::TERMINATOR_VALUE;
use crate::record::{field::FieldType, FieldInfo, FieldName};
use crate::{Error, ErrorKind, FieldIOError, Record};
use reading::TableInfo;

/// A dbase file ends with this byte
const FILE_TERMINATOR: u8 = 0x1A;

/// Builder to be used to create a [TableWriter](struct.TableWriter.html).
///
/// The dBase format il akin to a database, thus you have to specify the fields
/// of the record you are going to write
///
/// # Example
///
/// Here we will create a writer that will be able to write records with 2 character fields
/// where both fields cannot exceed 50 bytes in length.
///
/// The writer will write its data to a cursor, but files are also supported.
/// ```
/// use dbase::{TableWriterBuilder, FieldName};
/// use std::convert::TryFrom;
/// use std::io::Cursor;
///
/// let writer = TableWriterBuilder::new()
///     .add_character_field(FieldName::try_from("First Name").unwrap(), 50)
///     .add_character_field(FieldName::try_from("Last Name").unwrap(), 50)
///     .build_with_dest(Cursor::new(Vec::<u8>::new()));
/// ```
pub struct TableWriterBuilder {
    v: Vec<FieldInfo>,
    hdr: Header,
}

impl TableWriterBuilder {
    /// Creates a new builder with an empty dBase record definition
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the field definition from the reader to construct the TableWriter
    ///
    /// # Example
    /// ```
    /// use dbase::{FieldValue, TableWriterBuilder};
    /// use std::io::Cursor;
    /// let mut  reader = dbase::Reader::from_path("tests/data/stations.dbf").unwrap();
    /// let mut stations = reader.read().unwrap();
    /// let old_name = stations[0].insert("name".parse().unwrap(), String::from("Montparnasse").into());
    /// assert_eq!(old_name, Some(FieldValue::Character(Some("Van Dorn Street".parse().unwrap()))));
    ///
    /// let mut writer = TableWriterBuilder::from_reader(reader)
    ///     .build_with_dest(Cursor::new(Vec::<u8>::new()));
    ///
    /// // from_reader picked up the record definition,
    /// // so writing will work
    /// let writing_result = writer.write_records(&stations);
    /// assert_eq!(writing_result.is_ok(), true);
    /// ```
    pub fn from_reader<T: std::io::Read + std::io::Seek>(
        reader: crate::reading::Reader<T>,
    ) -> Self {
        Self::from_table_info(reader.into_table_info())
    }

    pub fn from_table_info(table_info: TableInfo) -> Self {
        let mut fields_info = table_info.fields_info;
        if let Some(i) = fields_info.first() {
            if i.is_deletion_flag() {
                fields_info.remove(0);
            }
        }
        let mut hdr = table_info.header;
        hdr.update_date();
        Self {
            v: fields_info,
            hdr,
        }
    }

    /// Adds a Character field to the record definition,
    /// the length is the maximum number of bytes (not chars) that fields can hold
    pub fn add_character_field(mut self, name: FieldName, length: u8) -> Self {
        self.v
            .push(FieldInfo::new(name, FieldType::Character, length));
        self
    }

    /// Adds a [Date](struct.Date.html) field
    pub fn add_date_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::Date,
            FieldType::Date.size().unwrap(),
        ));
        self
    }

    /// Adds a [Numeric](enum.FieldValue.html#variant.Numeric)
    pub fn add_numeric_field(mut self, name: FieldName, length: u8, num_decimals: u8) -> Self {
        let mut info = FieldInfo::new(name, FieldType::Numeric, length);
        info.num_decimal_places = num_decimals;
        self.v.push(info);
        self
    }

    /// Adds a [Float](enum.FieldValue.html#variant.Float)
    pub fn add_float_field(mut self, name: FieldName, length: u8, num_decimals: u8) -> Self {
        let mut info = FieldInfo::new(name, FieldType::Float, length);
        info.num_decimal_places = num_decimals;
        self.v.push(info);
        self
    }

    /// Adds a [Logical](enum.FieldValue.html#variant.Logical)
    pub fn add_logical_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::Logical,
            FieldType::Logical
                .size()
                .expect("Internal error Logical field date should be known"),
        ));
        self
    }

    /// Adds a [Integer](enum.FieldValue.html#variant.Integer)
    pub fn add_integer_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::Integer,
            FieldType::Integer
                .size()
                .expect("Internal error Integer field date should be known"),
        ));
        self.hdr.file_type = crate::header::Version::FoxPro2 {
            supports_memo: false,
        };
        self
    }

    /// Adds a [DateTime](enum.FieldValue.html#variant.DateTime)
    pub fn add_datetime_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::DateTime,
            FieldType::DateTime
                .size()
                .expect("Internal error datetime field date should be known"),
        ));
        self.hdr.file_type = crate::header::Version::FoxPro2 {
            supports_memo: false,
        };
        self
    }

    /// Adds a [Double](enum.FieldValue.html#variant.Double)
    pub fn add_double_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::Double,
            FieldType::Double
                .size()
                .expect("Internal error Double field date should be known"),
        ));
        self.hdr.file_type = crate::header::Version::FoxPro2 {
            supports_memo: false,
        };
        self
    }

    /// Adds a [Currency](enum.FieldValue.html#variant.Currency)
    pub fn add_currency_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(
            name,
            FieldType::Currency,
            FieldType::Currency
                .size()
                .expect("Internal error Currency field date should be known"),
        ));
        self.hdr.file_type = crate::header::Version::FoxPro2 {
            supports_memo: false,
        };
        self
    }
    /// Builds the writer and set the dst as where the file data will be written
    pub fn build_with_dest<W: Write + Seek>(self, dst: W) -> TableWriter<W> {
        TableWriter::new(dst, self.v, self.hdr)
    }

    /// Helper function to set create a file at the given path
    /// and make the writer write to the newly created file.
    ///
    /// This function wraps the `File` in a `BufWriter` to increase performance.
    pub fn build_with_file_dest<P: AsRef<Path>>(
        self,
        path: P,
    ) -> Result<TableWriter<BufWriter<File>>, Error> {
        let file = File::create(path)
            .map_err(|err| Error::io_error(err, 0))?;
        let dst = BufWriter::new(file);
        Ok(self.build_with_dest(dst))
    }

    pub fn build_table_info(self) -> TableInfo {
        TableInfo {
            header: self.hdr,
            fields_info: self.v,
        }
    }
}

impl Default for TableWriterBuilder {
    fn default() -> Self {
        Self {
            v: vec![],
            hdr: Header::new(0, 0, 0),
        }
    }
}

mod private {
    pub trait Sealed {}

    macro_rules! impl_sealed_for {
        ($type:ty) => {
            impl Sealed for $type {}
        };
    }

    impl_sealed_for!(bool);
    impl_sealed_for!(Option<bool>);
    impl_sealed_for!(std::string::String);
    impl_sealed_for!(Option<std::string::String>);
    impl_sealed_for!(&str);
    impl_sealed_for!(f64);
    impl_sealed_for!(f32);
    impl_sealed_for!(i32);
    impl_sealed_for!(Option<f64>);
    impl_sealed_for!(Option<f32>);
    impl_sealed_for!(crate::record::field::Date);
    impl_sealed_for!(Option<crate::record::field::Date>);
    impl_sealed_for!(crate::record::field::FieldValue);
    impl_sealed_for!(crate::record::field::DateTime);
}

/// Trait implemented by types we can write as dBase types
///
/// This trait is 'private' and cannot be implemented on your custom types.
pub trait WritableAsDbaseField: private::Sealed {
    fn write_as<W: Write>(&self, field_type: FieldType, dst: &mut W) -> Result<(), ErrorKind>;
}

/// Trait to be implemented by struct that you want to be able to write to (serialize)
/// a dBase file
pub trait WritableRecord {
    /// Use the FieldWriter to write the fields of the record
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldIOError>;
}

impl WritableRecord for Record {
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldIOError> {
        while let Some(name) = field_writer.next_field_name() {
            let value = self.get(name).ok_or_else(|| {
                FieldIOError::new(
                    ErrorKind::Message(format!(
                        "Could not find field named '{}' in the record map",
                        name
                    )),
                    None,
                )
            })?;
            field_writer.write_next_field_value(value)?;
        }
        Ok(())
    }
}

/// Struct that knows how to write a record
///
/// You give it the values you want to write and it writes them.
/// The order and type of value must match the one given when creating the
/// [TableWriter](struct.TableWriter.html), otherwise an error will occur.
pub struct FieldWriter<'a, W: Write> {
    pub(crate) dst: &'a mut W,
    pub(crate) fields_info: std::iter::Peekable<std::slice::Iter<'a, FieldInfo>>,
    pub(crate) buffer: &'a mut Cursor<Vec<u8>>,
}

impl<'a, W: Write> FieldWriter<'a, W> {
    /// Returns the name of the next field that is expected to be written
    pub fn next_field_name(&mut self) -> Option<&'a str> {
        self.fields_info.peek().map(|info| info.name.as_str())
    }

    /// Writes the given `field_value` to the record.
    ///
    /// # Notes
    ///
    /// If the corresponding `FieldType` of the the field_value type (`T`) does not
    /// match the expected type an error is returned.
    ///
    /// Values for which the number of bytes written would exceed the specified field_length
    /// (if it had to be specified) will be truncated
    ///
    /// Trying to write more values than was declared when creating the writer will cause
    /// an `EndOfRecord` error.
    pub fn write_next_field_value<T: WritableAsDbaseField>(
        &mut self,
        field_value: &T,
    ) -> Result<(), FieldIOError> {
        if let Some(field_info) = self.fields_info.next() {
            self.buffer.set_position(0);

            field_value
                .write_as(field_info.field_type, &mut self.buffer)
                .map_err(|kind| FieldIOError::new(kind, Some(field_info.clone())))?;

            let mut bytes_written = self.buffer.position();
            let mut bytes_to_pad = i64::from(field_info.field_length) - bytes_written as i64;
            if bytes_to_pad > 0 {
                if field_info.field_type == FieldType::Float
                    || field_info.field_type == FieldType::Numeric
                {
                    // Depending on the locale, the dot might not be the delimiter for floating point
                    // but we are not yet ready to handle correctly codepages, etc
                    let mut maybe_dot_pos = self.buffer.get_ref().iter().position(|b| *b == b'.');
                    if maybe_dot_pos.is_none() {
                        write!(self.buffer, ".").map_err(|error| {
                            FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                        })?;
                        bytes_written = self.buffer.position();
                        maybe_dot_pos = Some(bytes_written as usize)
                    }
                    let dot_pos = maybe_dot_pos.unwrap();
                    let missing_decimals =
                        field_info.num_decimal_places - (bytes_written - dot_pos as u64) as u8;
                    for _ in 0..missing_decimals {
                        write!(self.buffer, "0").map_err(|error| {
                            FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                        })?;
                    }
                    bytes_written = self.buffer.position();
                    bytes_to_pad = i64::from(field_info.field_length) - bytes_written as i64;
                }
                for _ in 0..bytes_to_pad {
                    write!(self.buffer, " ").map_err(|error| {
                        FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                    })?;
                }
                let field_bytes = self.buffer.get_ref();
                debug_assert_eq!(self.buffer.position(), field_info.field_length as u64);
                self.dst
                    .write_all(&field_bytes[..self.buffer.position() as usize])
                    .map_err(|error| {
                        FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                    })?;
            } else {
                // The current field value size exceeds the one one set
                // when creating the writer, we just crop
                let field_bytes = self.buffer.get_ref();
                debug_assert_eq!(self.buffer.position(), field_info.field_length as u64);
                self.dst
                    .write_all(&field_bytes[..field_info.field_length as usize])
                    .map_err(|error| {
                        FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                    })?;
            }
            Ok(())
        } else {
            Err(FieldIOError::new(ErrorKind::TooManyFields, None))
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn write_next_field_raw(&mut self, value: &[u8]) -> Result<(), FieldIOError> {
        if let Some(field_info) = self.fields_info.next() {
            if value.len() == field_info.field_length as usize {
                self.dst.write_all(value).map_err(|error| {
                    FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                })?;
            } else if value.len() < field_info.field_length as usize {
                self.dst.write_all(value).map_err(|error| {
                    FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                })?;
                for _ in 0..field_info.field_length - value.len() as u8 {
                    write!(self.dst, " ").map_err(|error| {
                        FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                    })?;
                }
            } else {
                self.dst
                    .write_all(&value[..field_info.field_length as usize])
                    .map_err(|error| {
                        FieldIOError::new(ErrorKind::IoError(error), Some(field_info.clone()))
                    })?;
            }
            Ok(())
        } else {
            Err(FieldIOError::new(ErrorKind::EndOfRecord, None))
        }
    }

    fn write_deletion_flag(&mut self) -> std::io::Result<()> {
        self.dst.write_u8(b' ')
    }

    fn all_fields_were_written(&mut self) -> bool {
        self.fields_info.peek().is_none()
    }
}


/// Structs that writes dBase records to a destination
///
/// The only way to create a TableWriter is to use its
/// [TableWriterBuilder](struct.TableWriterBuilder.html)
pub struct TableWriter<W: Write + Seek> {
    dst: W,
    fields_info: Vec<FieldInfo>,
    /// contains the header of the input file
    /// if this writer was created form a reader
    header: Header,
    /// Buffer used by the FieldWriter
    buffer: Cursor<Vec<u8>>,
    closed: bool,
}

impl<W: Write + Seek> TableWriter<W> {
    fn new(dst: W, fields_info: Vec<FieldInfo>, origin_header: Header) -> Self {
        Self {
            dst,
            fields_info,
            header: origin_header,
            buffer: Cursor::new(vec![0u8; 255]),
            closed: false,
        }
    }

    /// Writes a record the inner destination
    ///
    /// # Example
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// # fn main() -> Result<(), dbase::Error> {
    /// let mut writer = dbase::TableWriterBuilder::new()
    ///     .add_character_field(dbase::FieldName::try_from("First Name").unwrap(), 50)
    ///     .build_with_file_dest("records.dbf")?;
    ///
    /// let mut record = dbase::Record::default();
    /// record.insert("First Name".to_string(), dbase::FieldValue::Character(Some("Yoshi".to_string())));
    ///
    /// writer.write_record(&record)?;
    ///
    /// # let ignored_result = std::fs::remove_file("record.dbf");
    /// Ok(())
    /// # }
    /// ```
    pub fn write_record<R: WritableRecord>(&mut self, record: &R) -> Result<(), Error> {
        if self.header.num_records == 0 {
            // reserve the header
            self.write_header()?;
        }

        let mut field_writer = FieldWriter {
            dst: &mut self.dst,
            fields_info: self.fields_info.iter().peekable(),
            buffer: &mut self.buffer,
        };

        let current_record_num = self.header.num_records as usize;

        field_writer
            .write_deletion_flag()
            .map_err(|error| Error::io_error(error, current_record_num))?;

        record
            .write_using(&mut field_writer)
            .map_err(|error| Error::new(error, current_record_num))?;

        if !field_writer.all_fields_were_written() {
            return Err(Error {
                record_num: current_record_num,
                field: None,
                kind: ErrorKind::NotEnoughFields,
            });
        }

        self.header.num_records += 1;
        Ok(())
    }

    /// Writes the records to the inner destination
    ///
    /// Values for which the number of bytes written would exceed the specified field_length
    /// (if it had to be specified) will be truncated
    /// # Example
    /// ```
    /// use dbase::{TableWriterBuilder, FieldName, WritableRecord, FieldWriter, ErrorKind, FieldIOError};
    /// use std::convert::TryFrom;
    /// use std::io::{Cursor, Write};
    ///
    /// struct User {
    ///     first_name: String,
    /// }
    ///
    /// impl WritableRecord for User {
    ///     fn write_using<'a, W: Write>(&self,field_writer: &mut FieldWriter<'a, W>) -> Result<(), FieldIOError> {
    ///         field_writer.write_next_field_value(&self.first_name)
    ///     }
    /// }
    ///
    /// let mut cursor = Cursor::new(Vec::<u8>::new());
    /// let writer = TableWriterBuilder::new()
    ///     .add_character_field(FieldName::try_from("First Name").unwrap(), 50)
    ///     .build_with_dest(&mut cursor);
    ///
    /// let records = vec![
    ///     User {
    ///         first_name: "Yoshi".to_owned(),
    ///     }
    /// ];
    /// writer.write_records(&records).unwrap();
    /// assert_eq!(cursor.position(), 117)
    /// ```
    pub fn write_records<'a, R: WritableRecord + 'a, C: IntoIterator<Item=&'a R>>(mut self, records: C) -> Result<(), Error> {
        for record in records.into_iter() {
            self.write_record(record)?;
        }
        Ok(())
    }

    /// Close the writer
    ///
    /// Automatically closed when the writer is dropped,
    /// use it if you want to handle error that can happen when the writer is closing
    ///
    /// Calling close on an already closed writer is a no-op
    fn close(&mut self) -> Result<(), Error> {
        if !self.closed {
            self.dst.seek(SeekFrom::Start(0))
                .map_err(|error| Error::io_error(error, self.header.num_records as usize))?;
            self.update_header();
            self.write_header()?;
            self.dst.seek(SeekFrom::End(0))
                .map_err(|error| Error::io_error(error, self.header.num_records as usize))?;
            self.dst
                .write_u8(FILE_TERMINATOR)
                .map_err(|error| Error::io_error(error, self.header.num_records as usize))?;
            self.closed = true;
        }
        Ok(())
    }

    fn update_header(&mut self) {
        let offset_to_first_record =
            Header::SIZE + (self.fields_info.len() * FieldInfo::SIZE) + std::mem::size_of::<u8>();
        let size_of_record = self
            .fields_info
            .iter()
            .fold(0u16, |s, ref info| s + info.field_length as u16);

        self.header.offset_to_first_record = offset_to_first_record as u16;
        self.header.size_of_record = size_of_record;
    }

    fn write_header(&mut self) -> Result<(), Error> {
        self.header
            .write_to(&mut self.dst)
            .map_err(|error| Error::io_error(error, 0))?;
        for record_info in &self.fields_info {
            record_info
                .write_to(&mut self.dst)
                .map_err(|error| Error::io_error(error, 0))?;
        }
        self.dst
            .write_u8(TERMINATOR_VALUE)
            .map_err(|error| Error::io_error(error, 0))
    }
}

impl<T: Write + Seek> Drop for TableWriter<T> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

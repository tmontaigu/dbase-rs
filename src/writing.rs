//! Module with all structs & functions charged of writing .dbf file content
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

use byteorder::WriteBytesExt;

use {Error, Record};
use header::Header;
use reading::TERMINATOR_VALUE;
use record::{FieldInfo, FieldName, field::FieldType};


/// A dbase file ends with this byte
const FILE_TERMINATOR: u8 = 0x1A;

/// Builder to be used to create a [TableWriter](struct.TableWriter).
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
    hdr: Option<Header>,
}

impl TableWriterBuilder {
    pub fn new() -> Self {
        Self {
            v: vec![],
            hdr: None
        }
    }

    pub fn from_reader<T: std::io::Read + std::io::Seek>(reader: crate::reading::Reader<T>) -> Self {
        let mut fields_info = reader.fields_info;
        if let Some(i) = fields_info.first() {
            if i.is_deletion_flag() {
                fields_info.remove(0);
            }
        }
        Self {
            v: fields_info,
            hdr: Some(reader.header)
        }
    }

    /// Adds a Character field to the record definition,
    /// the length is the maximum number of bytes (not chars) that fields can hold
    pub fn add_character_field(mut self, name: FieldName, length: u8) -> Self {
        self.v.push(FieldInfo::new(name, FieldType::Character, length));
        self
    }

    /// Adds a [Date](struct.Date.html) field
    pub fn add_date_field(mut self, name: FieldName) -> Self {
        self.v.push(FieldInfo::new(name, FieldType::Date, FieldType::Date.size().unwrap()));
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

    /// Adds a [Logicak](enum.FieldValue.html#variant.Logical)
    pub fn add_logical_field(mut self, name: FieldName) -> Self {
        self.v.push(
            FieldInfo::new(
                name,
                FieldType::Logical,
                FieldType::Logical
                    .size()
                    .expect("Internal error Logical field date should be known"))
        );
        self
    }

    /// Builds the writer and set the dst as where the file data will be written
    pub fn build_with_dest<W: Write>(self, dst: W) -> TableWriter<W> {
        TableWriter::new(dst, self.v, self.hdr)
    }

    /// Helper function to set create a file at the given path
    /// and make the writer write to the newly created file.
    ///
    /// This function wraps the `File` in a `BufWriter` to increase performance.
    pub fn build_with_file_dest<P: AsRef<Path>>(self, path: P) -> std::io::Result<TableWriter<BufWriter<File>>> {
        let dst = BufWriter::new(File::create(path)?);
        Ok(self.build_with_dest(dst))
    }
}

mod private {
    pub trait Sealed {}

    macro_rules! impl_sealed_for {
        ($type:ty)  => {
            impl Sealed for $type {}
        }
    }

    impl_sealed_for!(bool);
    impl_sealed_for!(Option<bool>);
    impl_sealed_for!(std::string::String);
    impl_sealed_for!(Option<std::string::String>);
    impl_sealed_for!(&str);
    impl_sealed_for!(f64);
    impl_sealed_for!(f32);
    impl_sealed_for!(Option<f64>);
    impl_sealed_for!(Option<f32>);
    impl_sealed_for!(crate::record::field::Date);
    impl_sealed_for!(Option<crate::record::field::Date>);
    impl_sealed_for!(crate::record::field::FieldValue);
}

/// Trait implemented by all types that we know how to write in a dBase file
pub trait WritableDbaseField: private::Sealed {
    fn field_type(&self) -> FieldType;
    fn write_to<W: Write>(&self, dst: &mut W) -> std::io::Result<()>;
}

/// Trait to be implemented by struct to you want to be able to write to (serialize)
/// to a dBase file
pub trait WritableRecord {
    /// Use the FieldWriter to write the fields of the record
    fn write_using<'a, W: Write>(&self, field_writer: &mut FieldWriter<'a, W>) -> Result<(), Error>;
}

impl WritableRecord for Record {
    fn write_using<'a, W: Write>(&self, field_writer: &mut FieldWriter<'a, W>) -> Result<(), Error> {
        while let Some(name ) = field_writer.next_field_name() {
            let value = self.get(name)
                .ok_or_else(||
                    Error::Message(format!("Could not find field named '{}' in the record map", name)))?;
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
    pub(crate) buffer: Cursor<Vec<u8>>,
}

impl<'a, W: Write> FieldWriter<'a, W> {
    /// Returns the name of the field that is expected to be written
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
    /// Values for witch the number of bytes written would exceed the specified field_length
    /// (if it had to be specified) will be truncated
    ///
    /// Trying to write more values than was declared when creating the writer will cause
    /// an `EndOfRecord` error.
    pub fn write_next_field_value<T: WritableDbaseField>(&mut self, field_value: &T) -> Result<(), Error> {
        if let Some(field_info) = self.fields_info.next() {
            self.buffer.set_position(0);
            if field_value.field_type() != field_info.field_type {
                return Err(Error::BadFieldType {
                    expected: field_info.field_type,
                    got: field_value.field_type(),
                    field_name: field_info.name.to_owned()
                });
            }

            field_value.write_to(&mut self.buffer)?;

            let mut bytes_written = self.buffer.position();
            let mut bytes_to_pad = i64::from(field_info.field_length) - bytes_written as i64;
            if bytes_to_pad > 0 {
                if field_info.field_type == FieldType::Float ||
                    field_info.field_type == FieldType::Numeric {
                    // Depending on the locale, the dot might not be the delimiter for floating point
                    // but we are not yet ready to handle correctly codepages, etc
                    let mut maybe_dot_pos = self.buffer.get_ref().iter().position(|b| *b == '.' as u8);
                    if maybe_dot_pos.is_none() {
                        write!(self.buffer, ".")?;
                        bytes_written = self.buffer.position();
                        maybe_dot_pos = Some(bytes_written as usize)
                    }
                    let dot_pos = maybe_dot_pos.unwrap();
                    let missing_decimals = field_info.num_decimal_places - (bytes_written - dot_pos as u64) as u8;
                    for _ in 0..missing_decimals {
                        write!(self.buffer, "0")?;
                    }
                    bytes_written = self.buffer.position();
                    bytes_to_pad = i64::from(field_info.field_length) - bytes_written as i64;
                }
                for _ in 0..bytes_to_pad {
                    write!(self.buffer, " ")?;
                }
                let field_bytes = self.buffer.get_ref();
                debug_assert_eq!(self.buffer.position(), field_info.field_length as u64);
                self.dst.write_all(&field_bytes[..self.buffer.position() as usize])?;
            } else {
                // The current field value size exceeds the one one set
                // when creating the writer, we just crop
                let field_bytes = self.buffer.get_ref();
                debug_assert_eq!(self.buffer.position(), field_info.field_length as u64);
                self.dst.write_all(&field_bytes[..field_info.field_length as usize])?;
            }
            Ok(())
        } else {
            Err(Error::EndOfRecord)
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn write_next_field_raw(&mut self, value: &[u8]) -> Result<(), Error> {
        if let Some(field_info) = self.fields_info.next() {
            if value.len() == field_info.field_length as usize {
                self.dst.write_all(value)?;
            } else if value.len() < field_info.field_length as usize {
                self.dst.write_all(value)?;
                for _ in 0..field_info.field_length - value.len() as u8 {
                    write!(self.dst, " ")?;
                }
            } else {
                self.dst.write_all(&value[..field_info.field_length as usize])?;
            }
            Ok(())
        } else {
            Err(Error::EndOfRecord)
        }
    }

    fn write_deletion_flag(&mut self) -> std::io::Result<()> {
        self.dst.write_u8(' ' as u8)
    }

    fn all_fields_were_written(&mut self) -> bool {
        self.fields_info.peek().is_none()
    }

}

/// Structs that writes dBase records to a destination
///
/// The only way to create a TableWriter is to use its
/// [TableWriterBuilder](struct.TableWriterBuilder.html)
pub struct TableWriter<W: Write> {
    dst: W,
    fields_info: Vec<FieldInfo>,
    /// contains the header of the input file
    /// if this writer was created form a reader
    origin_header: Option<Header>,
}

impl<W: Write> TableWriter<W> {
    fn new(dst: W, fields_info: Vec<FieldInfo>, origin_header: Option<Header>) -> Self {
        Self {
            dst,
            fields_info,
            origin_header
        }
    }

    /// Writes the records to the inner destination
    /// and returns it once finished
    ///
    /// # Example
    /// ```
    /// use dbase::{TableWriterBuilder, FieldName, WritableRecord, Error, FieldWriter};
    /// use std::convert::TryFrom;
    /// use std::io::{Cursor, Write};
    ///
    /// struct User {
    ///     first_name: String,
    /// }
    ///
    /// impl WritableRecord for User {
    ///     fn write_using<'a, W: Write>(&self,field_writer: &mut FieldWriter<'a, W>) -> Result<(), Error> {
    ///         field_writer.write_next_field_value(&self.first_name)
    ///     }
    /// }
    ///
    /// let writer = TableWriterBuilder::new()
    ///     .add_character_field(FieldName::try_from("First Name").unwrap(), 50)
    ///     .build_with_dest(Cursor::new(Vec::<u8>::new()));
    ///
    /// let records = vec![
    ///     User {
    ///         first_name: "Yoshi".to_owned(),
    ///     }
    /// ];
    /// let cursor = writer.write(&records).unwrap();
    /// assert_eq!(cursor.position(), 117)
    /// ```
    pub fn write<R: WritableRecord>(mut self, records: &Vec<R>) -> Result<W, Error> {
        let header = self.build_header(records.len());
        header.write_to(&mut self.dst)?;
        for record_info in &self.fields_info {
            record_info.write_to(&mut self.dst)?;
        }
        self.dst.write_u8(TERMINATOR_VALUE)?;

        let mut field_writer = FieldWriter {
            dst: &mut self.dst,
            fields_info: self.fields_info.iter().peekable(),
            buffer: Cursor::new(vec![0u8; 255])
        };

        for record in records {
            if header.file_type.is_dbase() {
                field_writer.write_deletion_flag()?;
            }
            record.write_using(&mut field_writer)?;
            if !field_writer.all_fields_were_written() {
                return Err(Error::NotEnoughFields);
            }
            field_writer.fields_info = self.fields_info.iter().peekable();
        }

        self.dst.write_u8(FILE_TERMINATOR)?;
        Ok(self.dst)
    }

    fn build_header(&self, num_records: usize) -> Header {
        let offset_to_first_record =
            Header::SIZE + (self.fields_info.len() * FieldInfo::SIZE) + std::mem::size_of::<u8>();
        let size_of_record = self.fields_info
            .iter()
            .fold(0u16, |s, ref info| s + info.field_length as u16);

        let mut header = Header::new(
            num_records as u32,
            offset_to_first_record as u16,
            size_of_record,
        );

        if let Some(ref hdr) = self.origin_header {
            header.code_page_mark = hdr.code_page_mark;
            header.file_type = hdr.file_type;
        }
        header
    }
}


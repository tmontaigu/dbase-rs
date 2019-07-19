//! Module with all structs & functions charged of writing .dbf file content
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use byteorder::WriteBytesExt;


use header::Header;
use reading::TERMINATOR_VALUE;
use record::RecordFieldInfo;
use {Error, Record};
/// A dbase file ends with this byte
const FILE_TERMINATOR: u8 = 0x1A;

/// Struct that handles the writing of records to any destination
/// that supports the `Write` trait
pub struct Writer<T: Write> {
    dest: T,
}


impl<T: Write> Writer<T> {
    /// Creates a new Writer
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// let writer = dbase::Writer::new(Cursor::new(Vec::<u8>::new()));
    /// ```
    pub fn new(dest: T) -> Self {
        Self { dest }
    }

    /// Writes the collection of records
    ///
    /// # Returns
    /// Returns the `dest` provided when constructing the writer, in case you need it.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    ///
    /// let mut fst = dbase::Record::new();
    /// fst.insert("Name".to_string(), dbase::FieldValue::from("Fallujah"));
    /// let records = vec![fst];
    ///
    /// let writer = dbase::Writer::new(Cursor::new(Vec::<u8>::new()));
    /// let cursor = writer.write(&records).unwrap();
    /// ```
    pub fn write(mut self, records: &Vec<Record>) -> Result<(T), Error> {
        if records.is_empty() {
            return Ok(self.dest);
        }
        let fields_name: Vec<&String> = records[0].keys().collect();

        let mut fields_info = Vec::<RecordFieldInfo>::with_capacity(fields_name.len());
        for (field_name, field_value) in &records[0] {
            let field_length = field_value.size_in_bytes();
            if field_length > std::u8::MAX as usize {
                return Err(Error::FieldLengthTooLong);
            }

            fields_info.push(RecordFieldInfo::new(
                field_name.to_owned(),
                field_value.field_type(),
                field_length as u8,
            ));
        }

        // TODO check that for the same field, the field type is the same
        for record in &records[1..records.len()] {
            for (field_name, record_info) in fields_name.iter().zip(&mut fields_info) {
                let field_value = record.get(*field_name).unwrap(); // TODO: Should return an Err()
                let field_length = field_value.size_in_bytes();
                if field_length > std::u8::MAX as usize {
                    return Err(Error::FieldLengthTooLong);
                }
                record_info.field_length =
                    std::cmp::max(record_info.field_length, field_length as u8);
            }
        }

        let offset_to_first_record =
            Header::SIZE + (fields_info.len() * RecordFieldInfo::SIZE) + std::mem::size_of::<u8>();
        let size_of_record = fields_info
            .iter()
            .fold(0u16, |s, ref info| s + info.field_length as u16);
        let hdr = Header::new(
            records.len() as u32,
            offset_to_first_record as u16,
            size_of_record,
        );

        hdr.write_to(&mut self.dest)?;
        for record_info in &fields_info {
            record_info.write_to(&mut self.dest)?;
        }

        self.dest.write_u8(TERMINATOR_VALUE)?;

        let value_buffer = [' ' as u8; std::u8::MAX as usize];
        for record in records {
            self.dest.write_u8(' ' as u8)?; // DeletionFlag
            for (field_name, record_info) in fields_name.iter().zip(&fields_info) {
                let value = record.get(*field_name).unwrap();
                let bytes_written = value.write_to(&mut self.dest)? as u8;
                if bytes_written > record_info.field_length {
                    panic!("record length was miscalculated");
                }

                let bytes_to_pad = record_info.field_length - bytes_written;
                self.dest
                    .write_all(&value_buffer[0..bytes_to_pad as usize])?;
            }
        }
        self.dest.write_u8(FILE_TERMINATOR)?;
        Ok(self.dest)
    }
}

impl Writer<BufWriter<File>> {
    /// Creates a new writer that will write the to a new filed
    /// # Examples
    /// ```
    /// let writer = dbase::Writer::from_path("new_records.dbf").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        Ok(Writer::new(BufWriter::new(File::create(path)?)))
    }
}

/// Writes the records to the dest
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// let mut fst = dbase::Record::new();
/// fst.insert("Name".to_string(), dbase::FieldValue::from("The Flesh Prevails"));
/// fst.insert("Price".to_string(), dbase::FieldValue::Numeric(Some(9.99)));
/// let records = vec![fst];
///
/// let cursor = Cursor::new(Vec::<u8>::new());
/// let cursor = dbase::write_to(&records, cursor).unwrap();
/// ```
pub fn write_to<T: Write>(records: &Vec<Record>, dest: T) -> Result<T, Error> {
    let writer = Writer::new(dest);
    writer.write(&records)
}

/// Writes all the records to the a new file at path
///
/// # Examples
///
/// ```
/// let mut fst = dbase::Record::new();
/// fst.insert("Name".to_string(), dbase::FieldValue::from("The Flesh Prevails"));
/// fst.insert("Price".to_string(), dbase::FieldValue::Numeric(Some(9.99)));
/// let records = vec![fst];
///
/// dbase::write_to_path(&records, "albums.dbf").unwrap();
/// ```
pub fn write_to_path<P: AsRef<Path>>(records: &Vec<Record>, path: P) -> Result<(), Error> {
    let writer = Writer::from_path(path)?;
    writer.write(&records)?;
    Ok(())
}

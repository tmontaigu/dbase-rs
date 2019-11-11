//! Module with all structs & functions charged of writing .dbf file content
use std::io::{Cursor, Write, BufWriter};

use byteorder::WriteBytesExt;

use {Error, Record};
use crate::record::field::{FieldValue};
use header::Header;
use reading::TERMINATOR_VALUE;
use record::field::FieldType;
use record::RecordFieldInfo;
use std::path::Path;
use std::fs::File;

/// A dbase file ends with this byte
const FILE_TERMINATOR: u8 = 0x1A;

pub struct FieldName(String);

impl FieldName {
    pub fn new(name: String) -> Result<Self, &'static str> {
        if name.as_bytes().len() > 11 {
            Ok(Self { 0: name })
        } else {
            Err("FieldName byte representation cannot exceed 11 bytes")
        }
    }
}

pub struct TableWriterBuilder {
    v: Vec<RecordFieldInfo>
}

//TODO check len of name, create add method for other types
impl TableWriterBuilder {
    pub fn new() -> Self {
        Self {
            v: vec![]
        }
    }

    //TODO the header should/must be reused
    pub fn from_reader<T: std::io::Read + std::io::Seek>(reader: crate::reading::Reader<T>) -> Self {
        let mut fields_info = reader.fields_info;
        if let Some(i) = fields_info.first() {
            if i.is_deletion_flag() {
                fields_info.remove(0);
            }
        }
        Self {
            v: fields_info
        }
    }

    pub fn add_character_field(mut self, name: String, length: u8) -> Self {
        self.v.push(RecordFieldInfo::new(name, FieldType::Character, length));
        self
    }

    pub fn add_date_field(mut self, name: String) -> Self {
        self.v.push(RecordFieldInfo::new(name, FieldType::Date, FieldType::Date.size().unwrap()));
        self
    }

    //TODO num decimal places
    pub fn add_numeric_field(mut self, name: String, length: u8) -> Self {
        self.v.push(RecordFieldInfo::new(name, FieldType::Numeric, length));
        self
    }

    pub fn add_float_field(mut self, name: String, length: u8) -> Self {
        self.v.push(RecordFieldInfo::new(name, FieldType::Float, length));
        self
    }

    pub fn build_with_dest<W: Write>(self, dst: W) -> TableWriter<W> {
        TableWriter::new(dst, self.v)
    }

    pub fn build_with_file_dest<P: AsRef<Path>>(self, path: P) -> std::io::Result<TableWriter<BufWriter<File>>> {
        let dst = BufWriter::new(File::create(path)?);
        Ok(self.build_with_dest(dst))
    }
}

pub trait WritableRecord {
    fn values_for_fields(self, field_names: &[&str], values: &mut Vec<FieldValue>);
}

impl WritableRecord for Record {
    fn values_for_fields(mut self, field_names: &[&str], values: &mut Vec<FieldValue>) {
        for name in field_names {
            values.push(self
                .remove(*name)
                .expect(&format!("Expected field with name '{}' to be in the record", name)));
        }
    }
}

pub struct TableWriter<W: Write> {
    dst: W,
    fields_info: Vec<RecordFieldInfo>,
    buffer: Cursor<Vec<u8>>,
    fields_values: Vec<FieldValue>,
}

//TODO the header written should be constructed better
// (choose the right vesion depending on fields ,etc)
impl<W: Write> TableWriter<W> {
    fn new(dst: W, fields_info: Vec<RecordFieldInfo>) -> Self {
        let biggest_field = fields_info
            .iter()
            .map(|info| info.field_length)
            .max()
            .unwrap();
        let buffer = Cursor::new(vec![0u8; biggest_field as usize]);
        let fields_values = Vec::<FieldValue>::with_capacity(fields_info.len() as usize);
        Self {
            dst,
            fields_info,
            buffer,
            fields_values,
        }
    }

    pub fn write<R: WritableRecord>(mut self, records: Vec<R>) -> Result<W, Error> {
        let offset_to_first_record =
            dbg!(Header::SIZE + (self.fields_info.len() * RecordFieldInfo::SIZE) + std::mem::size_of::<u8>());
        let size_of_record = self.fields_info
            .iter()
            .fold(0u16, |s, ref info| s + info.field_length as u16);
        let header = Header::new(
            records.len() as u32,
            offset_to_first_record as u16,
            size_of_record,
        );

        header.write_to(&mut self.dst)?;
        for record_info in &self.fields_info {
            record_info.write_to(&mut self.dst)?;
        }
        self.dst.write_u8(TERMINATOR_VALUE)?;

        let mut field_names = Vec::<&str>::with_capacity(self.fields_info.len());
        for field_info in &self.fields_info {
            field_names.push(&field_info.name);
        }

        let pad_buf = [' ' as u8; std::u8::MAX as usize];
        for record in records {
            self.dst.write_u8(' ' as u8)?; // DeletionFlag
            record.values_for_fields(&field_names, &mut self.fields_values);
            println!("Will Write: {:?}", self.fields_values);
            if self.fields_values.len() != self.fields_info.len() {
                panic!("Number of fields_value given does no match what was expected, got {} expected: {}", self.fields_info.len(), self.fields_values.len());
            }
            for (field_value, field_info) in self.fields_values.drain(..self.fields_info.len()).zip(&self.fields_info) {
                self.buffer.set_position(0);
                if field_value.field_type() != field_info.field_type {
                    panic!("FieldType for field '{}' is expected to be '{:?}', but we were given a '{:?}'",
                           field_info.name, field_info.field_type, field_value.field_type());
                }

                field_value.write_to(&mut self.buffer)?;
                let field_bytes = self.buffer.get_ref();
                let bytes_written = self.buffer.position();
                let bytes_to_pad = i64::from(field_info.field_length) - bytes_written as i64;
                println!("bytes_written {}, field length {}, bytes tp pad {}", bytes_written, field_info.field_length, bytes_to_pad);
                if bytes_to_pad > 0 {
                    self.dst.write_all(&field_bytes[..bytes_written as usize])?;
                    self.dst.write_all(&pad_buf[..bytes_to_pad as usize])?;
                } else {
                    // The current field value size exceeds the one one set
                    // when creating the writer, we just crop
                    self.dst.write_all(&field_bytes[..field_info.field_length as usize])?;
                }
            }
            self.fields_values.clear();
        }
        self.dst.write_u8(FILE_TERMINATOR)?;
        Ok(self.dst)
    }
}


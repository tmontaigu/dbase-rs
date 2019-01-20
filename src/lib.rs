use std::io::{Seek, SeekFrom, Read};
use std::fs::File;

extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt};

use std::collections::HashMap;
use std::path::Path;
use std::ffi::CString;

type Record = HashMap<String, FieldValue>;

#[derive(Debug)]
pub enum Error {
    IoError (std::io::Error),
    ParseFloatError(std::num::ParseFloatError),
    InvalidFieldType(char),

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

pub struct Header {
    num_records: u32,
    offset_to_first_record: u16,
    size_of_record: u16,
}

impl Header {
    pub const SIZE: usize = 32;
    fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let mut skip = [0u8; 4];
        source.read_exact(&mut skip)?; //level + last date
        let num_records = source.read_u32::<LittleEndian>()?;
        let offset_to_first_record = source.read_u16::<LittleEndian>()?;
        let size_of_record = source.read_u16::<LittleEndian>()?;
        let mut skip = [0u8; 20];
        source.read_exact(&mut skip)?; //level + last date
        Ok(Self{
            num_records,
            offset_to_first_record,
            size_of_record
        })
    }
}

#[derive(Debug)]
pub enum FieldType {
    Character,
    Currency,
    Numeric,
    Float,
    Date,
    DateTime,
    Double,
    Integer,
    Logical,
    Memo,
    General,
    BinaryCharacter,
    BinaryMemo,
    Picture,
    Varbinary,
    BinaryVarchar,
}

impl FieldType {
    fn from(c: char) -> Option<FieldType> {
        match c {
            'C' => Some(FieldType::Character),
            'Y' => Some(FieldType::Currency),
            'N' => Some(FieldType::Numeric),
            'F' => Some(FieldType::Float),
            'D' => Some(FieldType::Date),
            'T' => Some(FieldType::DateTime),
            'B' => Some(FieldType::Double),
            'I' => Some(FieldType::Integer),
            'L' => Some(FieldType::Logical),
            'M' => Some(FieldType::Memo),
            'G' => Some(FieldType::General),
            //'C' => Some(FieldType::BinaryCharacter), ??
            //'M' => Some(FieldType::BinaryMemo),
            _  => None,
        }
    }

    fn try_from(c: char) -> Result<FieldType, Error> {
        match Self::from(c) {
            Some(t) => Ok(t),
            None => Err(Error::InvalidFieldType(c))
        }
    }
}


#[derive(Debug, PartialEq)]
pub enum FieldValue {
    Character(String),
    Numeric(f64), //Stored as String
    Logical(bool), // Stored as one char
    Integer(i32),
    Float(f32),
    Double(f64),
}

impl FieldValue {
    fn read_from<T: Read>(mut source: &mut T, field_info: &RecordFieldInfo) -> Result<Self, Error> {
        let value = match field_info.field_type {
            FieldType::Logical => {
                match source.read_u8()? as char {
                    '1' | 'T' | 't' | 'Y' | 'y' => FieldValue::Logical(true),
                    _ => FieldValue::Logical(false),
                }
            },
            FieldType::Integer => {
                FieldValue::Integer(source.read_i32::<LittleEndian>()?)
            },
            FieldType::Character => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Character(value.trim().to_owned())
            }
            FieldType::Numeric => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Numeric(value.trim().parse::<f64>()?)
            },
            FieldType::Float => FieldValue::Float(source.read_f32::<LittleEndian>()?),
            FieldType::Double => FieldValue::Double(source.read_f64::<LittleEndian>()?),
            _ => panic!("unhandled type")
        };
        Ok(value)
    }
}

pub struct RecordFieldInfo {
    name: String,
    field_type: FieldType,
    record_length: u8,
    num_decimal_places: u8,
}


impl RecordFieldInfo {
    pub const SIZE: usize = 32;

    fn read_from<T: Read>(source: &mut T) -> Result<Self, Error> {
        let mut name = [0u8; 11];
        source.read_exact(&mut name)?;
        let field_type = source.read_u8()?;

        let mut displacement_field = [0u8; 4];
        source.read_exact(&mut displacement_field)?;

        let record_length = source.read_u8()?;
        let num_decimal_places = source.read_u8()?;

        let mut skip = [0u8; 14];
        source.read_exact(&mut skip)?;

        let s = String::from_utf8_lossy(&name).trim_matches(|c| c == '\u{0}').to_owned();
        let field_type = FieldType::try_from(field_type as char)?;
        Ok(Self{
            name: s,
            field_type,
            record_length,
            num_decimal_places
        })
    }

    fn new_deletion_flag() -> Self {
        Self{
            name: "DeletionFlag".to_string(),
            field_type: FieldType::Character,
            record_length: 1,
            num_decimal_places: 0
        }
    }
}

fn read_string_of_len<T: Read>(source: &mut T, len: u8) -> Result<String, std::io::Error> {
    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0u8);
    source.read_exact(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub struct Reader<T: Read> {
    source: T,
    header: Header,
    fields_info: Vec<RecordFieldInfo>,
    current_record: u32,
}

impl<T: Read> Reader<T> {
    fn new(mut source: T) -> Result<Self, Error> {
        let header = Header::read_from(&mut source)?;
        let num_fields = (header.offset_to_first_record as usize - Header::SIZE) / RecordFieldInfo::SIZE;

        let mut fields_info = Vec::<RecordFieldInfo>::with_capacity(num_fields as usize + 1);
        fields_info.push(RecordFieldInfo::new_deletion_flag());
        for i in 0..num_fields {
            let info = RecordFieldInfo::read_from(&mut source)?;
            //println!("{} -> {}, {:?}, length: {}", i, info.name, info.field_type, info.record_length);
            fields_info.push(info);
        }

        let terminator = source.read_u8()? as char;
        if terminator != '\r' {
            panic!("unexpected terminator");
        }

        Ok(Self{
            source,
            header,
            fields_info,
            current_record: 0
        })
    }

    fn read(self) -> Result<Vec<Record>, Error> {
        let mut records = Vec::<Record>::with_capacity(self.fields_info.len());
        for record in self {
            records.push(record?);
        }
        //let file_end = self.source.read_u16::<LittleEndian>()?;
        Ok(records)
    }
}


impl<T: Read> Iterator for Reader<T>{
    type Item = Result<Record, Error>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.current_record >= self.header.num_records {
            None
        }
        else {
            let mut record = Record::with_capacity(self.fields_info.len() as usize);
            for field_info in &self.fields_info {
                let value = match FieldValue::read_from(&mut self.source, field_info) {
                    Err(e) => return Some(Err(e)),
                    Ok(value) => value,
                };

                if field_info.name != "DeletionFlag" {
                    record.insert(field_info.name.clone(), value);
                }
            }
            self.current_record += 1;
            Some(Ok(record))
        }
    }
}



pub fn read<P: AsRef<Path>>(path: P) -> Result<Vec<Record>, Error> {
    let f = File::open(path)?;
    let reader = Reader::new(f)?;
    reader.read()
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {

    }
}

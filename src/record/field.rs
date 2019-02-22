use std::io::{Read, Write};
use std::str::FromStr;
use std::fmt;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use record::RecordFieldInfo;
use Error;


#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum FieldType {
    Character = 'C' as isize,
    Currency,
    Numeric = 'N' as isize,
    Float = 'F' as isize,
    Date,
    DateTime,
    Double,
    Integer,
    Logical,
    //Memo,
    //General,
    //BinaryCharacter,
    //BinaryMemo,
    //Picture,
    //Varbinary,
    //BinaryVarchar,
}

impl FieldType {
    pub fn from(c: char) -> Option<FieldType> {
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
            //'M' => Some(FieldType::Memo),
            //'G' => Some(FieldType::General),
            //'C' => Some(FieldType::BinaryCharacter), ??
            //'M' => Some(FieldType::BinaryMemo),
            _ => None,
        }
    }

    pub fn try_from(c: char) -> Result<FieldType, Error> {
        match Self::from(c) {
            Some(t) => Ok(t),
            None => Err(Error::InvalidFieldType(c))
        }
    }
}


#[derive(Debug, PartialEq)]
pub struct Date {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub(crate) fn from_bytes(bytes: [u8; 3]) -> Self {
        Self {
            year: 1900u32 + bytes[0] as u32,
            month: bytes[1] as u32,
            day: bytes[2] as u32,
        }
    }

    pub(crate) fn write_to<T: Write>(&self, dest: &mut T) -> Result<(), Error> {
        self.validate()?;
        dest.write_u8((self.year - 1900) as u8)?;
        dest.write_u8(self.month as u8)?;
        dest.write_u8(self.day as u8)?;
        Ok(())
    }

    // Does some extremely basic checks
    fn validate(&self) -> Result<(), Error> {
        if self.month > 12 ||
           self.day > 31 ||
           self.year < 1900 ||
           self.year > 2155 {
               Err(Error::InvalidDate)
           }
        else {
            Ok(())
        }
    }
}


impl FromStr for Date {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let year = s[0..4].parse::<u32>()?;
        let month = s[4..6].parse::<u32>()?;
        let day = s[6..8].parse::<u32>()?;

        Ok(Self {
            year,
            month,
            day,
        })
    }
}


/// Enum where each variant stores the record value
#[derive(Debug, PartialEq)]
pub enum FieldValue {
    Character(String),
    //Stored as String
    Numeric(f64),
    // Stored as one char
    Logical(bool),
    Integer(i32),
    Float(f32),
    Double(f64),
    Date(Date),
}

impl FieldValue {
    pub(crate) fn read_from<T: Read>(mut source: &mut T, field_info: &RecordFieldInfo) -> Result<Self, Error> {
        let value = match field_info.field_type {
            FieldType::Logical => {
                match source.read_u8()? as char {
                    '1' | 'T' | 't' | 'Y' | 'y' => FieldValue::Logical(true),
                    _ => FieldValue::Logical(false),
                }
            }
            FieldType::Integer => {
                FieldValue::Integer(source.read_i32::<LittleEndian>()?)
            }
            FieldType::Character => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Character(value.trim().to_owned())
            }
            FieldType::Numeric => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Numeric(value.trim().parse::<f64>()?)
            }
            FieldType::Float => FieldValue::Float(source.read_f32::<LittleEndian>()?),
            FieldType::Double => FieldValue::Double(source.read_f64::<LittleEndian>()?),
            FieldType::Date => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Date(value.parse::<Date>()?)
            }
            _ => panic!("unhandled type")
        };
        Ok(value)
    }

    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::Character(_) => FieldType::Character,
            FieldValue::Numeric(_) => FieldType::Numeric,
            FieldValue::Logical(_) => FieldType::Logical,
            FieldValue::Integer(_) => FieldType::Integer,
            FieldValue::Float(_) => FieldType::Float,
            FieldValue::Double(_) => FieldType::Double,
            FieldValue::Date(_) => FieldType::Date,
        }
    }

    pub(crate) fn size_in_bytes(&self)-> usize {
        match self {
            FieldValue::Character(s) => {
                let str_bytes: &[u8] = s.as_ref();
                str_bytes.len()
            },
            FieldValue::Numeric(n) => {
                let s = n.to_string();
                s.len()
            },
            FieldValue::Logical(_) => 1,

            _ => unimplemented!(),
        }
    }

    pub(crate) fn write_to<T: Write>(&self, mut dest: T) -> Result<usize, std::io::Error> {
        match self {
            FieldValue::Character(s) => {
                let bytes = s.as_bytes();
                dest.write_all(&bytes)?;
                Ok(bytes.len())
            },
            FieldValue::Numeric(d) => {
                let str_rep = d.to_string();
                dest.write_all(&str_rep.as_bytes())?;
                Ok(str_rep.as_bytes().len())
            },
            _ => unimplemented!(),

        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn read_string_of_len<T: Read>(source: &mut T, len: u8) -> Result<String, std::io::Error> {
    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0u8);
    source.read_exact(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

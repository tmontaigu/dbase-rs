
use std::fmt;
use std::io::{Read, Write};

use std::str::FromStr;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use record::RecordFieldInfo;
use Error;


#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum FieldType {
    // dBASE III
    Character = 'C' as isize,
    Date,
    Float = 'F' as isize,
    Numeric = 'N' as isize,
    Logical,
    // Visual FoxPro
    Currency,
    DateTime,
    Integer,
    // Unknown
    Double,
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
            // dBASE III field types
            // All stored as strings
            'C' => Some(FieldType::Character),
            'D' => Some(FieldType::Date),
            'F' => Some(FieldType::Float),
            'N' => Some(FieldType::Numeric),
            'L' => Some(FieldType::Logical),
            // Visual FoxPro field types
            // stored in binary formats
            'Y' => Some(FieldType::Currency),
            'T' => Some(FieldType::DateTime),
            'I' => Some(FieldType::Integer),
            // unknown version
            'B' => Some(FieldType::Double),
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
            None => Err(Error::InvalidFieldType(c)),
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
        if self.month > 12 || self.day > 31 || self.year < 1900 || self.year > 2155 {
            Err(Error::InvalidDate)
        } else {
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

        Ok(Self { year, month, day })
    }
}

impl std::string::ToString for Date {
    fn to_string(&self) -> String {
        let mut s = String::with_capacity(8);
        let year_str = self.year.to_string();
        let month_str = self.month.to_string();
        let day_str = self.day.to_string();

        if self.year < 100 {
            s.push('0');
            s.push('0');
        } else if self.year < 1000 {
            s.push('0');
        }
        s.push_str(&year_str);

        if self.month < 10 {
            s.push('0');
        }
        s.push_str(&month_str);

        if self.day < 10 {
            s.push('0');
        }
        s.push_str(&day_str);
        s
    }
}


/// Enum where each variant stores the record value
#[derive(Debug, PartialEq)]
pub enum FieldValue {
    // dBase III fields
    // Stored as strings, fully padded (ie only space char) strings
    // are interpreted as None
    Character(Option<String>),
    Numeric(Option<f64>),
    Logical(Option<bool>),
    Date(Option<Date>),
    Float(Option<f32>),
    //Visual FoxPro fields
    Integer(i32),
    Double(f64),
}

impl FieldValue {
    pub(crate) fn read_from<T: Read>(
        mut source: &mut T,
        field_info: &RecordFieldInfo,
    ) -> Result<Self, Error> {
        let value = match field_info.field_type {
            FieldType::Logical => match source.read_u8()? as char {
                ' ' => FieldValue::Logical(None),
                '1' | '0' | 'T' | 't' | 'Y' | 'y'| 'N' | 'n' | 'F' | 'f' => FieldValue::Logical(Some(true)),
                _ => FieldValue::Logical(Some(false)),
            },
            FieldType::Character => {
                let value = read_string_of_len(&mut source, field_info.field_length)?;
                let trimmed_value = value.trim();
                if trimmed_value.is_empty() {
                    FieldValue::Character(None)
                } else {
                    FieldValue::Character(Some(trimmed_value.to_owned()))
                }
            }
            FieldType::Numeric => {
                let value = read_string_of_len(&mut source, field_info.field_length)?;
                let trimmed_value = value.trim();
                if trimmed_value.is_empty() {
                    FieldValue::Numeric(None)
                } else {
                    FieldValue::Numeric(Some(trimmed_value.parse::<f64>()?))
                }
            }
            FieldType::Float => FieldValue::Float(Some(source.read_f32::<LittleEndian>()?)),
            FieldType::Date => {
                let value = read_string_of_len(&mut source, field_info.field_length)?;
                if value.chars().all(|c| c == ' ') {
                    FieldValue::Date(None)
                } else {
                    FieldValue::Date(Some(value.parse::<Date>()?))
                }
            }
            FieldType::Integer => FieldValue::Integer(source.read_i32::<LittleEndian>()?),
            FieldType::Double => FieldValue::Double(source.read_f64::<LittleEndian>()?),
            _ => panic!("unhandled type"),
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

    pub(crate) fn size_in_bytes(&self) -> usize {
        match self {
            FieldValue::Character(value) => {
                match value {
                    Some(s) => {
                        let str_bytes: &[u8] = s.as_ref();
                        str_bytes.len()
                    }
                    None => 0
                }
            }
            FieldValue::Numeric(value) => {
                match value {
                    Some(n) => {
                        let s = n.to_string();
                        s.len()
                    }
                    None => 0
                }
            }
            FieldValue::Logical(_) => 1,
            FieldValue::Date(_) => 8,
            _ => unimplemented!(),
        }
    }

    pub(crate) fn write_to<T: Write>(&self, mut dest: T) -> Result<usize, Error> {
        match self {
            FieldValue::Character(value) => {
                match value {
                    Some(s) => {
                        let bytes = s.as_bytes();
                        dest.write_all(&bytes)?;
                        Ok(bytes.len())
                    }
                    None => Ok(0)
                }
            }
            FieldValue::Numeric(value) => {
                match value {
                    Some(n) => {
                        let str_rep = n.to_string();
                        dest.write_all(&str_rep.as_bytes())?;
                        Ok(str_rep.as_bytes().len())
                    }
                    None => {
                        Ok(0)
                    }
                }
            }
            FieldValue::Logical(value) => {
                if let Some(b) = value {
                    if *b {
                        dest.write_u8('t' as u8)?;
                    } else {
                        dest.write_u8('f' as u8)?;
                    }
                } else {
                    dest.write_u8('?' as u8)?;
                }
                Ok(1)
            }
            FieldValue::Date(value) => {
                match value {
                    Some(d) => {
                        let date_str = d.to_string();
                        let date_str_bytes: &[u8] = date_str.as_ref();
                        dest.write_all(&date_str_bytes)?;
                        Ok(date_str_bytes.len())
                    }
                    None => {
                        dest.write_all(&[' ' as u8; 8])?;
                        Ok(8)
                    }
                }
            }
            FieldValue::Double(d) => {
                dest.write_f64::<LittleEndian>(*d)?;
                Ok(std::mem::size_of::<f64>())
            }
            FieldValue::Integer(i) => {
                dest.write_i32::<LittleEndian>(*i)?;
                Ok(std::mem::size_of::<i32>())
            }
            _ => unimplemented!(),
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}


impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::Character(Some(String::from(s)))
    }
}

impl From<Date> for FieldValue {
    fn from(d: Date) -> Self {
        FieldValue::Date(Some(d))
    }
}

fn read_string_of_len<T: Read>(source: &mut T, len: u8) -> Result<String, std::io::Error> {
    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0u8);
    source.read_exact(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}


#[cfg(test)]
mod test {
    use super::*;

    use record::FieldFlags;
    use std::io::{Cursor, Seek, SeekFrom};
    fn create_temp_record_field_info(field_type: FieldType, len: u8) -> RecordFieldInfo {
        RecordFieldInfo {
            name: "".to_owned(),
            field_type,
            displacement_field: [0u8; 4],
            field_length: len,
            num_decimal_places: 0,
            flags: FieldFlags { 0: 0u8 },
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,
        }
    }

    #[test]
    fn write_read_date() {
        let date = FieldValue::from(Date {
            year: 2019,
            month: 01,
            day: 01,
        });

        let mut out = Cursor::new(Vec::<u8>::new());
        let num_bytes_written = date.write_to(&mut out).unwrap();
        assert_eq!(num_bytes_written, date.size_in_bytes());

        out.seek(SeekFrom::Start(0)).unwrap();
        let record_info = create_temp_record_field_info(FieldType::Date, num_bytes_written as u8);


        match FieldValue::read_from(&mut out, &record_info).unwrap() {
            FieldValue::Date(Some(read_date)) => {
                assert_eq!(read_date.year, 2019);
                assert_eq!(read_date.month, 1);
                assert_eq!(read_date.day, 1);
            }
            _ => assert!(false, "Did not read a date ??"),
        }
    }


    #[test]
    fn write_read_ascii_char() {
        let field = FieldValue::Character(Some(String::from("Only ASCII")));

        let mut out = Cursor::new(Vec::<u8>::new());
        let num_bytes_written = field.write_to(&mut out).unwrap();
        assert_eq!(num_bytes_written, field.size_in_bytes());

        out.seek(SeekFrom::Start(0)).unwrap();
        let record_info =
            create_temp_record_field_info(FieldType::Character, num_bytes_written as u8);


        match FieldValue::read_from(&mut out, &record_info).unwrap() {
            FieldValue::Character(s) => {
                assert_eq!(s, Some(String::from("Only ASCII")));
            }
            _ => assert!(false, "Did not read a Character field ??"),
        }
    }


    #[test]
    fn write_read_utf8_char() {
        let field = FieldValue::Character(Some(String::from("🤔")));

        let mut out = Cursor::new(Vec::<u8>::new());
        let num_bytes_written = field.write_to(&mut out).unwrap();
        assert_eq!(num_bytes_written, field.size_in_bytes());

        out.seek(SeekFrom::Start(0)).unwrap();
        let record_info =
            create_temp_record_field_info(FieldType::Character, num_bytes_written as u8);


        match FieldValue::read_from(&mut out, &record_info).unwrap() {
            FieldValue::Character(s) => {
                assert_eq!(s, Some(String::from("🤔")));
            }
            _ => assert!(false, "Did not read a Character field ??"),
        }
    }
}

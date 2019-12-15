use std::convert::TryFrom;
use std::io::{Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use ::{Error, FieldValue};
use record::field::FieldType;

pub mod field;

const DELETION_FLAG_NAME: &'static str = "DeletionFlag";
const FIELD_NAME_LENGTH: usize = 11;

#[derive(Debug)]
/// Wrapping struct to create a FieldName from a String.
///
/// FieldNames in the dBase format cannot exceed 11 bytes (not char).
///
/// # Examples
///
/// ```
/// use dbase::FieldName;
/// use std::convert::TryFrom;
///
/// let name = FieldName::try_from("Small Name");
/// assert!(name.is_ok())
/// ```
pub struct FieldName(String);

impl TryFrom<&str> for FieldName {
    type Error = &'static str;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        if name.as_bytes().len() > FIELD_NAME_LENGTH {
            Err("FieldName byte representation cannot exceed 11 bytes")
        } else {
            Ok(Self { 0: name.to_string() })
        }
    }
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FieldFlags(u8);

impl FieldFlags {
    pub fn new() -> Self {
        Self { 0: 0 }
    }

    pub fn system_column(self) -> bool {
        (self.0 & 0x01) != 0
    }

    pub fn can_store_null(self) -> bool {
        (self.0 & 0x02) != 0
    }

    pub fn is_binary(self) -> bool {
        (self.0 & 0x04) != 0
    }

    pub fn is_auto_incrementing(self) -> bool {
        (self.0 & 0x0C) != 0
    }
}

/// Struct giving the info for a record field
#[derive(Debug, PartialEq)]
pub struct FieldInfo {
    /// The name of the field
    pub(crate) name: String,
    /// The field type
    pub(crate) field_type: FieldType,
    pub(crate) displacement_field: [u8; 4],
    pub(crate) field_length: u8,
    pub(crate) num_decimal_places: u8,
    pub(crate) flags: FieldFlags,
    pub(crate) autoincrement_next_val: [u8; 5],
    pub(crate) autoincrement_step: u8,
}


impl FieldInfo {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn new(name: FieldName, field_type: FieldType, length: u8) -> Self {
        Self {
            name: name.0,
            field_type,
            displacement_field: [0u8; 4],
            field_length: length,
            num_decimal_places: 0,
            flags: FieldFlags::new(),
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,
        }
    }

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, Error> {
        let mut name = [0u8; FIELD_NAME_LENGTH];
        source.read_exact(&mut name)?;
        let field_type = source.read_u8()?;

        let mut displacement_field = [0u8; 4];
        source.read_exact(&mut displacement_field)?;

        let record_length = source.read_u8()?;
        let num_decimal_places = source.read_u8()?;

        let flags = FieldFlags {
            0: source.read_u8()?,
        };

        let mut autoincrement_next_val = [0u8; 5];

        source.read_exact(&mut autoincrement_next_val)?;
        let autoincrement_step = source.read_u8()?;

        let mut _reserved = [0u8; 7];
        source.read_exact(&mut _reserved)?;

        let s = String::from_utf8_lossy(&name)
            .trim_matches(|c| c == '\u{0}')
            .to_owned();
        let field_type = FieldType::try_from(field_type as char)?;

        Ok(Self {
            name: s,
            field_type,
            displacement_field,
            field_length: record_length,
            num_decimal_places,
            flags,
            autoincrement_next_val,
            autoincrement_step,
        })
    }

    pub(crate) fn write_to<T: Write>(&self, dest: &mut T) -> Result<(), Error> {
        let num_bytes = self.name.as_bytes().len();
        let mut name_bytes = [0u8; FIELD_NAME_LENGTH];
        name_bytes[..num_bytes.min(FIELD_NAME_LENGTH)].copy_from_slice(self.name.as_bytes());
        dest.write_all(&name_bytes)?;

        dest.write_u8(u8::from(self.field_type))?;
        dest.write_all(&self.displacement_field)?;
        dest.write_u8(self.field_length)?;
        dest.write_u8(self.num_decimal_places)?;
        dest.write_u8(self.flags.0)?;
        dest.write_all(&self.autoincrement_next_val)?;
        dest.write_u8(self.autoincrement_step)?;

        let reserved = [0u8; 7];
        dest.write_all(&reserved)?;

        Ok(())
    }

    pub(crate) fn new_deletion_flag() -> Self {
        Self {
            name: DELETION_FLAG_NAME.to_owned(),
            field_type: FieldType::Character,
            displacement_field: [0u8; 4],
            field_length: 1,
            num_decimal_places: 0,
            flags: FieldFlags { 0: 0u8 },
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,

        }
    }

    pub(crate) fn is_deletion_flag(&self) -> bool {
        &self.name == DELETION_FLAG_NAME
    }
}

/// Errors that can happen when trying to convert a FieldValue into
/// a more concrete type
#[derive(Debug)]
pub enum FieldConversionError {
    /// Happens when the conversion could not be mode because the FieldType
    /// does not mat the expected one
    FieldTypeNotAsExpected { expected: FieldType, actual: FieldType },
    NoneValue,
}

macro_rules! impl_try_from_field_value_for_ {
    (FieldValue::$variant:ident => $out_type:ty) => {
        impl TryFrom<FieldValue> for $out_type {
            type Error = FieldConversionError;

            fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
                if let FieldValue::$variant(v) = value {
                    Ok(v)
                } else {
                     Err(FieldConversionError::FieldTypeNotAsExpected {
                        expected: FieldType::$variant,
                        actual: value.field_type()
                     })
                }
            }
        }
    };
     (FieldValue::$variant:ident(Some($v:ident)) => $out_type:ty) => {
        impl TryFrom<FieldValue> for $out_type {
            type Error = FieldConversionError;

            fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
                match value {
                    FieldValue::$variant(Some($v)) => Ok($v),
                    FieldValue::$variant(None) => Err(FieldConversionError::NoneValue),
                    _ => Err(FieldConversionError::FieldTypeNotAsExpected { expected: FieldType::$variant, actual: value.field_type()})
                }
            }
        }
    };
}

impl_try_from_field_value_for_!(FieldValue::Numeric => Option<f64>);
impl_try_from_field_value_for_!(FieldValue::Numeric(Some(v)) => f64);

impl_try_from_field_value_for_!(FieldValue::Float => Option<f32>);
impl_try_from_field_value_for_!(FieldValue::Float(Some(v)) => f32);

impl_try_from_field_value_for_!(FieldValue::Date => Option<field::Date>);
impl_try_from_field_value_for_!(FieldValue::Date(Some(v)) => field::Date);

impl_try_from_field_value_for_!(FieldValue::Character => Option<String>);
impl_try_from_field_value_for_!(FieldValue::Character(Some(string)) => String);

impl_try_from_field_value_for_!(FieldValue::Logical => Option<bool>);
impl_try_from_field_value_for_!(FieldValue::Logical(Some(b)) => bool);

impl From<String> for FieldValue {
    fn from(s: String) -> Self {
        FieldValue::Character(Some(s))
    }
}

impl From<f64> for FieldValue {
    fn from(v: f64) -> Self {
        FieldValue::Numeric(Some(v))
    }
}

impl From<f32> for FieldValue {
    fn from(v: f32) -> Self {
        FieldValue::Float(Some(v))
    }
}

impl From<bool> for FieldValue {
    fn from(b: bool) -> FieldValue {
        FieldValue::Logical(Some(b))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_read_field_info() {
        let field_info = FieldInfo::new(FieldName::try_from("LICENSE").unwrap(), FieldType::Character, 30);
        let mut cursor = Cursor::new(Vec::<u8>::with_capacity(FieldInfo::SIZE));
        field_info.write_to(&mut cursor).unwrap();

        cursor.set_position(0);

        let read_field_info = FieldInfo::read_from(&mut cursor).unwrap();

        assert_eq!(read_field_info, field_info);
    }
}


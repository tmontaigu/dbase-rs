use std::convert::TryFrom;
use std::io::{Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

pub mod types;

use self::types::{Date, DateTime, FieldType};
use crate::{Encoding, ErrorKind, FieldValue};

const DELETION_FLAG_NAME: &str = "DeletionFlag";
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
            Ok(Self(name.to_string()))
        }
    }
}

/// Struct giving the info for a record field
#[derive(Debug, PartialEq, Clone)]
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn field_type(&self) -> FieldType {
        self.field_type
    }

    pub fn length(&self) -> u8 {
        self.field_length
    }

    pub(crate) fn new(name: FieldName, field_type: FieldType, length: u8) -> Self {
        Self {
            name: name.0,
            field_type,
            displacement_field: [0u8; 4],
            field_length: length,
            num_decimal_places: 0,
            flags: FieldFlags::default(),
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,
        }
    }

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, ErrorKind> {
        Self::read_with_encoding(source, &crate::encoding::Ascii)
    }

    /// Reads with the given encoding.
    ///
    /// The encoding is used only for the name
    fn read_with_encoding<T: Read, E: Encoding>(
        source: &mut T,
        encoding: &E,
    ) -> Result<Self, ErrorKind> {
        let mut name = [0u8; FIELD_NAME_LENGTH];
        source.read_exact(&mut name)?;
        let field_type = source.read_u8()?;

        let mut displacement_field = [0u8; 4];
        source.read_exact(&mut displacement_field)?;

        let record_length = source.read_u8()?;
        let num_decimal_places = source.read_u8()?;

        let flags = FieldFlags(source.read_u8()?);

        let mut autoincrement_next_val = [0u8; 5];

        source.read_exact(&mut autoincrement_next_val)?;
        let autoincrement_step = source.read_u8()?;

        let mut _reserved = [0u8; 7];
        source.read_exact(&mut _reserved)?;

        let s = encoding
            .decode(&name)?
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

    pub(crate) fn write_to<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
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
            flags: FieldFlags(0u8),
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,
        }
    }

    pub(crate) fn is_deletion_flag(&self) -> bool {
        self.name == DELETION_FLAG_NAME
    }
}

impl std::fmt::Display for FieldInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FieldInfo {{ Name: {}, Field Type: {} }}",
            self.name, self.field_type
        )
    }
}

/// Flags describing a field
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub(crate) struct FieldFlags(u8);

/// Errors that can happen when trying to convert a FieldValue into
/// a more concrete type
#[derive(Debug)]
pub enum FieldConversionError {
    /// Happens when the conversion could not be mode because the FieldType
    /// does not mat the expected one
    FieldTypeNotAsExpected {
        /// The expected FieldType of the FieldValue the conversion was tried on
        expected: FieldType,
        /// The actual FieldType of the FieldValue the conversion was tried on
        actual: FieldType,
    },
    IncompatibleType,
    /// The value written is the file was only pad bytes / uninitialized
    /// and the user tried to convert it into a non Option-Type
    NoneValue,
}

impl std::fmt::Display for FieldConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldConversionError::FieldTypeNotAsExpected { expected, actual } => {
                write!(f, "Cannot convert from {} to {}", expected, actual)
            }
            FieldConversionError::IncompatibleType => write!(f, "The type is not compatible"),
            FieldConversionError::NoneValue => {
                write!(f, "Value is not initialized, which is not allowed")
            }
        }
    }
}

impl std::error::Error for FieldConversionError {}

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
                        actual: value.field_type(),
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
                    _ => Err(FieldConversionError::FieldTypeNotAsExpected {
                        expected: FieldType::$variant,
                        actual: value.field_type(),
                    }),
                }
            }
        }
    };
}

impl_try_from_field_value_for_!(FieldValue::Numeric => Option<f64>);

impl_try_from_field_value_for_!(FieldValue::Float => Option<f32>);
impl_try_from_field_value_for_!(FieldValue::Float(Some(v)) => f32);

impl_try_from_field_value_for_!(FieldValue::Date => Option<types::Date>);
impl_try_from_field_value_for_!(FieldValue::Date(Some(v)) => types::Date);

impl_try_from_field_value_for_!(FieldValue::Character => Option<String>);
impl_try_from_field_value_for_!(FieldValue::Character(Some(string)) => String);

impl_try_from_field_value_for_!(FieldValue::Logical => Option<bool>);
impl_try_from_field_value_for_!(FieldValue::Logical(Some(b)) => bool);

impl_try_from_field_value_for_!(FieldValue::Integer => i32);

impl TryFrom<FieldValue> for f64 {
    type Error = FieldConversionError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        match value {
            FieldValue::Numeric(Some(v)) => Ok(v),
            FieldValue::Numeric(None) => Err(FieldConversionError::NoneValue),
            FieldValue::Currency(c) => Ok(c),
            FieldValue::Double(d) => Ok(d),
            _ => Err(FieldConversionError::IncompatibleType),
        }
    }
}

// Fox Pro types
impl_try_from_field_value_for_!(FieldValue::DateTime => DateTime);

macro_rules! impl_from_type_for_field_value (
    ($t:ty => FieldValue::$variant:ident) => {
        impl From<$t> for FieldValue {
            fn from(v: $t) -> Self {
                FieldValue::$variant(v)
            }
        }
    };
    ($t:ty => FieldValue::$variant:ident(Some($v:ident))) => {
        impl From<$t> for FieldValue {
            fn from(v: $t) -> Self {
                FieldValue::$variant(Some(v))
            }
        }
    }
);

impl_from_type_for_field_value!(Option<String> => FieldValue::Character);
impl_from_type_for_field_value!(String => FieldValue::Character(Some(s)));

impl_from_type_for_field_value!(Option<f64> => FieldValue::Numeric);
impl_from_type_for_field_value!(f64 => FieldValue::Numeric(Some(v)));

impl_from_type_for_field_value!(Option<f32> => FieldValue::Float);
impl_from_type_for_field_value!(f32 => FieldValue::Float(Some(v)));

impl_from_type_for_field_value!(Option<bool> => FieldValue::Logical);
impl_from_type_for_field_value!(bool => FieldValue::Logical(Some(v)));

impl_from_type_for_field_value!(Option<Date> => FieldValue::Date);
impl_from_type_for_field_value!(Date => FieldValue::Date(Some(v)));

// Fox Pro types
impl_from_type_for_field_value!(DateTime => FieldValue::DateTime);

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_read_field_info() {
        let field_info = FieldInfo::new(
            FieldName::try_from("LICENSE").unwrap(),
            FieldType::Character,
            30,
        );
        let mut cursor = Cursor::new(Vec::<u8>::with_capacity(FieldInfo::SIZE));
        field_info.write_to(&mut cursor).unwrap();

        cursor.set_position(0);

        let read_field_info = FieldInfo::read_from(&mut cursor).unwrap();

        assert_eq!(read_field_info, field_info);
    }
}

use std::convert::TryFrom;
use std::io::{Read, Write};
use std::ops::Index;
use std::slice::SliceIndex;

use byteorder::{ReadBytesExt, WriteBytesExt};

mod conversion;
pub mod types;

use self::types::FieldType;
use crate::{Encoding, ErrorKind, FieldValue};
pub use conversion::FieldConversionError;

pub(crate) const DELETION_FLAG_SIZE: usize = 1; // 1 byte
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
        if name.len() > FIELD_NAME_LENGTH {
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

    pub(crate) fn write_to<T: Write, E: Encoding>(
        &self,
        dest: &mut T,
        encoding: &E,
    ) -> std::io::Result<()> {
        let mut name_bytes = [0u8; FIELD_NAME_LENGTH];
        let encoded = encoding.encode(&self.name).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to encode field name",
            )
        })?;
        let num_bytes = encoded.len().min(FIELD_NAME_LENGTH);
        name_bytes[..num_bytes].copy_from_slice(&encoded[..num_bytes]);
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
}

#[derive(Debug)]
pub struct FieldsInfo {
    pub(crate) inner: Vec<FieldInfo>,
}

impl FieldsInfo {
    pub(crate) fn read_from<R: Read>(source: &mut R, num_fields: usize) -> Result<Self, ErrorKind> {
        Self::read_with_encoding(source, num_fields, &crate::encoding::Ascii)
    }

    pub(crate) fn read_with_encoding<R: Read, E: Encoding>(
        source: &mut R,
        num_fields: usize,
        encoding: &E,
    ) -> Result<Self, ErrorKind> {
        let mut fields_info = Vec::<FieldInfo>::with_capacity(num_fields);
        for _ in 0..num_fields {
            let info = FieldInfo::read_with_encoding(source, encoding)?;
            fields_info.push(info);
        }

        Ok(Self { inner: fields_info })
    }

    pub(crate) fn field_position_in_record(&self, index: usize) -> Option<usize> {
        self.inner
            .get(..index)
            .map(|slc| slc.iter().map(|i| i.field_length as usize).sum::<usize>())
            .map(|s| s + DELETION_FLAG_SIZE)
    }

    pub(crate) fn size_of_all_fields(&self) -> usize {
        self.inner
            .iter()
            .map(|i| i.field_length as usize)
            .sum::<usize>()
    }

    pub(crate) fn at_least_one_field_is_memo(&self) -> bool {
        self.inner
            .iter()
            .any(|f_info| f_info.field_type == FieldType::Memo)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, FieldInfo> {
        self.inner.iter()
    }
}

impl AsRef<[FieldInfo]> for FieldsInfo {
    fn as_ref(&self) -> &[FieldInfo] {
        &self.inner
    }
}

impl<I> Index<I> for FieldsInfo
where
    I: SliceIndex<[FieldInfo]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.inner.as_slice()[index]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeletionFlag {
    NotDeleted,
    Deleted,
}

impl DeletionFlag {
    pub(crate) const fn to_byte(self) -> u8 {
        match self {
            Self::NotDeleted => 0x20,
            Self::Deleted => 0x2A,
        }
    }

    pub(crate) const fn from_byte(byte: u8) -> Self {
        match byte {
            0x20 => Self::NotDeleted,
            0x2A => Self::Deleted,
            // Silently consider other values as not deleted
            _ => Self::NotDeleted,
        }
    }

    pub(crate) fn read_from<T: Read>(source: &mut T) -> std::io::Result<Self> {
        source.read_u8().map(Self::from_byte)
    }

    pub(crate) fn write_to<T: Write>(self, dst: &mut T) -> std::io::Result<()> {
        dst.write_u8(self.to_byte())
    }
}
/// Flags describing a field
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub(crate) struct FieldFlags(u8);

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
        field_info
            .write_to(&mut cursor, &crate::encoding::Ascii)
            .unwrap();

        cursor.set_position(0);

        let read_field_info = FieldInfo::read_from(&mut cursor).unwrap();

        assert_eq!(read_field_info, field_info);
    }
}

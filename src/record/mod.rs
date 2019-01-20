use std::io::{Read};

use byteorder::{ReadBytesExt};

pub mod field;
use record::field::{FieldType};
use Error;


pub struct RecordFieldInfo {
    pub name: String,
    pub field_type: FieldType,
    pub record_length: u8,
    pub num_decimal_places: u8,
}


impl RecordFieldInfo {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, Error> {
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

    pub fn new_deletion_flag() -> Self {
        Self{
            name: "DeletionFlag".to_owned(),
            field_type: FieldType::Character,
            record_length: 1,
            num_decimal_places: 0
        }
    }
}

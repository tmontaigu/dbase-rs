use std::io::{Read};

use byteorder::{ReadBytesExt};

pub mod field;
use record::field::{FieldType};
use Error;


pub struct FieldFlags(u8);

impl FieldFlags {
    pub fn system_column(&self) -> bool {
        (self.0 & 0x01) != 0
    }

    pub fn can_store_null(&self) -> bool {
        (self.0 & 0x02) != 0
    }

    pub fn is_binary(&self) -> bool {
        (self.0 & 0x04) != 0
    }

    pub fn is_auto_incrementing(&self) -> bool {
        (self.0 & 0x0C) != 0
    }
}

/// Struct giving the info for a record field
pub struct RecordFieldInfo {
    /// The name of the field
    pub name: String,
    /// The field type
    pub field_type: FieldType,
    pub record_length: u8,
    pub num_decimal_places: u8,
    pub flags: FieldFlags,
    pub autoincrement_next_val: [u8; 5],
    pub autoincrement_step: u8,
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

        let flags = FieldFlags{0: source.read_u8()?};

        let mut autoincrement_next_val = [0u8; 5];
        source.read_exact(&mut autoincrement_next_val)?;
        
        let autoincrement_step = source.read_u8()?;

        let mut _reserved = [0u8; 7];
        source.read_exact(&mut _reserved)?;

        let s = String::from_utf8_lossy(&name).trim_matches(|c| c == '\u{0}').to_owned();
        let field_type = FieldType::try_from(field_type as char)?;

        Ok(Self{
            name: s,
            field_type,
            record_length,
            num_decimal_places,
            flags, 
            autoincrement_next_val,
            autoincrement_step,
        })
    }

    pub fn new_deletion_flag() -> Self {
        Self{
            name: "DeletionFlag".to_owned(),
            field_type: FieldType::Character,
            record_length: 1,
            num_decimal_places: 0,
            flags: FieldFlags{0: 0u8},
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,

        }
    }
}

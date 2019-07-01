use std::io::{Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

pub mod field;
use record::field::FieldType;
use Error;


#[derive(Copy, Clone)]
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
pub struct RecordFieldInfo {
    /// The name of the field
    pub name: String,
    /// The field type
    pub field_type: FieldType,
    pub displacement_field: [u8; 4],
    pub field_length: u8,
    pub num_decimal_places: u8,
    pub flags: FieldFlags,
    pub autoincrement_next_val: [u8; 5],
    pub autoincrement_step: u8,
}


impl RecordFieldInfo {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn new(name: String, field_type: FieldType, length: u8) -> Self {
        Self {
            name,
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
        let mut name = [0u8; 11];
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
        if num_bytes > 10 {
            return Err(Error::FieldLengthTooLong);
        }
        dest.write_all(&self.name.as_bytes()[0..num_bytes])?;
        let mut name_bytes = [0u8; 11];
        name_bytes[10] = '\0' as u8;
        dest.write_all(&name_bytes[0..11 - num_bytes])?;

        dest.write_u8(self.field_type as u8)?;

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

    pub fn new_deletion_flag() -> Self {
        Self {
            name: "DeletionFlag".to_owned(),
            field_type: FieldType::Character,
            displacement_field: [0u8; 4],
            field_length: 1,
            num_decimal_places: 0,
            flags: FieldFlags { 0: 0u8 },
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,

        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use header::Header;


    use std::fs::File;
    use std::io::{Cursor, Seek, SeekFrom};
    #[test]
    fn test_record_info_read_writing() {
        let mut file = File::open("tests/data/line.dbf").unwrap();
        file.seek(SeekFrom::Start(Header::SIZE as u64)).unwrap();

        let mut record_info_bytes = [0u8; RecordFieldInfo::SIZE];
        file.read_exact(&mut record_info_bytes).unwrap();
        let mut cursor = Cursor::new(record_info_bytes);

        let records_info = RecordFieldInfo::read_from(&mut cursor).unwrap();


        let mut out = Cursor::new(Vec::<u8>::with_capacity(RecordFieldInfo::SIZE));
        records_info.write_to(&mut out).unwrap();

        let bytes_written = out.into_inner();
        assert_eq!(bytes_written.len(), record_info_bytes.len());
        assert_eq!(bytes_written, record_info_bytes);
    }
}


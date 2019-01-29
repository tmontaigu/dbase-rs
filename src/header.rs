#[allow(dead_code)]
use std::io::{Read};
use byteorder::{LittleEndian, ReadBytesExt};

use record::field::Date;

pub struct FileType(u8);

impl FileType {
    pub fn version_numer(&self) -> u8 {
        self.0 & 0b0000111
    }

    pub fn has_dbase_sql_table(&self) -> bool {
        (self.0 & 0b00110000) != 0
    }
}

pub struct TableFlags(u8);

impl TableFlags {
    pub fn has_structural_cdx(&self) -> bool {
        (self.0 & 0x01) == 1
    }

    pub fn has_memo_field(&self) -> bool {
        (self.0 & 0x02) == 1
    }

    pub fn is_a_database(&self) -> bool {
        (self.0 & 0x03) == 1
    }
}


pub struct Header {
    pub file_type: FileType,
    pub last_update: Date,
    pub num_records: u32,
    pub offset_to_first_record: u16,
    pub size_of_record: u16,
    pub is_transaction_incomplete: bool,
    pub encryption_flag: u8,
    pub table_flags: TableFlags,
    pub code_page_mark: u8,
}


impl Header {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let file_type = FileType{0: source.read_u8()?};

        let mut date = [0u8; 3];
        source.read_exact(&mut date)?;
        let last_update = Date::from_bytes(date);

        let num_records = source.read_u32::<LittleEndian>()?;
        let offset_to_first_record = source.read_u16::<LittleEndian>()?;
        let size_of_record = source.read_u16::<LittleEndian>()?;

        let _reserved = source.read_u16::<LittleEndian>()?;

        let is_transaction_incomplete = (source.read_u8()? != 0) as bool;
        let encryption_flag = source.read_u8()?;

        let mut _reserved = [0u8; 12];
        source.read_exact(&mut _reserved)?;

        let table_flags = TableFlags{0: source.read_u8()?};

        let code_page_mark = source.read_u8()?;

        let _reserved = source.read_u8()?;
        let terminator = source.read_u8()?;

        if terminator != 0x0D {
            println!("Strange header terminator value: {}", terminator);
        }

        Ok(Self {
            file_type,
            last_update,
            num_records,
            offset_to_first_record,
            is_transaction_incomplete,
            encryption_flag,
            size_of_record,
            table_flags,
            code_page_mark,
        })
    }
}

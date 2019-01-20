use std::io::{Read};
use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) struct Header {
    pub(crate) num_records: u32,
    pub(crate) offset_to_first_record: u16,
    #[allow(dead_code)]
    pub(crate) size_of_record: u16,
}

impl Header {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let mut skip = [0u8; 4];
        source.read_exact(&mut skip)?; //level + last date

        let num_records = source.read_u32::<LittleEndian>()?;
        let offset_to_first_record = source.read_u16::<LittleEndian>()?;
        let size_of_record = source.read_u16::<LittleEndian>()?;

        let mut skip = [0u8; 20];
        source.read_exact(&mut skip)?; //level + last date

        Ok(Self {
            num_records,
            offset_to_first_record,
            size_of_record,
        })
    }
}

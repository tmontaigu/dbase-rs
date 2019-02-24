use std::io::Write;

use byteorder::WriteBytesExt;

use {Error, Record};
use header::Header;
use reading::TERMINATOR_VALUE;
use record::RecordFieldInfo;

const FILE_TERMINATOR: u8 = 0x1A;

pub struct Writer<T: Write> {
    dest: T,
}


impl<T: Write> Writer<T> {
    pub fn new(dest: T) -> Self {
        Self { dest }
    }

    pub fn write(mut self, records: Vec<Record>) -> Result<(T), Error> {
        if records.is_empty() {
            return Ok(self.dest);
        }
        let fields_name: Vec<&String> = records[0].keys().collect();

        let mut fields_info = Vec::<RecordFieldInfo>::with_capacity(fields_name.len());
        for (field_name, field_value) in &records[0] {
            let field_length = field_value.size_in_bytes();
            if field_length > std::u8::MAX as usize {
                return Err(Error::FieldLengthTooLong);
            }

            fields_info.push(
                RecordFieldInfo::new(field_name.to_owned(), field_value.field_type(), field_length as u8)
            );
        }

        for record in &records[1..records.len()] {
            for (field_name, record_info) in fields_name.iter().zip(&mut fields_info) {
                let field_value = record.get(*field_name).unwrap(); // TODO: Should return an Err()
                let field_length = field_value.size_in_bytes();
                if field_length > std::u8::MAX as usize {
                    return Err(Error::FieldLengthTooLong);
                }
                record_info.field_length = std::cmp::max(record_info.field_length, field_length as u8);
            }
        }

        let offset_to_first_record = Header::SIZE + (fields_info.len() * RecordFieldInfo::SIZE) + std::mem::size_of::<u8>();
        let size_of_record = fields_info.iter().fold(0u16, |s, ref info| s + info.field_length as u16);
        let hdr = Header::new(records.len() as u32, offset_to_first_record as u16, size_of_record);

        hdr.write_to(&mut self.dest)?;
        for record_info in &fields_info {
            record_info.write_to(&mut self.dest)?;
        }

        self.dest.write_u8(TERMINATOR_VALUE)?;

        let value_buffer = [' ' as u8; std::u8::MAX as usize];
        for record in &records {
            self.dest.write_u8(' ' as u8)?; // DeletionFlag
            for (field_name, record_info) in fields_name.iter().zip(&fields_info) {
                let value = record.get(*field_name).unwrap();
                let bytes_written = value.write_to(&mut self.dest)? as u8;
                if bytes_written > record_info.field_length {
                    panic!("record length was miscalculated");
                }

                let bytes_to_pad = record_info.field_length - bytes_written;
                self.dest.write_all(&value_buffer[0..bytes_to_pad as usize])?;
            }
        }
        self.dest.write_u8(FILE_TERMINATOR)?;
        Ok(self.dest)
    }
}

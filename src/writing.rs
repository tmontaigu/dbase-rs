
use byteorder::{WriteBytesExt};

use record::{RecordFieldInfo, FieldFlags};
use reading::TERMINATOR_VALUE;
use header::Header;
use {Error, Record};
use std::io::Write;

const FILE_TERMINATOR: u8 = 0x1A;

pub struct Writer<T: Write> {
    dest: T,
}


impl<T: Write> Writer<T> {
    pub fn new(dest: T) -> Self {
        Self { dest }
    }

    pub fn write(mut self, records: Vec<Record>) -> Result<(), Error> {
        if records.len() < 1 {
            return Ok(());
        }

        let fields_name: Vec<&String> = records[0].keys().collect();

        let mut records_info = Vec::<RecordFieldInfo>::with_capacity(fields_name.len());
        for (field_name, field_value) in &records[0] {
            records_info.push(
                RecordFieldInfo{
                    name: field_name.to_owned(),
                    field_type: field_value.field_type(),
                    displacement_field: [0u8; 4],
                    record_length: field_value.size_in_bytes() as u8, //FIXME chec fo overflow
                    num_decimal_places: 0,
                    flags: FieldFlags::new(),
                    autoincrement_next_val: [0u8; 5],
                    autoincrement_step: 0u8,
                }
            );
        }

        for record in &records[1..records.len()] {
            for (field_name, record_info) in fields_name.iter().zip(&mut records_info) {
                let field_value = record.get(*field_name).unwrap();
                record_info.record_length = std::cmp::max(record_info.record_length, field_value.size_in_bytes() as u8);
            }
        }


        let offset_to_first_record = Header::SIZE + (records.len() * RecordFieldInfo::SIZE) + std::mem::size_of::<u8>();
        let size_of_record = records_info.iter().fold(0u16, |s, ref info| s + info.record_length as u16);
        let hdr = Header::new(records.len() as u32, offset_to_first_record as u16, size_of_record);

        hdr.write_to(&mut self.dest)?;
        for record_info in &records_info {
            record_info.write_to(&mut self.dest)?;
        }

        self.dest.write_u8(TERMINATOR_VALUE)?;

        let value_buffer = [0u8; std::u8::MAX as usize];
        for record in &records {
            self.dest.write_u8(' ' as u8)?; // DeletionFlag
            for (field_name, record_info) in fields_name.iter().zip(&records_info) {
                let value = record.get(*field_name).unwrap();
                let bytes_written = value.write_to(&mut self.dest)? as u8;
                if bytes_written > record_info.record_length {
                    panic!("record length was miscalculated");
                }

                let bytes_to_pad = record_info.record_length - bytes_written;
                self.dest.write_all(&value_buffer[0..bytes_to_pad as usize])?;
            }
        }
       self.dest.write_u8(FILE_TERMINATOR)?;
       Ok(())
    }
}

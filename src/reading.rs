//! Module with the definition of fn's and struct's to read .dbf files

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use byteorder::ReadBytesExt;

use header::Header;

use record::field::{FieldValue, MemoFileType, MemoReader};
use record::RecordFieldInfo;
use Error;

/// Value of the byte between the last RecordFieldInfo and the first record
pub(crate) const TERMINATOR_VALUE: u8 = 0x0D;

/// Type definition of a record.
/// A .dbf file is composed of many records
pub type Record = HashMap<String, FieldValue>;

/// Struct with the handle to the source .dbf file
/// Responsible for reading the content
pub struct Reader<T: Read + Seek> {
    /// Where the data is read from
    source: T,
    memo_reader: Option<MemoReader<T>>,
    header: Header,
    fields_info: Vec<RecordFieldInfo>,
    current_record: u32,
}

impl<T: Read + Seek> Reader<T> {
    /// Creates a new reader from the source.
    ///
    /// Reads the header and fields information as soon as its created.
    ///
    /// # Example
    ///
    /// ```
    /// let reader = dbase::Reader::from_path("tests/data/line.dbf").unwrap();
    ///
    /// ```
    ///
    /// ```
    /// use std::fs::File;
    /// let f = File::open("tests/data/line.dbf").unwrap();
    /// let reader = dbase::Reader::new(f).unwrap();
    /// ```
    pub fn new(mut source: T) -> Result<Self, Error> {
        let header = Header::read_from(&mut source)?;
        let offset_to_first_record = if header.file_type.is_visual_fox_pro() {
            header.offset_to_first_record - 263
        } else {
            header.offset_to_first_record
        };
        let num_fields =
            (offset_to_first_record as usize - Header::SIZE - std::mem::size_of::<u8>())
                / RecordFieldInfo::SIZE;

        let mut fields_info = Vec::<RecordFieldInfo>::with_capacity(num_fields as usize + 1);
        fields_info.push(RecordFieldInfo::new_deletion_flag());
        for i in 0..num_fields {
            println!("{} / {}", i, num_fields);
            let info = RecordFieldInfo::read_from(&mut source)?;
            println!("{:?}", info);
            fields_info.push(info);
        }

        let terminator = source.read_u8()?;
        if terminator != TERMINATOR_VALUE {
            panic!("unexpected terminator");
        }

        if header.file_type.is_visual_fox_pro() {
            let mut backlink = [0u8; 263];
            source.read_exact(&mut backlink)?;
        }

        Ok(Self {
            source,
            memo_reader: None,
            header,
            fields_info,
            current_record: 0,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Make the `Reader` read the [Records](type.Record.html)
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    ///
    /// let f = File::open("tests/data/line.dbf").unwrap();
    /// let reader = dbase::Reader::new(f).unwrap();
    /// let records = reader.read().unwrap();
    /// assert_eq!(records.len(), 1);
    /// ```
    pub fn read(self) -> Result<Vec<Record>, Error> {
        let mut records = Vec::<Record>::with_capacity(self.fields_info.len());
        for record in self {
            records.push(record?);
        }
        //let file_end = self.source.read_u16::<LittleEndian>()?;
        Ok(records)
    }
}

impl Reader<BufReader<File>> {
    /// Creates a new dbase Reader from a path
    ///
    /// # Example
    ///
    /// ```
    /// let reader = dbase::Reader::from_path("tests/data/line.dbf").unwrap();
    /// ```
    ///
    ///
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let p = path.as_ref().to_owned();
        let bufreader = BufReader::new(File::open(path)?);
        let mut reader = Reader::new(bufreader)?;
        //TODO should probably have only 1 fn that returns an Option<MemoFileType>
        if reader.header.file_type.has_memo() {
            let memo_type = reader.header.file_type.memo_type();
            let memo_path = match memo_type {
                MemoFileType::DbaseMemo => p.with_extension("dbt"),
                MemoFileType::FoxBaseMemo => p.with_extension("fpt")
            };
            // TODO if this fails, the returned error is not enough explicit about the fact that 
            // the needed memo file could not be found
            let memo_reader = MemoReader::new(memo_type, BufReader::new(File::open(memo_path)?))?;
            reader.memo_reader = Some(memo_reader);
        }
        Ok(reader)
    }
}


impl<T: Read + Seek> Iterator for Reader<T> {
    type Item = Result<Record, Error>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.current_record >= self.header.num_records {
            None
        } else {
            let mut record = Record::with_capacity(self.fields_info.len() as usize);
            for field_info in &self.fields_info {
                let value = match FieldValue::read_from(&mut self.source, &mut self.memo_reader, field_info) {
                    Err(e) => return Some(Err(e)),
                    Ok(value) => value,
                };

                if field_info.name != "DeletionFlag" {
                    record.insert(field_info.name.clone(), value);
                }
            }
            self.current_record += 1;
            Some(Ok(record))
        }
    }
}

/// One liner to read the content of a .dbf file
///
/// # Example
///
/// ```
/// let records = dbase::read("tests/data/line.dbf").unwrap();
/// assert_eq!(records.len(), 1);
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> Result<Vec<Record>, Error> {
    let reader = Reader::from_path(path)?;
    reader.read()
}

#[cfg(test)]
mod test {
    use super::*;

    use std::fs::File;
    use std::io::{Seek, SeekFrom};
    #[test]
    fn pos_after_reading() {
        let file = File::open("tests/data/line.dbf").unwrap();
        let mut reader = Reader::new(file).unwrap();
        let pos_after_reading = reader.source.seek(SeekFrom::Current(0)).unwrap();

        // Do not count the the "DeletionFlag record info that is added
        let mut expected_pos =
            Header::SIZE + ((reader.fields_info.len() - 1) * RecordFieldInfo::SIZE);
        // Add the terminator
        expected_pos += std::mem::size_of::<u8>();
        assert_eq!(pos_after_reading, expected_pos as u64);
    }
}

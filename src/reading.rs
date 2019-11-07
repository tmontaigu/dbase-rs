//! Module with the definition of fn's and struct's to read .dbf files

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use byteorder::ReadBytesExt;

use ::{Error, ReadableRecord};
use header::Header;
use record::field::{FieldType, FieldValue, MemoFileType, MemoReader};
use record::RecordFieldInfo;
use std::convert::TryFrom;

/// Value of the byte between the last RecordFieldInfo and the first record
pub(crate) const TERMINATOR_VALUE: u8 = 0x0D;

/// Type definition of a record.
/// A .dbf file is composed of many records
pub type Record = HashMap<String, FieldValue>;

impl ReadableRecord for Record {
    fn read_using<'a, T, I>(mut field_iterator: FieldIterator<'a, T, I>) -> Result<Self, Error>
        where T: Read + Seek + 'a,
              I: Iterator<Item=&'a RecordFieldInfo> {
        let mut record = Self::new();
        while let Some(result) = field_iterator.read_next_field() {
            let (name, value) = result?;
            record.insert(name.to_owned(), value);
        }
        Ok(record)
    }
}
/// Struct with the handle to the source .dbf file
/// Responsible for reading the content
pub struct Reader<T: Read + Seek> {
    /// Where the data is read from
    source: T,
    memo_reader: Option<MemoReader<T>>,
    header: Header,
    fields_info: Vec<RecordFieldInfo>,
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
        for _ in 0..num_fields {
            let info = RecordFieldInfo::read_from(&mut source)?;
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
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn iter_records_as<R: ReadableRecord>(&mut self) -> RecordIterator<T, R> {
        RecordIterator {
            reader: self,
            record_type: std::marker::PhantomData,
            current_record: 0
        }
    }

    pub fn iter_records(&mut self) -> RecordIterator<T, Record> {
        self.iter_records_as::<Record>()
    }

    pub fn read_as<R: ReadableRecord>(&mut self) -> Result<Vec<R>, Error> {
        // We don't read the file terminator
        self.iter_records_as::<R>().collect::<Result<Vec<R>, Error>>()
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
    pub fn read(mut self) -> Result<Vec<Record>, Error> {
        // We don't read the file terminator
        self.iter_records().collect::<Result<Vec<Record>, Error>>()
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
        let at_least_one_field_is_memo = reader
            .fields_info
            .iter()
            .any(|f_info| f_info.field_type == FieldType::Memo);

        let memo_type = reader.header.file_type.supported_memo_type();
        if at_least_one_field_is_memo && memo_type.is_some() {
            let memo_path = match memo_type.unwrap() {
                MemoFileType::DbaseMemo | MemoFileType::DbaseMemo4 => p.with_extension("dbt"),
                MemoFileType::FoxBaseMemo => p.with_extension("fpt")
            };

            let memo_file = match File::open(memo_path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(Error::ErrorOpeningMemoFile(err));
                }
            };

            let memo_reader = MemoReader::new(memo_type.unwrap(), BufReader::new(memo_file))?;
            reader.memo_reader = Some(memo_reader);
        }
        Ok(reader)
    }
}


pub struct FieldIterator<'a, T: Read + Seek + 'a, I: Iterator<Item=&'a RecordFieldInfo>> {
    source: &'a mut T,
    fields_info: I,
    memo_reader: &'a mut Option<MemoReader<T>>,
}

impl<'a, T: Read + Seek + 'a, I: Iterator<Item=&'a RecordFieldInfo>> FieldIterator<'a, T, I> {
    pub fn read_next_field(&mut self) -> Option<Result<(&str, FieldValue), Error>> {
        let field_info = self.fields_info.next()?;
        let value = match FieldValue::read_from(self.source, self.memo_reader, field_info) {
            Err(e) => return Some(Err(e)),
            Ok(value) => value
        };
        if field_info.name == "DeletionFlag" {
            self.read_next_field()
        } else {
            Some(Ok((&field_info.name, value)))
        }
    }

    pub fn read_next_field_as<F>(&mut self) -> Option<Result<(&str, F), Error>>
        where F: TryFrom<FieldValue>,
              <F as TryFrom<FieldValue>>::Error: Into<Error> {

        match self.read_next_field() {
            Some(Ok((name, value))) => {
                match F::try_from(value) {
                    Err(e) => Some(Err(e.into())),
                    Ok(v) => Some(Ok((name, v)))
                }
            },
            Some(Err(e)) => Some(Err(e)),
            None => None
        }
    }
}

pub struct RecordIterator<'a, T: Read + Seek, R: ReadableRecord> {
    reader: &'a mut Reader<T>,
    record_type: std::marker::PhantomData<R>,
    current_record: u32,
}

impl<'a, T: Read + Seek, R: ReadableRecord> Iterator for RecordIterator<'a, T, R,> {
    type Item = Result<R, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_record >= self.reader.header.num_records {
            None
        } else {
            let iter = FieldIterator{
                source: &mut self.reader.source,
                fields_info: self.reader.fields_info.iter(),
                memo_reader: &mut None
            };
            self.current_record += 1;
            Some(R::read_using(iter))
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
    use std::fs::File;
    use std::io::{Seek, SeekFrom};

    use super::*;

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

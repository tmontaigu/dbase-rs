//! Module with the definition of fn's and struct's to read .dbf files

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::iter::FusedIterator;
use std::path::Path;

use byteorder::ReadBytesExt;

use Error;
use header::Header;
use record::field::{FieldType, FieldValue, MemoFileType, MemoReader};
use record::FieldInfo;
use std::collections::hash_map::RandomState;

/// Value of the byte between the last RecordFieldInfo and the first record
pub(crate) const TERMINATOR_VALUE: u8 = 0x0D;

const BACKLINK_SIZE: u16 = 263;

/// Trait to be implemented by structs that represent records read from a
/// dBase file.
///
/// The field iterator gives access to methods that allow to read fields value
/// or skip them.
/// It is not required that the user reads / skips all the fields in a record,
/// in other words: it is not required to consume the iterator.
pub trait ReadableRecord: Sized {
    fn read_using<'a, 'b, T>(field_iterator: &'a mut FieldIterator<'b, T>) -> Result<Self, Error>
        where T: Read + Seek;
}

/// Type definition of a generic record.
/// A .dbf file is composed of many records
#[derive(Debug, PartialEq)]
pub struct Record {
   map: HashMap<String, FieldValue>
}

impl ReadableRecord for Record {
    fn read_using<T>(field_iterator: &mut FieldIterator<T>) -> Result<Self, Error>
        where T: Read + Seek
    {
        let mut map = HashMap::<String, FieldValue>::new();
        for result in field_iterator {
            let NamedValue { name, value } = result?;
            map.insert(name.to_owned(), value);
        }
        Ok(Self {
            map
        })
    }
}

impl Record {
    pub fn insert(&mut self, field_name: String, value: FieldValue) -> Option<FieldValue>{
        self.map.insert(field_name, value)
    }

    pub fn get(&self, field_name: &str) -> Option<&FieldValue> {
        self.map.get(field_name)
    }

    pub fn get_mut(&mut self, field_name: &str) -> Option<&mut FieldValue> {
        self.map.get_mut(field_name)
    }

    pub fn remove(&mut self, field_name: &str) -> Option<FieldValue> {
        self.map.remove(field_name)
    }
}

impl IntoIterator for Record {
    type Item = (String, FieldValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, FieldValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            map: Default::default()
        }
    }
}

impl From<HashMap<String, FieldValue>> for Record {
    fn from(map: HashMap<String, FieldValue, RandomState>) -> Self {
        Self {
            map
        }
    }
}

/// Struct with the handle to the source .dbf file
/// Responsible for reading the content
pub struct Reader<T: Read + Seek> {
    /// Where the data is read from
    source: T,
    memo_reader: Option<MemoReader<T>>,
    pub(crate) header: Header,
    pub(crate) fields_info: Vec<FieldInfo>,
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
            header.offset_to_first_record - BACKLINK_SIZE
        } else {
            header.offset_to_first_record
        };
        let num_fields =
            (offset_to_first_record as usize - Header::SIZE - std::mem::size_of::<u8>())
                / FieldInfo::SIZE;


        let mut fields_info = Vec::<FieldInfo>::with_capacity(num_fields as usize + 1);
        fields_info.push(FieldInfo::new_deletion_flag());
        for _ in 0..num_fields {
            let info = FieldInfo::read_from(&mut source)?;
            fields_info.push(info);
        }

        let terminator = source.read_u8()?;
        debug_assert_eq!(terminator, TERMINATOR_VALUE);

        if header.file_type.is_visual_fox_pro() {
            source.seek(SeekFrom::Current(i64::from(BACKLINK_SIZE)))?;
        }

        Ok(Self {
            source,
            memo_reader: None,
            header,
            fields_info,
        })
    }

    /// Returns the header of the file
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Creates an iterator of records of the type you want
    pub fn iter_records_as<R: ReadableRecord>(&mut self) -> RecordIterator<T, R> {
        RecordIterator {
            reader: self,
            record_type: std::marker::PhantomData,
            current_record: 0,
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
    /// let mut reader = dbase::Reader::new(f).unwrap();
    /// let records = reader.read().unwrap();
    /// assert_eq!(records.len(), 1);
    /// ```
    pub fn read(&mut self) -> Result<Vec<Record>, Error> {
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


/// Simple struct to wrap together the value with the name
/// of the field it belongs to
pub struct NamedValue<'a, T> {
    pub name: &'a str,
    pub value: T,
}

/// Iterator over the fields in a dBase record
///
/// This iterator only iterates over the fields contained in one record
/// and will keep returning [None] when the last field was read.
/// And will not go through the fields of the next record.
pub struct FieldIterator<'a, T: Read + Seek> {
    pub(crate) source: &'a mut T,
    pub(crate) fields_info: std::iter::Peekable<std::slice::Iter<'a, FieldInfo>>,
    pub(crate) memo_reader: &'a mut Option<MemoReader<T>>,
}

impl<'a, T: Read + Seek> FieldIterator<'a, T> {
    /// Reads the next field and returns its name and value
    ///
    /// If the "DeletionFlag" field is present in the file it won't be returned
    /// and instead go to the next field.
    pub fn read_next_field(&mut self) -> Result<NamedValue<'a, FieldValue>, Error> {
        let field_info = self.fields_info.next().ok_or(Error::EndOfRecord)?;
        if field_info.is_deletion_flag() {
            if let Err(e) = self.skip_field(field_info) {
                Err(e.into())
            } else {
                self.read_next_field()
            }
        } else {
            self.read_field(field_info)
        }
    }

    /// Reads the next field and tries to convert into the requested type
    /// using [TryFrom]
    ///
    /// If the "DeletionFlag" field is present in the file it won't be returned
    /// and instead go to the next field.
    pub fn read_next_field_as<F>(&mut self) -> Result<NamedValue<'a, F>, Error>
        where F: TryFrom<FieldValue>,
              <F as TryFrom<FieldValue>>::Error: Into<Error> {
        let field_value = self.read_next_field()?;
        match F::try_from(field_value.value) {
            Ok(v) => Ok(NamedValue{
                name: field_value.name,
                value: v
            }),
            Err(e) => Err(e.into())
        }
    }


    /// Skips the next field of the record, useful if the field does not interest you
    /// but the ones after do.
    ///
    /// Does nothing if the last field of the record was already skipped or read.
    pub fn skip_next_field(&mut self) -> std::io::Result<()> {
        match self.fields_info.next() {
            None => Ok(()),
            Some(field_info) => self.skip_field(field_info)
        }
    }

    /// Skips all the remaining field of the record
    ///
    /// used internally to make sure the data stream is at the right position
    /// when we will start reading the next record
    ///
    /// Does nothing if the last field of the record was already skipped or read.
    fn skip_remaining_fields(&mut self) -> std::io::Result<()> {
        while let Some(field_info) = self.fields_info.next() {
            self.skip_field(field_info)?;
        }
        Ok(())
    }

    /// Reads the raw bytes of the next field without doing any filtering or trimming
    #[cfg(feature = "serde")]
    pub(crate) fn read_next_field_raw(&mut self) -> Result<Vec<u8>, Error> {
        let field_info = self.fields_info.next().ok_or(Error::EndOfRecord)?;
        if field_info.is_deletion_flag() {
            self.skip_next_field()?;
            self.read_next_field_raw()
        } else {
            let mut buf = vec![0u8; field_info.field_length as usize];
            self.source.read_exact(&mut buf)?;
            Ok(buf)
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn peek_next_field(&mut self) -> Result<NamedValue<'a, FieldValue>, Error> {
        let mut field_info = *self.fields_info.peek().ok_or(Error::EndOfRecord)?;
        if field_info.is_deletion_flag() {
            self.skip_field(field_info)?;
            self.fields_info.next().unwrap();
            field_info = self.fields_info.peek().ok_or(Error::EndOfRecord)?;
        }
        let value = self.read_field(field_info)?;
        self.source.seek(SeekFrom::Current(-i64::from(field_info.field_length)))?;
        Ok(value)
    }

    /// Advance the source to skip the field
    fn skip_field(&mut self, field_info: &FieldInfo) -> std::io::Result<()> {
        self.source.seek(SeekFrom::Current(i64::from(field_info.field_length)))?;
        Ok(())
    }

    /// read the next field using the given info
    fn read_field(&mut self, field_info: &'a FieldInfo) -> Result<NamedValue<'a, FieldValue>, Error> {
        let value = FieldValue::read_from(self.source, self.memo_reader, field_info)?;
        Ok(NamedValue { name: &field_info.name, value })
    }
}

impl<'a, T: Read + Seek> Iterator for FieldIterator<'a, T> {
    type Item = Result<NamedValue<'a, FieldValue>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_field() {
            Err(Error::EndOfRecord) => None,
            Ok(field_value) => Some(Ok(field_value)),
            Err(e) => Some(Err(e))
        }
    }
}

impl<'a, T: Read + Seek> FusedIterator for FieldIterator<'a, T> {}


/// Iterator over records contained in the dBase
pub struct RecordIterator<'a, T: Read + Seek, R: ReadableRecord> {
    reader: &'a mut Reader<T>,
    record_type: std::marker::PhantomData<R>,
    current_record: u32,
}

impl<'a, T: Read + Seek, R: ReadableRecord> Iterator for RecordIterator<'a, T, R, > {
    type Item = Result<R, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_record >= self.reader.header.num_records {
            None
        } else {
            let mut iter = FieldIterator {
                source: &mut self.reader.source,
                fields_info: self.reader.fields_info.iter().peekable(),
                memo_reader: &mut None,
            };
            let record = Some(R::read_using(&mut iter));

            if let Err(e) = iter.skip_remaining_fields() {
                Some(Err(e.into()))
            } else {
                self.current_record += 1;
                record
            }
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
    let mut reader = Reader::from_path(path)?;
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
            Header::SIZE + ((reader.fields_info.len() - 1) * FieldInfo::SIZE);
        // Add the terminator
        expected_pos += std::mem::size_of::<u8>();
        assert_eq!(pos_after_reading, expected_pos as u64);
    }
}

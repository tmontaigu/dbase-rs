//! Module with the definition of fn's and struct's to read .dbf files

use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::iter::FusedIterator;
use std::path::Path;

use crate::encoding::DynEncoding;
use crate::error::{Error, ErrorKind, FieldIOError};
use crate::field::types::{FieldType, FieldValue};
use crate::field::{DeletionFlag, FieldInfo};
use crate::header::Header;
use crate::memo::{MemoFileType, MemoReader};
use crate::{Encoding, FieldConversionError, Record};

/// Value of the byte between the last RecordFieldInfo and the first record
pub(crate) const TERMINATOR_VALUE: u8 = 0x0D;

pub(crate) const BACKLINK_SIZE: u16 = 263;

/// Trait to be implemented by structs that represent records read from a
/// dBase file.
///
/// The field iterator gives access to methods that allow to read fields value
/// or skip them.
/// It is not required that the user reads / skips all the fields in a record,
/// in other words: it is not required to consume the iterator.
pub trait ReadableRecord: Sized {
    /// function to be implemented that returns a new instance of your type
    /// using values read from the `FieldIterator'
    fn read_using<Source, MemoSource>(
        field_iterator: &mut FieldIterator<Source, MemoSource>,
    ) -> Result<Self, FieldIOError>
    where
        Source: Read + Seek,
        MemoSource: Read + Seek;
}

/// Structs containing the information allowing to
/// create a new TableWriter which would write file
/// with the same record structure as another dbase file.
///
/// You can get this by using [Reader::into_table_info].
#[derive(Clone)]
pub struct TableInfo {
    pub(crate) header: Header,
    pub(crate) fields_info: Vec<FieldInfo>,
    pub(crate) encoding: DynEncoding,
}

/// Struct with the handle to the source .dbf file
/// Responsible for reading the content
// TODO Debug impl
#[derive(Clone)]
pub struct Reader<T: Read + Seek> {
    /// Where the data is read from
    source: T,
    memo_reader: Option<MemoReader<T>>,
    header: Header,
    fields_info: Vec<FieldInfo>,
    encoding: DynEncoding,
}

impl<T: Read + Seek> Reader<T> {
    /// Creates a new reader from the source.
    ///
    /// Reads the header and fields information as soon as its created.
    ///
    /// Creating a reader from a file path using the [from_path](struct.Reader.html#method.from_path) is the prefered
    /// way of doing it as it wraps the file in a BufReader for performance.
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> Result<(), dbase::Error> {
    /// let mut reader = dbase::Reader::from_path("tests/data/line.dbf")?;
    /// let records = reader.read()?;
    /// # Ok(())
    /// # }
    ///
    /// ```
    ///
    /// ```
    /// use std::fs::File;
    /// # fn main() -> Result<(), dbase::Error> {
    /// let f = File::open("tests/data/line.dbf").unwrap();
    /// let reader = dbase::Reader::new(f)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(source: T) -> Result<Self, Error> {
        let file = crate::File::open(source)?;
        Ok(Self {
            source: file.inner,
            memo_reader: None,
            header: file.header,
            fields_info: file.fields_info,
            encoding: file.encoding,
        })
    }

    /// Creates a new reader from the source and reads strings using the encoding provided.
    ///
    /// See [`Self::new`] for more information.
    pub fn new_with_encoding<E: Encoding + 'static>(source: T, encoding: E) -> Result<Self, Error> {
        let mut reader = Self::new(source)?;
        reader.set_encoding(encoding);
        Ok(reader)
    }

    pub fn set_encoding<E: Encoding + 'static>(&mut self, encoding: E) {
        self.encoding = DynEncoding::new(encoding);
    }

    /// Returns the header of the file
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Returns the fields contained in the opened file
    pub fn fields(&self) -> &[FieldInfo] {
        &self.fields_info
    }

    /// Creates an iterator of records of the type you want
    pub fn iter_records_as<R: ReadableRecord>(&mut self) -> RecordIterator<T, R> {
        let record_size: usize = self
            .fields_info
            .iter()
            .map(|i| i.field_length as usize)
            .sum();
        RecordIterator {
            reader: self,
            record_type: std::marker::PhantomData,
            current_record: 0,
            record_data_buffer: std::io::Cursor::new(vec![0u8; record_size]),
            field_data_buffer: [0u8; 255],
        }
    }

    /// Shortcut function to get an iterator over the [Records](struct.Record.html) in the file
    pub fn iter_records(&mut self) -> RecordIterator<T, Record> {
        self.iter_records_as::<Record>()
    }

    /// Reads all the records of the file inside a `Vec`
    pub fn read_as<R: ReadableRecord>(&mut self) -> Result<Vec<R>, Error> {
        // We don't read the file terminator
        self.iter_records_as::<R>()
            .collect::<Result<Vec<R>, Error>>()
    }

    /// Make the `Reader` read the [Records](struct.Record.html)
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// # fn main() -> Result<(), dbase::Error> {
    /// let mut reader = dbase::Reader::from_path("tests/data/line.dbf")?;
    /// let records = reader.read()?;
    /// assert_eq!(records.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn read(&mut self) -> Result<Vec<Record>, Error> {
        // We don't read the file terminator
        self.iter_records().collect::<Result<Vec<Record>, Error>>()
    }

    /// Seek to the start of the record at `index`
    pub fn seek(&mut self, index: usize) -> Result<(), Error> {
        let offset = self.header.offset_to_first_record as usize
            + (index * self.header.size_of_record as usize);
        self.source
            .seek(SeekFrom::Start(offset as u64))
            .map_err(|err| Error::io_error(err, 0))?;
        Ok(())
    }

    /// Consumes the reader, and returns the info that
    /// allow to create a writer that would write a file
    /// with the same structure.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), dbase::Error> {
    /// let mut reader = dbase::Reader::from_path("some_file.dbf")?;
    /// let records = reader.read()?;
    /// let table_info = reader.into_table_info();
    /// let writer_1 = dbase::TableWriterBuilder::from_table_info(table_info.clone())
    ///         .build_with_file_dest("new_file_1.dbf");
    ///
    /// let writer_2 = dbase::TableWriterBuilder::from_table_info(table_info)
    ///         .build_with_file_dest("new_file_2.dbf");
    /// # Ok(())
    /// # }
    ///
    /// ```
    pub fn into_table_info(self) -> TableInfo {
        TableInfo {
            header: self.header,
            fields_info: self.fields_info,
            encoding: self.encoding,
        }
    }
}

impl Reader<BufReader<File>> {
    /// Creates a new dbase Reader from a path
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> Result<(), dbase::Error> {
    /// let reader = dbase::Reader::from_path("tests/data/line.dbf")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let p = path.as_ref().to_owned();
        let bufreader =
            BufReader::new(File::open(path).map_err(|error| Error::io_error(error, 0))?);
        let mut reader = Reader::new(bufreader)?;
        let at_least_one_field_is_memo = reader
            .fields_info
            .iter()
            .any(|f_info| f_info.field_type == FieldType::Memo);

        if at_least_one_field_is_memo {
            let memo_type = dbg!(reader.header.file_type.supported_memo_type());
            if let Some(mt) = memo_type {
                let memo_path = match mt {
                    MemoFileType::DbaseMemo | MemoFileType::DbaseMemo4 => p.with_extension("dbt"),
                    MemoFileType::FoxBaseMemo => p.with_extension("fpt"),
                };

                let memo_file = File::open(memo_path).map_err(|error| Error {
                    record_num: 0,
                    field: None,
                    kind: ErrorKind::ErrorOpeningMemoFile(error),
                })?;

                let memo_reader = MemoReader::new(mt, BufReader::new(memo_file))
                    .map_err(|error| Error::io_error(error, 0))?;
                reader.memo_reader = Some(memo_reader);
            }
        }
        Ok(reader)
    }

    /// Creates a new dbase Reader from a path and reads string using the encoding provided.
    pub fn from_path_with_encoding<P: AsRef<Path>, E: Encoding + 'static>(
        path: P,
        encoding: E,
    ) -> Result<Self, Error> {
        let mut reader = Self::from_path(path)?;
        reader.encoding = DynEncoding::new(encoding);
        Ok(reader)
    }
}

/// Simple struct to wrap together the value with the name
/// of the field it belongs to
pub struct NamedValue<'a, T> {
    /// Reference to the field name the value belongs to
    pub name: &'a str,
    /// The value
    pub value: T,
}

/// Iterator over the fields in a dBase record
///
/// This iterator only iterates over the fields contained in one record.
///
/// When trying to read more fields than there are, an EndOfRecord error
/// will be returned.
pub struct FieldIterator<'a, Source: Read + Seek, MemoSource: Read + Seek> {
    /// The source from where we read the data
    pub(crate) source: &'a mut Source,
    /// The fields that make the records
    pub(crate) fields_info: std::iter::Peekable<std::slice::Iter<'a, FieldInfo>>,
    /// The source where the Memo field data is read
    pub(crate) memo_reader: &'a mut Option<MemoReader<MemoSource>>,
    /// Buffer where field data is stored
    pub(crate) field_data_buffer: &'a mut [u8; 255],
    /// The string encoding
    pub(crate) encoding: &'a DynEncoding,
}

impl<'a, Source: Read + Seek, MemoSource: Read + Seek> FieldIterator<'a, Source, MemoSource> {
    /// Reads the next field and returns its name and value
    pub fn read_next_field_impl(&mut self) -> Result<(&'a FieldInfo, FieldValue), FieldIOError> {
        let field_info = self
            .fields_info
            .next()
            .ok_or_else(FieldIOError::end_of_record)?;
        Ok((field_info, self.read_field(field_info)?))
    }

    /// Reads the next field and returns its name and value
    pub fn read_next_field(&mut self) -> Result<NamedValue<'a, FieldValue>, FieldIOError> {
        self.read_next_field_impl()
            .map(|(field_info, field_value)| NamedValue {
                name: field_info.name(),
                value: field_value,
            })
    }

    /// Reads the next field and tries to convert into the requested type
    /// using [TryFrom]
    pub fn read_next_field_as<F>(&mut self) -> Result<NamedValue<'a, F>, FieldIOError>
    where
        F: TryFrom<FieldValue, Error = FieldConversionError>,
    {
        self.read_next_field_impl()
            .and_then(|(field_info, field_value)| match F::try_from(field_value) {
                Ok(v) => Ok(NamedValue {
                    name: field_info.name(),
                    value: v,
                }),
                Err(e) => Err(FieldIOError::new(e.into(), Some(field_info.to_owned()))),
            })
    }

    /// Skips the next field of the record, useful if the field does not interest you
    /// but the ones after do.
    ///
    /// Does nothing if the last field of the record was already skipped or read.
    pub fn skip_next_field(&mut self) -> Result<(), FieldIOError> {
        match self.fields_info.next() {
            None => Ok(()),
            Some(field_info) => self.skip_field(field_info),
        }
    }

    /// Skips all the remaining field of the record
    ///
    /// used internally to make sure the data stream is at the right position
    /// when we will start reading the next record
    ///
    /// Does nothing if the last field of the record was already skipped or read.
    fn skip_remaining_fields(&mut self) -> Result<(), FieldIOError> {
        while let Some(field_info) = self.fields_info.next() {
            self.skip_field(field_info)?;
        }
        Ok(())
    }

    /// Reads the raw bytes of the next field without doing any filtering or trimming
    #[cfg(feature = "serde")]
    pub(crate) fn read_next_field_raw(&mut self) -> Result<Vec<u8>, FieldIOError> {
        let field_info = self
            .fields_info
            .next()
            .ok_or(FieldIOError::end_of_record())?;
        let mut buf = vec![0u8; field_info.field_length as usize];
        self.source.read_exact(&mut buf).map_err(|error| {
            FieldIOError::new(ErrorKind::IoError(error), Some(field_info.to_owned()))
        })?;
        Ok(buf)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn peek_next_field(&mut self) -> Result<NamedValue<'a, FieldValue>, FieldIOError> {
        let field_info = *self.fields_info.peek().ok_or(FieldIOError {
            field: None,
            kind: ErrorKind::EndOfRecord,
        })?;
        let value = self.read_field(field_info)?;
        self.source
            .seek(SeekFrom::Current(-i64::from(field_info.field_length)))
            .map_err(|error| {
                FieldIOError::new(ErrorKind::IoError(error), Some(field_info.to_owned()))
            })?;

        Ok(NamedValue {
            name: field_info.name(),
            value,
        })
    }

    /// Advance the source to skip the field
    fn skip_field(&mut self, field_info: &FieldInfo) -> Result<(), FieldIOError> {
        self.source
            .seek(SeekFrom::Current(i64::from(field_info.field_length)))
            .map_err(|error| {
                FieldIOError::new(ErrorKind::IoError(error), Some(field_info.to_owned()))
            })?;
        Ok(())
    }

    /// read the next field using the given info
    fn read_field(&mut self, field_info: &'a FieldInfo) -> Result<FieldValue, FieldIOError> {
        let field_data_buffer = &mut self.field_data_buffer[..field_info.length() as usize];
        self.source.read_exact(field_data_buffer).unwrap();
        match FieldValue::read_from(
            field_data_buffer,
            self.memo_reader,
            field_info,
            &*self.encoding,
        ) {
            Ok(value) => Ok(value),
            Err(kind) => Err(FieldIOError {
                field: Some(field_info.clone()),
                kind,
            }),
        }
    }
}

impl<'a, Source: Read + Seek, MemoSource: Read + Seek> Iterator
    for FieldIterator<'a, Source, MemoSource>
{
    type Item = Result<NamedValue<'a, FieldValue>, FieldIOError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_field() {
            Err(error) => match error.kind() {
                ErrorKind::EndOfRecord => None,
                _ => Some(Err(error)),
            },
            Ok(field_value) => Some(Ok(field_value)),
        }
    }
}

impl<'a, Source: Read + Seek, MemoSource: Read + Seek> FusedIterator
    for FieldIterator<'a, Source, MemoSource>
{
}

/// Iterator over records contained in the dBase
pub struct RecordIterator<'a, T: Read + Seek, R: ReadableRecord> {
    reader: &'a mut Reader<T>,
    record_type: std::marker::PhantomData<R>,
    current_record: u32,
    record_data_buffer: std::io::Cursor<Vec<u8>>,
    /// Non-Memo field length is stored on a u8,
    /// so fields cannot exceed 255 bytes
    field_data_buffer: [u8; 255],
}

impl<'a, T: Read + Seek, R: ReadableRecord> Iterator for RecordIterator<'a, T, R> {
    type Item = Result<R, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_record >= self.reader.header.num_records {
            None
        } else {
            let deletion_flag = DeletionFlag::read_from(&mut self.reader.source).ok()?;

            if deletion_flag == DeletionFlag::Deleted {
                self.reader
                    .source
                    .seek(SeekFrom::Current(
                        self.record_data_buffer.get_ref().len() as i64
                    ))
                    .ok()?;
                return self.next();
            }

            self.reader
                .source
                .read_exact(self.record_data_buffer.get_mut())
                .ok()?;
            self.record_data_buffer.set_position(0);

            let mut iter = FieldIterator {
                source: &mut self.record_data_buffer,
                fields_info: self.reader.fields_info.iter().peekable(),
                memo_reader: &mut self.reader.memo_reader,
                field_data_buffer: &mut self.field_data_buffer,
                encoding: &self.reader.encoding,
            };

            let record = R::read_using(&mut iter)
                .and_then(|record| iter.skip_remaining_fields().and(Ok(record)))
                .map_err(|error| Error::new(error, self.current_record as usize));
            self.current_record += 1;
            Some(record)
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
    let mut reader = Reader::from_path(path).unwrap();
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

        let mut expected_pos = Header::SIZE + ((reader.fields_info.len()) * FieldInfo::SIZE);
        // Don't forget terminator
        expected_pos += std::mem::size_of::<u8>();

        assert_eq!(pos_after_reading, expected_pos as u64);
    }
}

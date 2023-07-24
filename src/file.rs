use crate::encoding::DynEncoding;
use crate::field::types::TrimOption;
use crate::field::{DeletionFlag, DELETION_FLAG_SIZE};
use crate::header::Header;
use crate::reading::{ReadingOptions, BACKLINK_SIZE, TERMINATOR_VALUE};
use crate::writing::{write_header_parts, WritableAsDbaseField};
use crate::ErrorKind::UnsupportedCodePage;
use crate::{
    Error, ErrorKind, FieldConversionError, FieldIOError, FieldInfo, FieldIterator, FieldValue,
    FieldWriter, ReadableRecord, TableInfo, WritableRecord,
};
use byteorder::ReadBytesExt;
use std::fmt::{Debug, Formatter};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Index to a field in a record
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct FieldIndex(pub usize);

/// Index to a record in a dBase file
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct RecordIndex(pub usize);

/// 'reference' to a field in a dBase file.
///
/// - Allows to read the field content via [Self::read] or [Self::read_as]
/// - Allows to overwrite the field content via [Self::write]
pub struct FieldRef<'a, T> {
    file: &'a mut File<T>,
    record_index: RecordIndex,
    field_index: FieldIndex,
}

impl<'a, T> Debug for FieldRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldRef")
            .field("record_index", &self.record_index)
            .field("field_index", &self.field_index)
            .finish()
    }
}

impl<'a, T> FieldRef<'a, T> {
    fn position_in_source(&self) -> u64 {
        let record_position = self
            .file
            .header
            .record_position(self.record_index.0)
            .unwrap() as u64;
        let position_in_record = self.file.fields_info[..self.field_index.0]
            .iter()
            .map(|i| i.field_length as u64)
            .sum::<u64>();

        record_position + position_in_record
    }
}

impl<'a, T> FieldRef<'a, T>
where
    T: Seek,
{
    pub(crate) fn seek_to_beginning(&mut self) -> Result<u64, FieldIOError> {
        let field_info = &self.file.fields_info[self.field_index.0];

        self.file
            .inner
            .seek(SeekFrom::Start(self.position_in_source()))
            .map_err(|e| FieldIOError::new(ErrorKind::IoError(e), Some(field_info.clone())))
    }
}

impl<'a, T> FieldRef<'a, T>
where
    T: Seek + Read,
{
    /// Reads and returns the value
    pub fn read(&mut self) -> Result<FieldValue, Error> {
        self.seek_to_beginning()
            .map_err(|e| Error::new(e, self.record_index.0))?;

        let field_info = &self.file.fields_info[self.field_index.0];

        let buffer = &mut self.file.field_data_buffer[..field_info.field_length as usize];
        self.file.inner.read(buffer).map_err(|e| {
            Error::new(
                FieldIOError::new(ErrorKind::IoError(e), Some(field_info.clone())),
                self.record_index.0,
            )
        })?;

        FieldValue::read_from::<Cursor<Vec<u8>>, _>(
            &buffer,
            &mut None,
            field_info,
            &self.file.encoding,
            TrimOption::BeginEnd,
        )
        .map_err(|e| {
            Error::new(
                FieldIOError::new(e, Some(field_info.clone())),
                self.record_index.0,
            )
        })
    }

    /// Reads and returns the value converted to the requested type
    pub fn read_as<ValueType>(&mut self) -> Result<ValueType, Error>
    where
        ValueType: TryFrom<FieldValue, Error = FieldConversionError>,
    {
        let value = self.read()?;

        let converted_value = ValueType::try_from(value).map_err(|e| {
            let field_info = &self.file.fields_info[self.field_index.0];
            Error::new(
                FieldIOError::new(ErrorKind::BadConversion(e), Some(field_info.clone())),
                self.record_index.0,
            )
        })?;

        Ok(converted_value)
    }
}

impl<'a, T> FieldRef<'a, T>
where
    T: Seek + Write,
{
    /// Writes the value
    pub fn write<ValueType>(&mut self, value: &ValueType) -> Result<(), Error>
    where
        ValueType: WritableAsDbaseField,
    {
        self.seek_to_beginning()
            .map_err(|e| Error::new(e, self.record_index.0))?;

        let field_info = &self.file.fields_info[self.field_index.0];

        let buffer = &mut self.file.field_data_buffer[..field_info.field_length as usize];
        buffer.fill(0);
        let mut cursor = Cursor::new(buffer);
        value
            .write_as(field_info, &self.file.encoding, &mut cursor)
            .map_err(|e| {
                Error::new(
                    FieldIOError::new(e, Some(field_info.clone())),
                    self.record_index.0,
                )
            })?;

        let buffer = cursor.into_inner();
        self.file.inner.write_all(&buffer).map_err(|e| {
            Error::new(
                FieldIOError::new(ErrorKind::IoError(e), Some(field_info.clone())),
                self.record_index.0,
            )
        })?;

        Ok(())
    }
}

/// 'reference' to a record in a dBase file.
///
/// This can be used to read/write the whole record at once,
/// or select a particular field in the file [Self::field].
///
/// - Allows to read the field content via [Self::read] or [Self::read_as]
/// - Allows to overwrite the field content via [Self::write]
pub struct RecordRef<'a, T> {
    file: &'a mut File<T>,
    index: RecordIndex,
}

impl<'a, T> Debug for RecordRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordRef")
            .field("index", &self.index)
            .finish()
    }
}

impl<'a, T> RecordRef<'a, T> {
    pub fn field<'b>(&'b mut self, index: FieldIndex) -> Option<FieldRef<'b, T>> {
        if index.0 >= self.file.fields_info.len() {
            return None;
        }
        Some(FieldRef {
            file: self.file,
            record_index: self.index,
            field_index: index,
        })
    }

    fn position_in_source(&self) -> u64 {
        self.file.header.record_position(self.index.0).unwrap() as u64
    }
}

impl<'a, T> RecordRef<'a, T>
where
    T: Seek,
{
    pub fn seek_to_beginning(&mut self) -> Result<u64, FieldIOError> {
        self.file
            .inner
            .seek(SeekFrom::Start(self.position_in_source()))
            .map_err(|e| FieldIOError::new(ErrorKind::IoError(e), None))
    }

    pub fn seek_before_deletion_flag(&mut self) -> Result<u64, FieldIOError> {
        self.file
            .inner
            .seek(SeekFrom::Start(self.position_in_source() - 1))
            .map_err(|e| FieldIOError::new(ErrorKind::IoError(e), None))
    }
}

impl<'a, T> RecordRef<'a, T>
where
    T: Read + Seek,
{
    /// Returns the value of the special deletion flag
    ///
    /// - true -> the record is marked as deleted
    /// - false -> the record is **not** marked as deleted
    pub fn is_deleted(&mut self) -> Result<bool, Error> {
        let deletion_flag_pos = self.position_in_source() - DELETION_FLAG_SIZE as u64;
        self.file
            .inner
            .seek(SeekFrom::Start(deletion_flag_pos))
            .map_err(|error| Error::io_error(error, self.index.0))?;

        let deletion_flag = DeletionFlag::read_from(&mut self.file.inner)
            .map_err(|error| Error::io_error(error, self.index.0))?;

        Ok(deletion_flag == DeletionFlag::Deleted)
    }

    pub fn read(&mut self) -> Result<crate::Record, Error> {
        self.read_as()
    }

    pub fn read_as<R>(&mut self) -> Result<R, Error>
    where
        R: ReadableRecord,
    {
        self.seek_to_beginning()
            .map_err(|error| Error::new(error, self.index.0))?;

        let mut field_iterator = FieldIterator::<_, Cursor<Vec<u8>>> {
            source: &mut self.file.inner,
            fields_info: self.file.fields_info.iter().peekable(),
            memo_reader: &mut None,
            field_data_buffer: &mut self.file.field_data_buffer,
            encoding: &self.file.encoding,
            options: self.file.options,
        };

        R::read_using(&mut field_iterator).map_err(|error| Error::new(error, self.index.0))
    }
}

impl<'a, T> RecordRef<'a, T>
where
    T: Write + Seek,
{
    /// Writes the content of `record` ath the position
    /// pointed by `self`.
    pub fn write<R>(&mut self, record: &R) -> Result<(), Error>
    where
        R: WritableRecord,
    {
        self.seek_before_deletion_flag()
            .map_err(|error| Error::new(error, self.index.0))?;

        let mut field_writer = FieldWriter {
            dst: &mut self.file.inner,
            fields_info: self.file.fields_info.iter().peekable(),
            field_buffer: &mut Cursor::new(&mut self.file.field_data_buffer),
            encoding: &self.file.encoding,
        };

        field_writer
            .write_deletion_flag()
            .map_err(|error| Error::io_error(error, self.index.0))?;

        record
            .write_using(&mut field_writer)
            .map_err(|error| Error::new(error, self.index.0))
    }
}

/// Iterator over the records in a File
pub struct FileRecordIterator<'a, T> {
    file: &'a mut File<T>,
    current_record: RecordIndex,
}

impl<'a, T> FileRecordIterator<'a, T> {
    // To implement iterator we need the Iterator trait to make use of GATs
    // which is not the case, to iteration will have to use the while let Some() pattern
    pub fn next<'s>(&'s mut self) -> Option<RecordRef<'s, T>> {
        if self.current_record.0 >= self.file.header.num_records as usize {
            None
        } else {
            let r = RecordRef {
                file: &mut self.file,
                index: self.current_record,
            };
            self.current_record.0 += 1;
            Some(r)
        }
    }
}

/// Handle to a dBase File.
///
/// A `File`, allows to both read and write, it also
/// allows to do modifications on an existing file,
/// and enables to only read/modify parts of a file without
/// first having to fully read it.
///
/// # Example
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut file = dbase::File::open_read_only("tests/data/stations.dbf")?;
///
/// assert_eq!(file.num_records(), 6);
///
/// let name_idx = file.field_index("name").unwrap();
/// let marker_color_idx = file.field_index("marker-col").unwrap();
/// let marker_symbol_idx = file.field_index("marker-sym").unwrap();
///
/// // Test manually reading fields (not in correct order) to FieldValue
/// let mut rh = file.record(3).unwrap();
/// let marker_color = rh.field(marker_color_idx).unwrap().read()?;
/// assert_eq!(
///    marker_color,
///    dbase::FieldValue::Character(Some("#ff0000".to_string()))
/// );
/// let name = rh.field(name_idx).unwrap().read()?;
/// assert_eq!(
///    name,
///    dbase::FieldValue::Character(Some("Judiciary Sq".to_string()))
/// );
/// let marker_symbol = rh.field(marker_symbol_idx).unwrap().read()?;
/// assert_eq!(
///    marker_symbol,
///    dbase::FieldValue::Character(Some("rail-metro".to_string()))
/// );
/// # Ok(())
/// # }
/// ```
pub struct File<T> {
    pub(crate) inner: T,
    pub(crate) header: Header,
    pub(crate) fields_info: Vec<FieldInfo>,
    pub(crate) encoding: DynEncoding,
    /// Non-Memo field length is stored on a u8,
    /// so fields cannot exceed 255 bytes
    field_data_buffer: [u8; 255],
    pub(crate) options: ReadingOptions,
}

impl<T> File<T> {
    /// Returns the information about fields present in the records
    pub fn fields(&self) -> &[FieldInfo] {
        self.fields_info.as_slice()
    }

    /// Returns the field infex that corresponds to the given name
    pub fn field_index(&self, name: &str) -> Option<FieldIndex> {
        self.fields_info
            .iter()
            .position(|info| info.name.eq_ignore_ascii_case(name))
            .map(FieldIndex)
    }

    /// Returns the number of records in the file
    pub fn num_records(&self) -> usize {
        self.header.num_records as usize
    }

    pub fn set_options(&mut self, options: ReadingOptions) {
        self.options = options;
    }
}

impl<T: Read + Seek> File<T> {
    /// creates of File using source as the storate space.
    pub fn open(mut source: T) -> Result<Self, Error> {
        let header = Header::read_from(&mut source).map_err(|error| Error::io_error(error, 0))?;

        let offset = if header.file_type.is_visual_fox_pro() {
            if BACKLINK_SIZE > header.offset_to_first_record {
                panic!("Invalid file");
            }
            header.offset_to_first_record - BACKLINK_SIZE
        } else {
            header.offset_to_first_record
        };
        let num_fields =
            (offset as usize - Header::SIZE - std::mem::size_of::<u8>()) / FieldInfo::SIZE;

        let mut fields_info = Vec::<FieldInfo>::with_capacity(num_fields as usize);
        for _ in 0..num_fields {
            let info = FieldInfo::read_from(&mut source).map_err(|error| Error {
                record_num: 0,
                field: None,
                kind: error,
            })?;
            fields_info.push(info);
        }

        let terminator = source
            .read_u8()
            .map_err(|error| Error::io_error(error, 0))?;

        debug_assert_eq!(terminator, TERMINATOR_VALUE);

        source
            .seek(SeekFrom::Start(u64::from(header.offset_to_first_record)))
            .map_err(|error| Error::io_error(error, 0))?;

        let encoding = header.code_page_mark.to_encoding().ok_or_else(|| {
            let field_error = FieldIOError::new(UnsupportedCodePage(header.code_page_mark), None);
            Error::new(field_error, 0)
        })?;
        Ok(Self {
            inner: source,
            // memo_reader: None,
            header,
            fields_info,
            encoding,
            field_data_buffer: [0u8; 255],
            options: ReadingOptions::default(),
        })
    }

    /// Returns a reference to the record at the given index.
    ///
    /// Returns None if no record exist for the given index
    pub fn record(&mut self, index: usize) -> Option<RecordRef<'_, T>> {
        if index >= self.header.num_records as usize {
            None
        } else {
            Some(RecordRef {
                file: self,
                index: RecordIndex(index),
            })
        }
    }

    /// Returns an iterator over the records in the file.
    ///
    /// Always starts at the first record
    pub fn records(&mut self) -> FileRecordIterator<'_, T> {
        FileRecordIterator {
            file: self,
            current_record: RecordIndex(0),
        }
    }
}

impl<T: Write + Seek> File<T> {
    pub fn create_new(mut dst: T, table_info: TableInfo) -> Result<Self, Error> {
        write_header_parts(&mut dst, &table_info.header, &table_info.fields_info)?;

        Ok(Self {
            inner: dst,
            header: table_info.header,
            fields_info: table_info.fields_info,
            encoding: table_info.encoding,
            field_data_buffer: [0u8; 255],
            options: ReadingOptions::default(),
        })
    }

    pub fn append_record<R>(&mut self, record: &R) -> Result<(), Error>
    where
        R: WritableRecord,
    {
        self.append_records(std::slice::from_ref(record))
    }

    pub fn append_records<R>(&mut self, records: &[R]) -> Result<(), Error>
    where
        R: WritableRecord,
    {
        assert_eq!(
            self.header
                .num_records
                .overflowing_add(records.len() as u32)
                .1,
            false,
            "Too many records (u32 overflow)"
        );

        let end_of_last_record = self.header.offset_to_first_record as u64
            + self.num_records() as u64
                * (DELETION_FLAG_SIZE as u64 + self.header.size_of_record as u64);
        self.inner
            .seek(SeekFrom::Start(end_of_last_record))
            .map_err(|error| Error::io_error(error, self.num_records()))?;

        for record in records {
            let current_record_index = self.header.num_records + 1;

            let mut field_writer = FieldWriter {
                dst: &mut self.inner,
                fields_info: self.fields_info.iter().peekable(),
                field_buffer: &mut Cursor::new(&mut self.field_data_buffer),
                encoding: &self.encoding,
            };

            field_writer
                .write_deletion_flag()
                .map_err(|error| Error::io_error(error, current_record_index as usize))?;

            record
                .write_using(&mut field_writer)
                .map_err(|error| Error::new(error, current_record_index as usize))?;

            self.header.num_records = current_record_index;
        }

        self.sync_all()
            .map_err(|error| Error::io_error(error, self.num_records()))?;

        Ok(())
    }

    pub fn sync_all(&mut self) -> std::io::Result<()> {
        let current_pos = self.inner.seek(SeekFrom::Current(0))?;
        self.inner.seek(SeekFrom::Start(0))?;
        self.header.write_to(&mut self.inner)?;
        self.inner.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }
}

impl File<std::fs::File> {
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        options: std::fs::OpenOptions,
    ) -> Result<Self, Error> {
        let file = options
            .open(path)
            .map_err(|error| Error::io_error(error, 0))?;
        File::open(file)
    }

    /// Opens an existing dBase file in read only mode
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = std::fs::File::open(path).map_err(|error| Error::io_error(error, 0))?;

        File::open(file)
    }

    /// Opens an existing dBase file in write only mode
    pub fn open_write_only<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut options = std::fs::OpenOptions::new();
        options
            .read(false)
            .write(true)
            .create(false)
            .truncate(false);

        File::open_with_options(path, options)
    }

    /// Opens an existing dBase file in read **and** write mode
    pub fn open_read_write<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(false).truncate(false);

        File::open_with_options(path, options)
    }

    /// This function will create a file if it does not exist, and will truncate it if it does.
    pub fn create<P: AsRef<Path>>(path: P, table_info: TableInfo) -> Result<Self, Error> {
        let file = std::fs::File::create(path).map_err(|error| Error::io_error(error, 0))?;

        File::create_new(file, table_info)
    }
}

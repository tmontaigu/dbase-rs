use crate::encoding::DynEncoding;
use crate::field::{DeletionFlag, FieldsInfo, DELETION_FLAG_SIZE};
use crate::header::Header;
use crate::memo::MemoReader;
use crate::reading::{ReadingOptions, BACKLINK_SIZE, TERMINATOR_VALUE};
use crate::writing::{write_header_parts, WritableAsDbaseField};
use crate::ErrorKind::UnsupportedCodePage;
use crate::{
    Error, ErrorKind, FieldConversionError, FieldIOError, FieldInfo, FieldIterator, FieldValue,
    FieldWriter, ReadableRecord, TableInfo, WritableRecord,
};
use byteorder::ReadBytesExt;
use std::fmt::{Debug, Formatter};
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

// Workaround the absence of File::try_clone with WASM/WASI without penalizing the other platforms
#[cfg(target_family = "wasm")]
type SharedFile = std::sync::Arc<std::fs::File>;
#[cfg(not(target_family = "wasm"))]
type SharedFile = std::fs::File;

pub struct BufReadWriteFile {
    input: BufReader<SharedFile>,
    output: BufWriter<SharedFile>,
}

impl BufReadWriteFile {
    fn new(file: SharedFile) -> std::io::Result<Self> {
        #[cfg(target_family = "wasm")]
        let file_ = file.clone();
        #[cfg(not(target_family = "wasm"))]
        let file_ = file.try_clone()?;

        let input = BufReader::new(file_);
        let output = BufWriter::new(file);
        Ok(Self { input, output })
    }
}

impl Read for BufReadWriteFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(buf)
    }
}

impl Write for BufReadWriteFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

impl Seek for BufReadWriteFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.output.seek(pos)?;
        self.input.seek(pos)
    }
}

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

        record_position + self.position_in_record() as u64
    }

    fn position_in_record(&self) -> usize {
        self.file
            .fields_info
            .field_position_in_record(self.field_index.0)
            .expect("internal error: invalid field index")
    }
}

impl<'a, T> FieldRef<'a, T>
where
    T: Seek,
{
    fn seek_to_beginning(&mut self) -> Result<u64, FieldIOError> {
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
        self.file
            .ensure_record_has_been_read_into_buffer(self.record_index)?;

        let field_info = &self.file.fields_info[self.field_index.0];

        let start_pos = self.position_in_record();
        let field_bytes = &mut self.file.record_data_buffer.get_mut()
            [start_pos..start_pos + field_info.field_length as usize];

        FieldValue::read_from(
            field_bytes,
            &mut self.file.memo_reader,
            field_info,
            &self.file.encoding,
            self.file.options.character_trim,
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
        self.file.file_position = self
            .seek_to_beginning()
            .map_err(|e| Error::new(e, self.record_index.0))?;

        let field_info = &self.file.fields_info[self.field_index.0];

        let start_pos = self.position_in_record();
        let field_bytes = &mut self.file.record_data_buffer.get_mut()
            [start_pos..start_pos + field_info.field_length as usize];
        field_bytes.fill(0);

        // Note that since we modify the internal buffer, we don't need to re-read the
        // record / buffer, meaning if a user writes then reads it should get correct
        // value, and we did not re-read from file.
        let mut cursor = Cursor::new(field_bytes);
        value
            .write_as(field_info, &self.file.encoding, &mut cursor)
            .map_err(|e| {
                Error::new(
                    FieldIOError::new(e, Some(field_info.clone())),
                    self.record_index.0,
                )
            })?;

        let buffer = cursor.into_inner();

        self.file.inner.write_all(buffer).map_err(|e| {
            Error::new(
                FieldIOError::new(ErrorKind::IoError(e), Some(field_info.clone())),
                self.record_index.0,
            )
        })?;

        self.file.file_position += buffer.len() as u64;

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
    pub fn seek_before_deletion_flag(&mut self) -> Result<u64, FieldIOError> {
        self.file
            .inner
            .seek(SeekFrom::Start(self.position_in_source()))
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
        self.file
            .ensure_record_has_been_read_into_buffer(self.index)?;
        let deletion_flag = DeletionFlag::from_byte(self.file.record_data_buffer.get_ref()[0]);

        Ok(deletion_flag == DeletionFlag::Deleted)
    }

    /// reads a field from the record
    ///
    /// Shortcut for `.field(index).unwrap().read().unwrap();`
    pub fn read_field(&mut self, field_index: FieldIndex) -> Result<FieldValue, Error> {
        let record_index = self.index.0;
        let mut field = self
            .field(field_index)
            .ok_or_else(|| Error::new(FieldIOError::end_of_record(), record_index))?;
        field.read()
    }

    /// reads a field from the record
    ///
    /// Shortcut for `.field(index).unwrap().read_as().unwrap();`
    pub fn read_field_as<ValueType>(&mut self, field_index: FieldIndex) -> Result<ValueType, Error>
    where
        ValueType: TryFrom<FieldValue, Error = FieldConversionError>,
    {
        let record_index = self.index.0;
        let mut field = self
            .field(field_index)
            .ok_or_else(|| Error::new(FieldIOError::end_of_record(), record_index))?;
        field.read_as()
    }

    /// Reads the record
    pub fn read(&mut self) -> Result<crate::Record, Error> {
        self.read_as()
    }

    /// Reads the record as the given type
    pub fn read_as<R>(&mut self) -> Result<R, Error>
    where
        R: ReadableRecord,
    {
        self.file
            .ensure_record_has_been_read_into_buffer(self.index)?;
        self.file
            .record_data_buffer
            .set_position(DELETION_FLAG_SIZE as u64);
        let mut field_iterator = FieldIterator {
            source: &mut self.file.record_data_buffer,
            fields_info: self.file.fields_info.iter().peekable(),
            memo_reader: &mut self.file.memo_reader,
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
    /// writes a field to the record
    ///
    /// Shortcut for `.field(index).unwrap().write(&value).unwrap();`
    pub fn write_field<ValueType>(
        &mut self,
        field_index: FieldIndex,
        value: &ValueType,
    ) -> Result<(), Error>
    where
        ValueType: WritableAsDbaseField,
    {
        let record_index = self.index.0;
        let mut field = self
            .field(field_index)
            .ok_or_else(|| Error::new(FieldIOError::end_of_record(), record_index))?;
        field.write(value)
    }

    /// Writes the content of `record` ath the position
    /// pointed by `self`.
    pub fn write<R>(&mut self, record: &R) -> Result<(), Error>
    where
        R: WritableRecord,
    {
        self.file.record_data_buffer.get_mut().fill(0);
        self.file.record_data_buffer.get_mut()[0] = DeletionFlag::NotDeleted.to_byte();
        self.file.record_data_buffer.set_position(1);

        let mut field_writer = FieldWriter {
            dst: &mut self.file.record_data_buffer,
            fields_info: self.file.fields_info.iter().peekable(),
            field_buffer: &mut Cursor::new(&mut self.file.field_data_buffer),
            encoding: &self.file.encoding,
        };

        record
            .write_using(&mut field_writer)
            .map_err(|error| Error::new(error, self.index.0))?;

        self.seek_before_deletion_flag()
            .map_err(|error| Error::new(error, self.index.0))?;

        self.file
            .inner
            .write_all(self.file.record_data_buffer.get_ref())
            .map_err(|error| Error::io_error(error, self.index.0))?;

        // We don't need to update the file's inner position as we re-wrote the whole record
        debug_assert_eq!(
            self.file.file_position,
            self.file.inner.stream_position().unwrap()
        );

        Ok(())
    }
}

/// Iterator over the records in a File
pub struct FileRecordIterator<'a, T> {
    file: &'a mut File<T>,
    current_record: RecordIndex,
}

impl<'a, T> FileRecordIterator<'a, T>
where
    T: Seek + Read,
{
    // To implement iterator we need the Iterator trait to make use of GATs
    // which is not the case, to iteration will have to use the while let Some() pattern
    pub fn next<'s>(&'s mut self) -> Option<RecordRef<'s, T>> {
        let record_ref = self.file.record(self.current_record.0);
        if let Some(_) = record_ref {
            self.current_record.0 += 1
        }
        record_ref
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
/// assert_eq!(file.num_records(), 86);
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
    memo_reader: Option<MemoReader<T>>,
    pub(crate) header: Header,
    pub(crate) fields_info: FieldsInfo,
    pub(crate) encoding: DynEncoding,
    /// Buffer that contains a whole record worth of data
    /// It also contains the deletion flag
    record_data_buffer: Cursor<Vec<u8>>,
    /// Non-Memo field length is stored on a u8,
    /// so fields cannot exceed 255 bytes
    field_data_buffer: [u8; 255],
    pub(crate) options: ReadingOptions,
    /// We track the position in the file
    /// to avoid calling `seek` when we are reading buffer
    /// in order (0, 1, 2, etc)
    file_position: u64,
}

impl<T> File<T> {
    /// Returns the information about fields present in the records
    pub fn fields(&self) -> &[FieldInfo] {
        self.fields_info.as_ref()
    }

    /// Returns the field index that corresponds to the given name
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
    /// creates of File using source as the storage space.
    pub fn open(mut source: T) -> Result<Self, Error> {
        let mut header =
            Header::read_from(&mut source).map_err(|error| Error::io_error(error, 0))?;

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

        let fields_info =
            FieldsInfo::read_from(&mut source, num_fields).map_err(|error| Error {
                record_num: 0,
                field: None,
                kind: error,
            })?;

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

        let record_size: usize = DELETION_FLAG_SIZE + fields_info.size_of_all_fields();
        let record_data_buffer = Cursor::new(vec![0u8; record_size]);
        // Some file seems not to include the DELETION_FLAG_SIZE into the record size,
        // but we rely on it
        header.size_of_record = record_size as u16;
        // debug_assert_eq!(record_size - DELETION_FLAG_SIZE, header.size_of_record as usize);

        Ok(Self {
            inner: source,
            memo_reader: None,
            header,
            fields_info,
            encoding,
            record_data_buffer,
            field_data_buffer: [0u8; 255],
            options: ReadingOptions::default(),
            file_position: header.offset_to_first_record as u64,
        })
    }

    /// Returns a reference to the record at the given index.
    ///
    /// Returns None if no record exist for the given index
    pub fn record(&mut self, index: usize) -> Option<RecordRef<'_, T>> {
        if index >= self.header.num_records as usize {
            None
        } else {
            let record_ref = RecordRef {
                file: self,
                index: RecordIndex(index),
            };
            Some(record_ref)
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

    /// Returns true if it read from the source, false otherwise (used in tests).
    fn ensure_record_has_been_read_into_buffer(
        &mut self,
        record_index: RecordIndex,
    ) -> Result<bool, Error> {
        let record_ref = RecordRef {
            file: self,
            index: record_index,
        };
        let start_of_record_pos = record_ref.position_in_source();
        let end_of_record_pos = start_of_record_pos + u64::from(self.header.size_of_record);

        if self.file_position > start_of_record_pos && self.file_position <= end_of_record_pos {
            // If pos is in this range, then the record was already read into the buffer
            return Ok(false);
        }

        if start_of_record_pos != self.file_position {
            // Only call seek of the record we need to read
            // is the just after the one we read last
            self.file_position = self
                .inner
                .seek(SeekFrom::Start(start_of_record_pos))
                .map_err(|e| Error::io_error(e, record_index.0))?;
        }

        self.inner
            .read_exact(self.record_data_buffer.get_mut())
            .map_err(|e| Error::io_error(e, record_index.0))?;
        self.file_position += self.record_data_buffer.get_mut().len() as u64;
        Ok(true)
    }
}

impl<T: Write + Seek> File<T> {
    pub fn create_new(mut dst: T, table_info: TableInfo) -> Result<Self, Error> {
        write_header_parts(&mut dst, &table_info.header, &table_info.fields_info)?;
        let record_size: usize = DELETION_FLAG_SIZE
            + table_info
                .fields_info
                .iter()
                .map(|i| i.field_length as usize)
                .sum::<usize>();
        let record_data_buffer = Cursor::new(vec![0u8; record_size]);
        let file_position = table_info.header.offset_to_first_record as u64;
        debug_assert_eq!(file_position, dst.stream_position().unwrap());
        Ok(Self {
            inner: dst,
            memo_reader: None,
            header: table_info.header,
            fields_info: FieldsInfo {
                inner: table_info.fields_info,
            },
            encoding: table_info.encoding,
            record_data_buffer,
            field_data_buffer: [0u8; 255],
            options: ReadingOptions::default(),
            file_position,
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
            + (self.num_records() as u64 * self.header.size_of_record as u64);

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
        let current_pos = self.inner.stream_position()?;
        self.inner.seek(SeekFrom::Start(0))?;
        self.header.write_to(&mut self.inner)?;
        self.inner.seek(SeekFrom::Start(current_pos))?;
        Ok(())
    }
}

impl File<BufReadWriteFile> {
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        options: std::fs::OpenOptions,
    ) -> Result<Self, Error> {
        let file = options
            .open(path)
            .map_err(|error| Error::io_error(error, 0))?;
        let source = BufReadWriteFile::new(file.into()).unwrap();
        File::open(source)
    }

    /// Opens an existing dBase file in read only mode
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = std::fs::File::open(path.as_ref()).map_err(|error| Error::io_error(error, 0))?;

        let mut file = File::open(BufReadWriteFile::new(file.into()).unwrap())?;
        if file.fields_info.at_least_one_field_is_memo() {
            let p = path.as_ref();
            let memo_type = file.header.file_type.supported_memo_type();
            if let Some(mt) = memo_type {
                let memo_path = p.with_extension(mt.extension());

                let memo_file = std::fs::File::open(memo_path).map_err(|error| Error {
                    record_num: 0,
                    field: None,
                    kind: ErrorKind::ErrorOpeningMemoFile(error),
                })?;

                let memo_reader = BufReadWriteFile::new(memo_file.into())
                    .and_then(|memo_file| MemoReader::new(mt, memo_file))
                    .map_err(|error| Error::io_error(error, 0))?;

                file.memo_reader = Some(memo_reader);
            }
        }
        Ok(file)
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

        File::create_new(BufReadWriteFile::new(file.into()).unwrap(), table_info)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ensure_record_has_been_read_into_buffer() {
        let mut file = crate::File::open_read_only("tests/data/stations.dbf").unwrap();

        {
            let mut record = file.record(0).unwrap();
            let _ = record.read_field(crate::FieldIndex(0)).unwrap();
            // Must return false, meaning it correctly understands the record 0 is in memory
            assert!(!file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(0))
                .unwrap());
            assert!(file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(1))
                .unwrap());
        }

        {
            let mut record = file.record(4).unwrap();
            let _ = record.read_field(crate::FieldIndex(3)).unwrap();
            // Must return false, meaning it correctly understands the record 4 is in memory
            assert!(!file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(4))
                .unwrap());
            assert!(file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(1))
                .unwrap());
        }

        // Make sure writing a field still work with the ensure mechanism
        {
            let mut record = file.record(10).unwrap();
            let value = record.read_field(crate::FieldIndex(2)).unwrap();
            // Use record.file to avoid double borrow
            assert!(!record
                .file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(10))
                .unwrap());
            record.write_field(crate::FieldIndex(2), &value).unwrap();
            assert!(!file
                .ensure_record_has_been_read_into_buffer(crate::RecordIndex(10))
                .unwrap());
        }
    }
}

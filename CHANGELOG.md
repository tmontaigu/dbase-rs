# 0.6.0
    - Added support for wasm32-wasmi target
    - Added support for encoding_rs, to (notably) support GBK encoding
    - Added conversion of Date, Time, DateTime into chrono structs
    - Added `finalize` method, deprecate `close`
    - Added `BufReadWriteFile` to public API
    - Updated datafusion to version 46

# 0.5.0
    - Added `ReaderBuilder`
    - Fix off by one error in dbase::File
    - Improve performance of dbase::File
    - Datafusion now accounts for empty records
    - Datafusion properly escape special chars in memo files
# O.4.0
    - `Date` is now const-constructible
    - `dbase_record!` keeps visibility token
    - Added `File` struct to read/write file in-place.
    - Fixed stack overflow error on files with many sub-sequent deleted record.
    - Added `TrimOption`, to choose how whitespaces in Charater fields should be
    trimmed.

# 0.3.0
    - Replaced `chrono` with `time` v0.3
    - Added support for reading/writing non-unicode database files via custom encodings.
      The optional `yore` feature/crate can be used for supporting basic codepages.
      The `code_page_mark` contained in the header is used to create the correct decoder.
    - Fixed writing visual fox pro files
    - Added `FieldType` to publicly exported types

# 0.2.3
    - Added `impl std::error::Error for dbase::Error`
    - Fixed deserialization implementation that would fail to deserialize if one `None`
      value was encountered (issue #30, PR #31)

# 0.2.2
    - Fixed files written, their record size was wrong by one byte
    - Added the missing accessors for the `DateTime` and `Time` structs members

# 0.2.1
    - Implement `From<Record>` for `HashMap`
    - Implement `AsRef<HashMap>` & `AsMut<HashMap>` for `Record`
    - Add `derive(Clone)` for Record
    - Fix decimal places in writing numeric values
    - Performance improvement when reading (PR #21 and #23)

# 0.2.0
    - Added a `seek` method to the `Reader`
    - Added a `TableInfo` struct and a `into_table_info` method on the `Reader`.
      This `TableInfo` contains informations that can be used to create a new `TableWriterBuilder`
      that writes a dbf file with same 'layout' as the file from which the `TableInfo` comes from.
    - Added `TableWriterBuilder::from_table_info`
    - Changed `TableWriter::write` is now named `TableWriter::write_records` and takes any type that
      implements `IntoIterator<Item=&RecordType>` (so &[RecordType] is still a valid input).
    - Changed `TableWriter<T>` now requires `T` to implement `std::io::Seek and std::io::Read`,
      both `std::fs::File` & `std::io::Cursor` are example of valid `T`.
    - Added `TableWriter::write_record` to be able to write one record at a time.
    - Increased byteorder dependency from 1.3.0 to 1.4.3
    

# 0.1.2
    - Fixed some files not being properly read, by ensuring the reader seeks to the begining
      of the records after reading the header. (issue #11, Pull Request #12)

# 0.1.1
    - Added RecordIterator to the lib.rs exports
    - Added derive `Clone` to the `Reader`
    - Removed `pub` attribute from `FieldInfo`'s `name` struct member.

# 0.1.0
    - Added preliminary support for reading some 'VisualFoxPro' files
    - Added support for reading dBase / FoxPro files which have 'Memo' fields. (Writing Memo fields is not supported yet)
    - Added support for reading and writing the 'Datetime' field
    - Added support for reading and writing the 'Currency' field
    - Added Trait to allow users implementing it to read dBase records into their structs.
    - Added Trait to allow users implementing it to writre structs as dBase record.
    - Added optional feature "serde", to automatically impl the 2 trait described above
    - Added dependency to chrono
    
    - Changed how iteration on the records is made (/!\ very small breaking change)
    - Changed how dbase Writer are created, users now have to use the TableWriterBuilder
      to specify fields that constitute a record before being able to write records.
    - Changed the Error type.
      
    - Bumped byteorder dependency to 1.3.0

# 0.0.4
    - Added reading and writing of Float value #2
    - All dBase III types (Character, Numeric, Logical, Date, Float)
        are wrapped in Option<T> to handle empty / uninitialized values

# 0.0.3
    - Added writing of Character, Numeric, Logical, Date, Double, Integer

# 0.0.2
    - Added reading of Character, Numeric, Logical, Date, Double, Integer, types



#[macro_use]
extern crate dbase;

use std::io::{Cursor, Read, Seek, Write};

use dbase::{
    Date, DateTime, FieldIOError, FieldIterator, FieldName, FieldValue, FieldWriter,
    ReadableRecord, Reader, Record, TableWriterBuilder, Time, WritableRecord,
};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

const LINE_DBF: &str = "./tests/data/line.dbf";
const NONE_FLOAT_DBF: &str = "./tests/data/contain_none_float.dbf";
const NULL_PADDED_NUMERIC_DBF: &str = "./tests/data/contain_null_padded_numeric.dbf";
const STATIONS: &str = "./tests/data/stations.dbf";
const STATIONS_WITH_DELETED: &str = "./tests/data/stations_with_deleted.dbf";
#[cfg(feature = "yore")]
const CP850_DBF: &str = "tests/data/cp850.dbf";


const CP936_DBF: &str = "tests/data/cp936.dbf";

fn write_read_compare<R>(records: &Vec<R>, writer_builder: TableWriterBuilder)
where
    R: WritableRecord + ReadableRecord + Debug + PartialEq,
{
    let mut dst = Cursor::new(Vec::<u8>::new());
    let writer = writer_builder.build_with_dest(&mut dst);

    writer.write_records(records).unwrap();
    dst.set_position(0);

    let mut reader = Reader::new(dst).unwrap();
    let read_records = reader.read_as::<R>().unwrap();

    assert_eq!(&read_records, records);
}

#[test]
fn test_none_float() {
    let records = dbase::read(NONE_FLOAT_DBF).unwrap();
    assert_eq!(records.len(), 1);

    let mut expected_fields = Record::default();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("tralala".to_owned())),
    );
    expected_fields.insert("value_f".to_owned(), dbase::FieldValue::Float(Some(12.345)));
    expected_fields.insert("value_f_non".to_owned(), dbase::FieldValue::Float(None));
    expected_fields.insert("value_n".to_owned(), dbase::FieldValue::Numeric(Some(4.0)));
    expected_fields.insert("value_n_non".to_owned(), dbase::FieldValue::Numeric(None));

    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_simple_file() {
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    let mut expected_fields = Record::default();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("linestring1".to_owned())),
    );

    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_read_write_simple_file() {
    let mut expected_fields = Record::default();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("linestring1".to_owned())),
    );

    let mut reader = dbase::Reader::from_path(LINE_DBF).unwrap();
    let records = reader.read().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);

    let mut dst = Cursor::new(Vec::<u8>::new());
    let writer = TableWriterBuilder::from_reader(reader).build_with_dest(&mut dst);
    writer.write_records(&records).unwrap();
    dst.set_position(0);

    let mut reader = dbase::Reader::from_path(LINE_DBF).unwrap();
    let records = reader.read().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_read_numeric_value_null_padded() {
    let records = dbase::read(NULL_PADDED_NUMERIC_DBF).unwrap();
    assert_eq!(records.len(), 1);
    let mut expected_fields = Record::default();
    expected_fields.insert("number".to_owned(), dbase::FieldValue::Numeric(Some(1234.)));
    assert_eq!(records[0], expected_fields);
}

#[derive(Debug, PartialEq, Clone)]
struct Album {
    artist: String,
    name: String,
    released: dbase::Date,
    playtime: f64, // in seconds,
    available: bool,
}

impl ReadableRecord for Album {
    fn read_using<R1, R2>(field_iterator: &mut FieldIterator<R1, R2>) -> Result<Self, FieldIOError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        Ok(Self {
            artist: field_iterator.read_next_field_as()?.value,
            name: field_iterator.read_next_field_as()?.value,
            released: field_iterator.read_next_field_as()?.value,
            playtime: field_iterator.read_next_field_as()?.value,
            available: field_iterator.read_next_field_as()?.value,
        })
    }
}

impl WritableRecord for Album {
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldIOError> {
        field_writer.write_next_field_value(&self.artist)?;
        field_writer.write_next_field_value(&self.name)?;
        field_writer.write_next_field_value(&self.released)?;
        field_writer.write_next_field_value(&self.playtime)?;
        field_writer.write_next_field_value(&self.available)?;
        Ok(())
    }
}

#[test]
fn from_scratch_dbase() {
    let writer_builder = TableWriterBuilder::new()
        .add_character_field("Artist".try_into().unwrap(), 50)
        .add_character_field("Name".try_into().unwrap(), 50)
        .add_date_field("Released".try_into().unwrap())
        .add_numeric_field("Playtime".try_into().unwrap(), 10, 2)
        .add_logical_field(FieldName::try_from("Available").unwrap());

    let records = vec![
        Album {
            artist: "Fallujah".to_string(),
            name: "The Flesh Prevails".to_string(),
            released: dbase::Date::new(22, 6, 2014),
            playtime: 2481.12f64,
            available: false,
        },
        Album {
            artist: "Beyond Creation".to_string(),
            name: "Earthborn Evolution".to_string(),
            released: dbase::Date::new(24, 10, 2014),
            playtime: 24f64,
            available: true,
        },
    ];

    write_read_compare(&records, writer_builder);
}

#[test]
fn from_scratch_fox_pro_record() {
    let writer_builder = TableWriterBuilder::new()
        .add_integer_field(FieldName::try_from("integer").unwrap())
        .add_double_field(FieldName::try_from("double").unwrap())
        .add_currency_field(FieldName::try_from("currency").unwrap())
        .add_datetime_field(FieldName::try_from("datetime").unwrap());

    let mut record = Record::default();
    record.insert(String::from("integer"), FieldValue::Integer(17));
    record.insert(String::from("double"), FieldValue::Double(54621.154));
    record.insert(String::from("currency"), FieldValue::Currency(4567.134));
    record.insert(
        String::from("datetime"),
        FieldValue::DateTime(DateTime::new(
            Date::new(01, 06, 2006),
            Time::new(12, 50, 20),
        )),
    );

    let records = vec![record];
    write_read_compare(&records, writer_builder);
}

dbase_record! {
    #[derive(Clone, Debug, PartialEq)]
    struct FoxProRecord {
        datetime: DateTime,
        double: f64,
        currency: f64,
        integer: i32
    }
}

#[test]
fn from_scratch_fox_pro_struct_record() {
    let writer_builder = TableWriterBuilder::new()
        .add_datetime_field(FieldName::try_from("datetime").unwrap())
        .add_double_field(FieldName::try_from("double").unwrap())
        .add_currency_field(FieldName::try_from("currency").unwrap())
        .add_integer_field(FieldName::try_from("integer").unwrap());

    let records = vec![FoxProRecord {
        datetime: DateTime::new(Date::new(12, 02, 1999), Time::new(21, 20, 35)),
        double: 8649.48851,
        currency: 3489.9612314,
        integer: 42069,
    }];

    write_read_compare(&records, writer_builder);
}

dbase_record! {
    #[derive(Clone, Debug, PartialEq)]
    struct User {
        first_name: String,
        last_name: String,
    }
}

// We just test that this compiles
dbase_record! {
    struct TestStructWithoutDerive {
        this_should_compile: String
    }
}

#[test]
fn the_classical_user_record_example() {
    let users = vec![
        User {
            first_name: "Ferrys".to_string(),
            last_name: "Rust".to_string(),
        },
        User {
            first_name: "Alex".to_string(),
            last_name: "Rider".to_string(),
        },
        User {
            first_name: "Jamie".to_string(),
            last_name: "Oliver".to_string(),
        },
    ];

    let mut cursor = Cursor::new(Vec::<u8>::new());
    let writer = TableWriterBuilder::new()
        .add_character_field("First Name".try_into().unwrap(), 50)
        .add_character_field("Last Name".try_into().unwrap(), 50)
        .build_with_dest(&mut cursor);

    writer.write_records(&users).unwrap();
    cursor.set_position(0);

    let mut reader = Reader::new(cursor).unwrap();
    let read_records = reader.read_as::<User>().unwrap();

    assert_eq!(read_records, users);
}

#[cfg(feature = "yore")]
#[test]
fn non_unicode_codepages() {
    let mut reader =
        dbase::Reader::from_path_with_encoding(CP850_DBF, yore::code_pages::CP850).unwrap();
    let records = reader.read().unwrap();

    assert_eq!(
        records[0].get("TEXT"),
        Some(&dbase::FieldValue::Character(Some("Äöü!§$%&/".to_string())))
    );

    // Write back with same encoding
    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let writer = TableWriterBuilder::from_reader(reader).build_with_dest(&mut cursor);
        writer.write_records(&records).unwrap();
    }

    cursor.set_position(0);

    // Read again
    let mut reader = Reader::new_with_encoding(cursor, yore::code_pages::CP850).unwrap();
    let records = reader.read().unwrap();

    assert_eq!(
        records[0].get("TEXT"),
        Some(&dbase::FieldValue::Character(Some("Äöü!§$%&/".to_string())))
    );
}

dbase::dbase_record!(
    #[derive(PartialOrd, PartialEq, Debug)]
    struct StationRecord {
        name: String,
        marker_col: String,
        marker_sym: String,
        line: String,
    }
);

#[test]
fn test_char_trimming() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = dbase::Reader::from_path(STATIONS)?;
    let options = dbase::ReadingOptions::default().character_trim(dbase::TrimOption::Begin);

    reader.set_options(options);
    let records = reader.read_as::<StationRecord>()?;

    assert_eq!(records.len() as u32, reader.header().num_records);

    let expected = StationRecord {
        name: format!(
            "{:width$}",
            "Franconia-Springfield",
            width = reader.fields()[0].length() as usize
        ),
        marker_col: format!(
            "{:width$}",
            "#0000ff",
            width = reader.fields()[0].length() as usize
        ),
        marker_sym: format!(
            "{:width$}",
            "rail-metro",
            width = reader.fields()[0].length() as usize
        ),
        line: format!(
            "{:width$}",
            "blue",
            width = reader.fields()[0].length() as usize
        ),
    };
    assert_eq!(records[1], expected);

    Ok(())
}

#[test]
fn test_record_marked_as_deleted_are_skipped_by_reader() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = dbase::Reader::from_path(STATIONS_WITH_DELETED)?;
    let records = reader.read_as::<StationRecord>()?;

    // We have one deleted record in the file
    assert_eq!(records.len() as u32, reader.header().num_records - 1);

    let expected = StationRecord {
        name: "Franconia-Springfield".to_string(),
        marker_col: "#0000ff".to_string(),
        marker_sym: "rail-metro".to_string(),
        line: "blue".to_string(),
    };
    assert_eq!(records[0], expected);

    Ok(())
}


#[test]
fn test_codepages_cp936() {
    let mut reader =
        dbase::Reader::from_path(CP936_DBF).unwrap();
    let records = reader.read().unwrap();

    assert_eq!(
        records[0].get("TEST"),
        Some(&dbase::FieldValue::Character(Some("测试中文".to_string())))
    ); 
}

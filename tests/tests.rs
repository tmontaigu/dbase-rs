#[macro_use]
extern crate dbase;

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};

use dbase::{Error, TableWriterBuilder, FieldIterator, FieldValue, ReadableRecord, FieldInfo, WritableRecord, Reader};

const LINE_DBF: &str = "./tests/data/line.dbf";
const NONE_FLOAT_DBF: &str = "./tests/data/contain_none_float.dbf";

#[test]
fn test_none_float() {
    let records = dbase::read(NONE_FLOAT_DBF).unwrap();
    assert_eq!(records.len(), 1);

    let mut expected_fields = HashMap::new();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("tralala".to_owned())),
    );
    expected_fields.insert(
        "value_f".to_owned(),
        dbase::FieldValue::Float(Some(12.345)),
    );
    expected_fields.insert(
        "value_f_non".to_owned(),
        dbase::FieldValue::Float(None),
    );
    expected_fields.insert(
        "value_n".to_owned(),
        dbase::FieldValue::Numeric(Some(4.0)),
    );
    expected_fields.insert(
        "value_n_non".to_owned(),
        dbase::FieldValue::Numeric(None),
    );

    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_simple_file() {
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    let mut expected_fields = HashMap::new();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("linestring1".to_owned())),
    );

    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_read_write_simple_file() {
    let mut expected_fields = HashMap::new();
    expected_fields.insert(
        "name".to_owned(),
        dbase::FieldValue::Character(Some("linestring1".to_owned())),
    );

    let mut reader = dbase::Reader::from_path(LINE_DBF).unwrap();
    let records = reader.read().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);

    let writer = TableWriterBuilder::from_reader(reader)
        .build_with_dest(Cursor::new(Vec::<u8>::new()));
    let mut dst = writer.write(records).unwrap();
    dst.set_position(0);

    let mut reader = dbase::Reader::from_path(LINE_DBF).unwrap();
    let records = reader.read().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);
}

#[derive(Debug, PartialEq, Clone)]
struct Album {
    artist: String,
    name: String,
    released: dbase::Date,
    playtime: f64, // in seconds
}

impl ReadableRecord for Album {
    fn read_using<'a, 'b, T, I>(field_iterator: &mut FieldIterator<'a, 'b, T, I>) -> Result<Self, Error>
        where T: Read + Seek,
              I: Iterator<Item=&'b FieldInfo> {
        Ok(Self {
            artist: dbg!(field_iterator.read_next_field_as().unwrap()?.value),
            name: dbg!(field_iterator.read_next_field_as().unwrap()?.value),
            released: dbg!(field_iterator.read_next_field_as().unwrap()?.value),
            playtime: dbg!(field_iterator.read_next_field_as().unwrap()?.value)
        })
    }
}

impl WritableRecord for Album {
    fn values_for_fields(self, _field_names: &[&str], values: &mut Vec<FieldValue>) {
        values.push(FieldValue::Character(Some(self.artist)));
        values.push(FieldValue::Character(Some(self.name)));
        values.push(FieldValue::from(self.released));
        values.push(FieldValue::Numeric(Some(self.playtime)));
    }
}


#[test]
fn from_scratch() {
    let writer = TableWriterBuilder::new()
        .add_character_field("Artist".to_string(), 50)
        .add_character_field("Name".to_string(), 50)
        .add_date_field("Released".to_string())
        .add_numeric_field("Playtime".to_string(), 10, 2)
        .build_with_dest(Cursor::new(Vec::<u8>::new()));

    let records = vec![
        Album {
            artist: "Fallujah".to_string(),
            name: "The Flesh Prevails".to_string(),
            released: dbase::Date {
                year: 2014,
                month: 6,
                day: 22,
            },
            playtime: 2481f64
        },
        Album {
            artist: "Beyond Creation".to_string(),
            name: "Earthborn Evolution".to_string(),
            released: dbase::Date {
                year: 2014,
                month: 10,
                day: 24,
            },
            playtime: 2481f64
        },
    ];

    let mut cursor = writer.write(records.clone()).unwrap();
    cursor.seek(SeekFrom::Start(0)).unwrap();

    let mut reader = dbase::Reader::new(cursor).unwrap();
    let read_records = reader.read_as::<Album>().unwrap();

    assert_eq!(read_records, records);
}

dbase_record! {
    #[derive(Clone, Debug, PartialEq)]
    struct User {
        first_name: String,
        last_name: String
    }
}

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
        }
    ];

    let writer = TableWriterBuilder::new()
        .add_character_field("First Name".to_string(), 50)
        .add_character_field("Last Name".to_string(), 50)
        .build_with_dest(Cursor::new(Vec::<u8>::new()));

    let mut cursor = writer.write(users.clone()).unwrap();

    cursor.set_position(0);


    let mut reader = Reader::new(cursor).unwrap();
    let read_records = reader.read_as::<User>().unwrap();

    assert_eq!(read_records, users);
}

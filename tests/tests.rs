const LINE_DBF: &str = "./tests/data/line.dbf";
const NONE_FLOAT_DBF: &str = "./tests/data/contain_none_float.dbf";

extern crate dbase;

use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom, Read};
use dbase::{ReadableRecord, Error, FieldIterator, RecordFieldInfo};

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

    use std::fs::File;
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);

    let file = File::create("lol.dbf").unwrap();
    let writer = dbase::Writer::new(file);
    writer.write(&records).unwrap();

    let records = dbase::read("lol.dbf").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);
}


struct Album {
    artist: String,
    name: String,
}

impl ReadableRecord for Album {
    fn read_using<'a, 'b, T, I>(field_iterator: &mut FieldIterator<'a, 'b, T, I>) -> Result<Self, Error>
        where T: Read + Seek,
              I: Iterator<Item=&'b RecordFieldInfo> {
        Ok(Self {
            artist: field_iterator.read_next_field_as().unwrap()?.1,
            name: field_iterator.read_next_field_as().unwrap()?.1
        })
    }
}

#[test]
fn from_scratch() {
    let mut fst = dbase::Record::new();
    fst.insert(
        "Artist".to_string(),
        dbase::FieldValue::from("Fallujah"),
    );
    fst.insert(
        "Name".to_string(),
        dbase::FieldValue::from("The Flesh Prevails")
    );

    let mut scnd = dbase::Record::new();
    scnd.insert(
        "Artist".to_string(),
        dbase::FieldValue::from("Beyond Creation"),
    );
    scnd.insert(
        "Name".to_string(),
        dbase::FieldValue::from("Earthborn Evolution"),
    );

    let records = vec![fst, scnd];

    let cursor = Cursor::new(Vec::<u8>::new());
    let writer = dbase::Writer::new(cursor);
    let mut cursor = writer.write(&records).unwrap();
    cursor.seek(SeekFrom::Start(0)).unwrap();

    let reader = dbase::Reader::new(cursor).unwrap();
    let read_records = reader.read().unwrap();

    assert_eq!(read_records.len(), 2);

    match read_records[0].get("Artist").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, &Some(String::from("Fallujah"))),
        _ => assert!(false),
    }
    match read_records[0].get("Name").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, &Some(String::from("The Flesh Prevails"))),
        _ => assert!(false),
    }
    match read_records[1].get("Artist").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, &Some(String::from("Beyond Creation"))),
        _ => assert!(false),
    }
    match read_records[1].get("Name").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, &Some(String::from("Earthborn Evolution"))),
        _ => assert!(false),
    }
}


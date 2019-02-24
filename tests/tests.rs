const LINE_DBF: &str = "./tests/data/line.dbf";

extern crate dbase;

use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};


#[test]
fn test_simple_file() {
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    let mut expected_fields = HashMap::new();
    expected_fields.insert("name".to_owned(), dbase::FieldValue::Character("linestring1".to_owned()));

    assert_eq!(records[0], expected_fields);
}

#[test]
fn test_read_write_simple_file() {
    let mut expected_fields = HashMap::new();
    expected_fields.insert("name".to_owned(), dbase::FieldValue::Character("linestring1".to_owned()));

    use std::fs::File;
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);

    let file = File::create("lol.dbf").unwrap();
    let writer = dbase::Writer::new(file);
    writer.write(records).unwrap();

    let records = dbase::read("lol.dbf").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], expected_fields);
}

#[test]
fn from_scratch() {
    let mut fst = dbase::Record::new();
    fst.insert("Name".to_string(), dbase::FieldValue::Character("Fallujah".to_string()));

    let mut scnd = dbase::Record::new();
    scnd.insert("Name".to_string(), dbase::FieldValue::Character("Beyond Creation".to_string()));

    let records = vec![fst, scnd];

    let cursor = Cursor::new(Vec::<u8>::new());
    let writer = dbase::Writer::new(cursor);
    let mut cursor = writer.write(records).unwrap();
    cursor.seek(SeekFrom::Start(0)).unwrap();

    let reader = dbase::Reader::new(cursor).unwrap();
    let read_records = reader.read().unwrap();

    assert_eq!(read_records.len(), 2);

    match read_records[0].get("Name").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, "Fallujah"),
        _ => assert!(false)
    }
    match read_records[1].get("Name").unwrap() {
        dbase::FieldValue::Character(s) => assert_eq!(s, "Beyond Creation"),
        _ => assert!(false)
    }
}


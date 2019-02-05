const LINE_DBF: &str = "./tests/data/line.dbf";

extern crate dbase;

use std::collections::HashMap;


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


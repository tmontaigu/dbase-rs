const LINE_DBF: &str = "./tests/data/line.dbf";

extern crate dbase;

use std::collections::HashMap;

#[test]
fn test() {
    let records = dbase::read(LINE_DBF).unwrap();
    assert_eq!(records.len(), 1);
    let mut expected_fields = HashMap::new();
    expected_fields.insert("name".to_owned(), dbase::FieldValue::Character("linestring1".to_owned()));

    assert_eq!(records[0], expected_fields);
}
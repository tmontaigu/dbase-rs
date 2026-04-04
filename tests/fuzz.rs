use std::io::Cursor;

use dbase::Reader;

#[test]
fn overflow_in_num_fields_sub() {
    let data = include_bytes!("fuzz/reader_new/reader-new-overflow.bin");
    let data = Cursor::new(data);

    let _ = Reader::new(data);
}

#[test]
fn oob_memo_field_bytes() {
    let data = include_bytes!("fuzz/read_all/oob-memo-field-bytes.bin");
    let cursor = Cursor::new(data);

    if let Ok(mut reader) = Reader::new(cursor) {
        let _ = reader.read();
    }
}

#[test]
fn empty_logical_field() {
    let data = include_bytes!("fuzz/read_all/empty-logical-field.bin");
    let cursor = Cursor::new(data);

    if let Ok(mut reader) = Reader::new(cursor) {
        let _ = reader.read();
    }
}

#[test]
fn julian_day_overflow() {
    let data = include_bytes!("fuzz/read_all/julian-day-overflow.bin");
    let cursor = Cursor::new(data);

    if let Ok(mut reader) = Reader::new(cursor) {
        let _ = reader.read();
    }
}

#[test]
fn invalid_time_word() {
    let data = include_bytes!("fuzz/read_all/invalid-time-word.bin");
    let cursor = Cursor::new(data);

    if let Ok(mut reader) = Reader::new(cursor) {
        let _ = reader.read();
    }
}

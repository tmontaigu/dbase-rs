#![no_main]

use dbase::Reader;
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let cursor = Cursor::new(data);
    if let Ok(mut reader) = Reader::new(cursor) {
        let _ = reader.read();
    }
});

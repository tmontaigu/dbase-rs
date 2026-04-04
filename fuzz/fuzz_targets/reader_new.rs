#![no_main]

use dbase::Reader;
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let cursor = Cursor::new(data);
    let _ = Reader::new(cursor);

    // fuzzed code goes here
});

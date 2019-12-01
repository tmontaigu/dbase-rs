extern crate dbase;

fn main() {
    let dbf_path = std::env::args().nth(1).expect("Path to file as first arg");
    let mut reader = dbase::Reader::from_path(dbf_path).unwrap();

    for (i, record_result) in reader.iter_records().enumerate() {
        println!("Record {}", i);
        let record = record_result.unwrap();
        for (name, value) in record {
            println!("\tname: {}, value: {:?}", name, value);
        }
    }
}

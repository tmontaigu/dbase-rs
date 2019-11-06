extern crate dbase;

fn main() {
    let dbf_path = std::env::args().nth(1).expect("Path to file as first arg");
    let reader = dbase::Reader::from_path(dbf_path).unwrap();

    for record_result in reader {
        let record = record_result.unwrap();
        for (name, value) in record {
            println!("name: {}, value: {:?}", name, value);
        }
    }
}

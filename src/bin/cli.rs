
fn main() {
    let cp936_dbf: &str = "tests/data/cp936.dbf";

    //let mut reader = dbase::Reader::from_path(cp850_dbf).unwrap();
    let mut reader = dbase::Reader::from_path(cp936_dbf).unwrap();
    let records = reader.read().unwrap();
    for record in records {
        println!("{:?}", record.get("TEST"));
    }

}



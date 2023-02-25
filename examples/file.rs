use dbase::FieldValue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let from = "./tests/data/stations.dbf";
    let to = "./stations-can-be-deleted.dbf";
    std::fs::copy(from, to)?;

    let mut file = dbase::File::open_read_write(to)?;

    println!("{:?}", file.fields());

    let name_field = file.field_index("name").unwrap();

    let mut r = file.record(0).unwrap();
    let mut field = r.field(name_field).unwrap();

    let field_value = field.read().unwrap();
    println!("value: {}", field_value);

    field.write(&FieldValue::Character(Option::from("Toulouse".to_string())))?;

    let rr = file.record(0).unwrap().read().unwrap();
    println!("record: {:?}", rr);

    let mut record_iter = file.records();
    while let Some(mut record_ref) = record_iter.next() {
        println!("record: {:?}", record_ref);
        println!("{:#?}", record_ref.read().unwrap())
    }

    std::fs::remove_file(to)?;

    Ok(())
}

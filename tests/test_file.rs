use std::io::{Read, Seek, SeekFrom, Write};

const STATIONS_WITH_DELETED: &str = "./tests/data/stations_with_deleted.dbf";

fn copy_to_tmp_file(origin: &str) -> std::io::Result<std::fs::File> {
    let mut data = vec![];
    std::fs::File::open(origin).and_then(|mut f| f.read_to_end(&mut data))?;

    let mut tmp_file = tempfile::tempfile()?;
    tmp_file.write_all(&data)?;
    tmp_file.flush()?;
    tmp_file.seek(SeekFrom::Start(0))?;

    Ok(tmp_file)
}

fn copy_to_named_tmp_file(origin: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let mut data = vec![];
    std::fs::File::open(origin).and_then(|mut f| f.read_to_end(&mut data))?;

    let mut tmp_file = tempfile::NamedTempFile::new()?;
    tmp_file.write_all(&data)?;
    tmp_file.flush()?;
    tmp_file.seek(SeekFrom::Start(0))?;

    Ok(tmp_file)
}

#[test]
fn test_file_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = dbase::File::open_read_only("tests/data/stations.dbf")?;

    assert_eq!(file.num_records(), 6);

    let name_idx = file.field_index("name").unwrap();
    let marker_color_idx = file.field_index("marker-col").unwrap();
    let marker_symbol_idx = file.field_index("marker-sym").unwrap();

    // Test manually reading fields (not in correct order) to FieldValue
    let mut rh = file.record(3).unwrap();
    let marker_color = rh.field(marker_color_idx).unwrap().read()?;
    assert_eq!(
        marker_color,
        dbase::FieldValue::Character(Some("#ff0000".to_string()))
    );
    let name = rh.field(name_idx).unwrap().read()?;
    assert_eq!(
        name,
        dbase::FieldValue::Character(Some("Judiciary Sq".to_string()))
    );
    let marker_symbol = rh.field(marker_symbol_idx).unwrap().read()?;
    assert_eq!(
        marker_symbol,
        dbase::FieldValue::Character(Some("rail-metro".to_string()))
    );

    // Test manually reading fields (not in correct order) to concrete type
    let mut rh = file.record(0).unwrap();
    let marker_color = rh.field(marker_color_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_color, "#0000ff");
    let name = rh.field(name_idx).unwrap().read_as::<String>()?;
    assert_eq!(name, "Van Dorn Street");
    let marker_symbol = rh.field(marker_symbol_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_symbol, "rail-metro");

    // Test whole record at once
    let mut rh = file.record(5).unwrap();
    let record = rh.read()?;
    let mut expected_record = dbase::Record::default();
    expected_record.insert(
        "name".to_string(),
        dbase::FieldValue::Character(Some("Metro Center".to_string())),
    );
    expected_record.insert(
        "marker-col".to_string(),
        dbase::FieldValue::Character(Some("#ff0000".to_string())),
    );
    expected_record.insert(
        "marker-sym".to_string(),
        dbase::FieldValue::Character(Some("rail-metro".to_string())),
    );
    expected_record.insert(
        "line".to_string(),
        dbase::FieldValue::Character(Some("red".to_string())),
    );

    assert_eq!(record, expected_record);

    Ok(())
}

#[test]
fn test_file_read_write() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_file = copy_to_tmp_file("tests/data/stations.dbf")?;
    let mut file = dbase::File::open(tmp_file)?;

    assert_eq!(file.num_records(), 6);

    let name_idx = file.field_index("name").unwrap();
    let marker_color_idx = file.field_index("marker-col").unwrap();
    let marker_symbol_idx = file.field_index("marker-sym").unwrap();

    // Test manually writing fields (not in correct order) to FieldValue
    let mut rh = file.record(3).unwrap();

    let mut fh = rh.field(marker_color_idx).unwrap();
    let marker_color = fh.read()?;
    assert_eq!(
        marker_color,
        dbase::FieldValue::Character(Some("#ff0000".to_string()))
    );

    fh.write(&dbase::FieldValue::Character(Some("#00ff00".to_string())))?;
    let marker_color = fh.read()?;
    assert_eq!(
        marker_color,
        dbase::FieldValue::Character(Some("#00ff00".to_string()))
    );

    let mut fh = rh.field(name_idx).unwrap();
    let name = fh.read()?;
    assert_eq!(
        name,
        dbase::FieldValue::Character(Some("Judiciary Sq".to_string()))
    );
    fh.write(&dbase::FieldValue::Character(Some("Paris".to_string())))?;
    let marker_color = fh.read()?;
    assert_eq!(
        marker_color,
        dbase::FieldValue::Character(Some("Paris".to_string()))
    );

    let mut fh = rh.field(marker_symbol_idx).unwrap();
    let marker_symbol = fh.read()?;
    assert_eq!(
        marker_symbol,
        dbase::FieldValue::Character(Some("rail-metro".to_string()))
    );
    fh.write(&dbase::FieldValue::Character(Some("road".to_string())))?;
    let marker_color = fh.read()?;
    assert_eq!(
        marker_color,
        dbase::FieldValue::Character(Some("road".to_string()))
    );

    // Test manually writing fields (not in correct order) to concrete type
    let mut rh = file.record(0).unwrap();
    let marker_color = rh.field(marker_color_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_color, "#0000ff");
    rh.field(marker_color_idx).unwrap().write(&"#ff00ff")?;
    let marker_color = rh.field(marker_color_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_color, "#ff00ff");

    let name = rh.field(name_idx).unwrap().read_as::<String>()?;
    assert_eq!(name, "Van Dorn Street");
    rh.field(name_idx).unwrap().write(&"Yoshi's street")?;
    let name = rh.field(name_idx).unwrap().read_as::<String>()?;
    assert_eq!(name, "Yoshi's street");

    let marker_symbol = rh.field(marker_symbol_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_symbol, "rail-metro");
    rh.field(marker_symbol_idx).unwrap().write(&"egg")?;
    let marker_symbol = rh.field(marker_symbol_idx).unwrap().read_as::<String>()?;
    assert_eq!(marker_symbol, "egg");

    // Test whole record at once
    let mut rh = file.record(5).unwrap();
    let record = rh.read()?;
    let mut expected_record = dbase::Record::default();
    expected_record.insert(
        "name".to_string(),
        dbase::FieldValue::Character(Some("Metro Center".to_string())),
    );
    expected_record.insert(
        "marker-col".to_string(),
        dbase::FieldValue::Character(Some("#ff0000".to_string())),
    );
    expected_record.insert(
        "marker-sym".to_string(),
        dbase::FieldValue::Character(Some("rail-metro".to_string())),
    );
    expected_record.insert(
        "line".to_string(),
        dbase::FieldValue::Character(Some("red".to_string())),
    );

    assert_eq!(record, expected_record);

    let old = expected_record.insert(
        "name".to_string(),
        dbase::FieldValue::Character(Some("Nook Island".to_string())),
    );
    assert!(old.is_some());
    rh.write(&expected_record)?;
    let record = rh.read()?;
    assert_eq!(record, expected_record);

    Ok(())
}

#[test]
fn test_file_append_record() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_file = copy_to_named_tmp_file("tests/data/stations.dbf")?;

    let mut new_record = dbase::Record::default();
    new_record.insert(
        "name".to_string(),
        dbase::FieldValue::Character(Some("Dalaran".to_string())),
    );
    new_record.insert(
        "marker-col".to_string(),
        dbase::FieldValue::Character(Some("#0f0f0f".to_string())),
    );
    new_record.insert(
        "marker-sym".to_string(),
        dbase::FieldValue::Character(Some("underground".to_string())),
    );
    new_record.insert(
        "line".to_string(),
        dbase::FieldValue::Character(Some("purple".to_string())),
    );

    {
        let mut file = dbase::File::open_read_write(tmp_file.path())?;
        assert_eq!(file.num_records(), 6);
        file.append_record(&new_record)?;

        assert_eq!(file.num_records(), 7);
        let record = file.record(6).unwrap().read()?;
        assert_eq!(record, new_record);
    }

    {
        // Check that after closing the file, if we re-open it,
        // our appended record is still here
        let mut file = dbase::File::open_read_write(tmp_file.path())?;
        assert_eq!(file.num_records(), 7);
        let record = file.record(6).unwrap().read()?;
        assert_eq!(record, new_record);
    }

    Ok(())
}

#[test]
fn test_file_classical_user_record_example() -> Result<(), Box<dyn std::error::Error>> {
    dbase::dbase_record! {
        #[derive(Clone, Debug, PartialEq)]
        struct User {
            first_name: String,
            last_name: String,
        }
    }

    let users = vec![
        User {
            first_name: "Ferrys".to_string(),
            last_name: "Rust".to_string(),
        },
        User {
            first_name: "Alex".to_string(),
            last_name: "Rider".to_string(),
        },
        User {
            first_name: "Jamie".to_string(),
            last_name: "Oliver".to_string(),
        },
    ];

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    let table_info = dbase::TableWriterBuilder::new()
        .add_character_field("First Name".try_into().unwrap(), 50)
        .add_character_field("Last Name".try_into().unwrap(), 50)
        .build_table_info();

    {
        let mut file = dbase::File::create_new(&mut cursor, table_info)?;
        file.append_records(&users)?;
    }

    cursor.set_position(0);

    let mut reader = dbase::Reader::new(cursor).unwrap();
    let read_records = reader.read_as::<User>().unwrap();
    assert_eq!(read_records, users);

    Ok(())
}

#[test]
fn test_file_is_record_deleted() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = dbase::File::open_read_only(STATIONS_WITH_DELETED)?;

    let is_first_record_deleted = file.record(0).unwrap().is_deleted()?;
    assert!(is_first_record_deleted);

    let is_second_record_deleted = file.record(1).unwrap().is_deleted()?;
    assert!(!is_second_record_deleted);
    Ok(())
}

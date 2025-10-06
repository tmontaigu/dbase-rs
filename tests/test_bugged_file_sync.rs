use dbase::FieldName;
use std::error::Error;

dbase::dbase_record!(
    #[derive(Debug, Clone, PartialEq)]
    pub struct DokumentRecord {
        d_tip: Option<String>,
        d_tipn: Option<String>,
        d_broj: Option<String>,
        d_konto: Option<String>,
        d_firma: Option<String>,
        d_mag: Option<String>,
        d_zak: Option<String>,
        d_otp: Option<String>,
        d_fak: Option<String>,
        d_knj: Option<bool>,
    }
);

impl Default for DokumentRecord {
    fn default() -> Self {
        Self {
            d_tip: Some("Yoshi".to_string()),
            d_tipn: Some("Yoshi".to_string()),
            d_broj: Some("Yoshi".to_string()),
            d_konto: Some("Yoshi".to_string()),
            d_firma: Some("Yoshi".to_string()),
            d_mag: Some("Yoshi".to_string()),
            d_zak: Some("Yoshi".to_string()),
            d_otp: Some("Yoshi".to_string()),
            d_fak: Some("Yoshi".to_string()),
            d_knj: Some(true),
        }
    }
}

#[test]
fn test_bugged_file_sync() -> Result<(), Box<dyn Error>> {
    let document_path = "bugged-sync-file.dbf";

    // Write file with default values
    {
        let mut writer = dbase::TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("d_tip").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_tipn").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_broj").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_konto").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_firma").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_mag").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_zak").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_otp").unwrap(), 255)
            .add_character_field(FieldName::try_from("d_fak").unwrap(), 255)
            .add_logical_field(FieldName::try_from("d_knj").unwrap())
            .build_with_file_dest(document_path)
            .unwrap();

        let record = DokumentRecord::default();
        for _ in 0..50_000 {
            writer.write_record(&record).unwrap();
        }

        writer.finalize().unwrap();
    }

    // Rewrite a field of each record
    {
        let mut file = dbase::File::open_read_write(document_path)?;

        let mut records = file.records();
        while let Some(mut record) = records.next() {
            let mut read_record: DokumentRecord = record.read_as()?;
            read_record.d_firma = Some("Luigi".to_string());
            record.write(&read_record)?;
        }
    }

    // Read file and check if the field was rewritten
    // Without the fix, the field will not have been modified
    {
        let expected = Some("Luigi".to_string());
        let mut reader = dbase::Reader::from_path(document_path).unwrap();
        for record in reader.iter_records_as::<DokumentRecord>() {
            let record = record.unwrap();
            assert_eq!(record.d_firma, expected);
        }
    }

    std::fs::remove_file(document_path)?;

    Ok(())
}

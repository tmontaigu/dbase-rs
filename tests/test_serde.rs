#[cfg(feature = "serde")]
mod serde_tests {
    use std::convert::TryFrom;
    use std::io::Cursor;

    use serde_derive::{Deserialize, Serialize};

    use dbase::{ErrorKind, FieldName, ReadableRecord, Reader, TableWriterBuilder, WritableRecord};
    use std::fmt::Debug;

    fn single_char_field_reader(value: &str) -> Reader<Cursor<Vec<u8>>> {
        #[derive(Serialize)]
        struct Record {
            name: String,
        }
        let mut dst = Cursor::new(Vec::<u8>::new());
        TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("name").unwrap(), 50)
            .build_with_dest(&mut dst)
            .write_records(&[Record {
                name: value.to_string(),
            }])
            .unwrap();
        dst.set_position(0);
        Reader::new(dst).unwrap()
    }

    fn write_read_compare<R>(records: &Vec<R>, writer_builder: TableWriterBuilder)
    where
        R: WritableRecord + ReadableRecord + Debug + PartialEq,
    {
        let mut dst = Cursor::new(Vec::<u8>::new());
        let writer = writer_builder.build_with_dest(&mut dst);

        writer.write_records(records).unwrap();
        dst.set_position(0);

        let mut reader = Reader::new(dst).unwrap();
        let read_records = reader.read_as::<R>().unwrap();

        assert_eq!(&read_records, records);
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct DeserializableRecord {
        name: String,
        price: f64,
        date: dbase::Date,
        available: bool,
        score: f32,
    }

    #[test]
    fn test_serde_roundtrip() {
        let records = vec![DeserializableRecord {
            name: "Holy Fawn".to_string(),
            price: 10.2,
            date: dbase::Date::new(1, 1, 2012).unwrap(),
            available: true,
            score: 9.87,
        }];

        let writer_builder = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("name").unwrap(), 25)
            .add_numeric_field(FieldName::try_from("price").unwrap(), 7, 4)
            .add_date_field(FieldName::try_from("date").unwrap())
            .add_logical_field(FieldName::try_from("available").unwrap())
            .add_float_field(FieldName::try_from("score").unwrap(), 7, 5);

        write_read_compare(&records, writer_builder);
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct DeserializableStation {
        name: String,
        marker_col: Option<String>,
        marker_symbol: String,
        line: Option<String>,
    }

    #[test]
    fn test_serde_stations_optional() {
        let mut reader = dbase::Reader::from_path("tests/data/stations_optional.dbf").unwrap();
        let records = reader.read_as::<DeserializableStation>().unwrap();
        assert_eq!(
            records[3],
            DeserializableStation {
                name: String::from("Judiciary Sq"),
                marker_col: None,
                marker_symbol: "rail-metro".to_string(),
                line: Some("blue".to_string())
            }
        );
    }

    #[test]
    fn test_serde_optional_types() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Record {
            opt_bool: Option<bool>,
        }

        let writer_builder =
            TableWriterBuilder::new().add_logical_field(FieldName::try_from("opt_bool").unwrap());

        let records = vec![
            Record {
                opt_bool: Some(true),
            },
            Record {
                opt_bool: Some(false),
            },
            Record { opt_bool: None },
        ];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serde_tuple() {
        let writer_builder = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("Name").unwrap(), 50)
            .add_numeric_field(FieldName::try_from("Price").unwrap(), 20, 6);

        let records = vec![
            ("Companion 50".to_string(), 525.32f64),
            ("Companion 20".to_string(), 125.99f64),
        ];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serde_tuple_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Record(bool, dbase::Date);

        let writer_builder = TableWriterBuilder::new()
            .add_logical_field(FieldName::try_from("bool").unwrap())
            .add_date_field(FieldName::try_from("date").unwrap());

        let records = vec![
            Record(true, dbase::Date::new(12, 10, 2012).unwrap()),
            Record(false, dbase::Date::new(12, 11, 2005).unwrap()),
        ];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serde_new_type_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Name(String);

        let writer_builder = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("character").unwrap(), 50);

        let records = vec![Name("Sinmara".to_string())];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serialize_not_enough_fields() {
        #[derive(Serialize)]
        struct Record {
            yes: bool,
        }

        let records = vec![Record { yes: false }];

        let writer = TableWriterBuilder::new()
            .add_logical_field(FieldName::try_from("yes").unwrap())
            .add_character_field(FieldName::try_from("not present").unwrap(), 50)
            .build_with_dest(Cursor::new(Vec::<u8>::new()));

        let error = writer
            .write_records(&records)
            .expect_err("We expected an Error");
        assert!(matches!(error.kind(), ErrorKind::NotEnoughFields));
    }

    #[test]
    fn test_serialize_not_too_many_fields() {
        #[derive(Serialize)]
        struct Record {
            yes: bool,
        }

        let records = vec![Record { yes: false }];

        let writer = TableWriterBuilder::new().build_with_dest(Cursor::new(Vec::<u8>::new()));

        let error = writer
            .write_records(&records)
            .expect_err("Expected an error");

        assert!(matches!(error.kind(), ErrorKind::TooManyFields));
    }

    #[test]
    fn test_serde_fox_pro_types() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Record {
            datetime: dbase::DateTime,
            currency: f64,
            double: f64,
            integer: i32,
        }

        let writer_builder = TableWriterBuilder::new()
            .add_datetime_field(FieldName::try_from("datetime").unwrap())
            .add_currency_field(FieldName::try_from("currency").unwrap())
            .add_double_field(FieldName::try_from("double").unwrap())
            .add_integer_field(FieldName::try_from("integer").unwrap());

        let records = vec![Record {
            datetime: dbase::DateTime::new(
                dbase::Date::new(12, 5, 2130).unwrap(),
                dbase::Time::new(15, 52, 12).unwrap(),
            ),
            currency: 79841.156846,
            double: 976114.1846,
            integer: -15315,
        }];

        write_read_compare(&records, writer_builder);
    }

    // `from_field_error` has two branches: when the FieldError carries a FieldContext it promotes
    // to Error::field (field_index is Some), and when it doesn't it falls back to Error::record
    // (field_index is None). The two tests below pin each branch.

    #[test]
    fn test_from_field_error_with_context_sets_field_index() {
        // A type mismatch that goes through read_next_field_as produces a FieldError with
        // context (field index + name + type), so field_index() should be Some.
        #[derive(Debug, Deserialize)]
        #[allow(
            dead_code,
            reason = "field exists only to drive serde deserialization; value is never read"
        )]
        struct ReadRecord {
            name: bool, // Character field read as bool → IncompatibleType with context
        }

        let error = single_char_field_reader("test")
            .read_as::<ReadRecord>()
            .expect_err("expected a type mismatch error");

        assert_eq!(error.record_index(), Some(dbase::RecordIndex(0)));
        assert_eq!(error.field_index(), Some(dbase::FieldIndex(0)));
    }

    #[test]
    fn test_from_field_error_without_context_has_no_field_index() {
        // deserialize_any on FieldIterator returns FieldError { context: None }, so
        // from_field_error should fall back to Error::record: record_index is Some but
        // field_index is None.
        #[derive(Debug)]
        struct TriggerDeserializeAny;

        impl<'de> serde::Deserialize<'de> for TriggerDeserializeAny {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = TriggerDeserializeAny;
                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "anything")
                    }
                }
                deserializer.deserialize_any(V)
            }
        }

        #[derive(Debug, Deserialize)]
        #[allow(
            dead_code,
            reason = "field exists only to drive serde deserialization; value is never read"
        )]
        struct ReadRecord {
            name: TriggerDeserializeAny,
        }

        let error = single_char_field_reader("test")
            .read_as::<ReadRecord>()
            .expect_err("expected IncompatibleType from deserialize_any");

        assert!(matches!(error.kind(), ErrorKind::IncompatibleType));
        assert_eq!(error.record_index(), Some(dbase::RecordIndex(0)));
        assert_eq!(error.field_index(), None);
    }
}

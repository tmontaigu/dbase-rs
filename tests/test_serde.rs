extern crate dbase;
#[cfg(feature = "serde")]
extern crate serde_derive;

#[cfg(feature = "serde")]
mod serde_tests {
    use std::convert::TryFrom;
    use std::io::Cursor;

    use serde_derive::{Deserialize, Serialize};

    use dbase::{Error, FieldName, ReadableRecord, Reader, TableWriterBuilder, WritableRecord};
    use std::fmt::Debug;

    fn write_read_compare<R: WritableRecord + ReadableRecord + Debug + PartialEq>(records: &Vec<R>, writer_builder: TableWriterBuilder) {
        let writer = writer_builder.build_with_dest(Cursor::new(Vec::<u8>::new()));

        let mut dst = writer.write(records).unwrap();
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
        let records = vec![
            DeserializableRecord {
                name: "Holy Fawn".to_string(),
                price: 10.2,
                date: dbase::Date::new(01, 01, 2012).unwrap(),
                available: true,
                score: 79.87,
            }
        ];

        let writer_builder = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("name").unwrap(), 25)
            .add_numeric_field(FieldName::try_from("price").unwrap(), 7, 4)
            .add_date_field(FieldName::try_from("date").unwrap())
            .add_logical_field(FieldName::try_from("available").unwrap())
            .add_float_field(FieldName::try_from("score").unwrap(), 7, 5);

        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serde_optional_types() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Record {
            opt_bool: Option<bool>
        }

        let writer_builder = TableWriterBuilder::new()
            .add_logical_field(FieldName::try_from("opt_bool").unwrap());

        let records = vec![
            Record {
                opt_bool: Some(true)
            },
            Record {
                opt_bool: Some(false)
            },
            Record {
                opt_bool: None
            }
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
            ("Companion 20".to_string(), 125.99f64)
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
            Record { 0: true, 1: dbase::Date::new(12, 10, 2012).unwrap() },
            Record { 0: false, 1: dbase::Date::new(12, 11, 2005).unwrap() },
        ];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serde_new_type_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Name(String);

        let writer_builder = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("character").unwrap(), 50);

        let records = vec![
            Name("Sinmara".to_string())
        ];
        write_read_compare(&records, writer_builder);
    }

    #[test]
    fn test_serialize_not_enough_fields() {
        #[derive(Serialize)]
        struct Record {
            yes: bool
        }

        let records = vec![
            Record {yes: false}
        ];

        let writer = TableWriterBuilder::new()
            .add_logical_field(FieldName::try_from("yes").unwrap())
            .add_character_field(FieldName::try_from("not present").unwrap(), 50)
            .build_with_dest(Cursor::new(Vec::<u8>::new()));


        let result = writer.write(&records);

        match result {
            Err(Error::NotEnoughFields) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_serialize_not_too_many_fields() {
        #[derive(Serialize)]
        struct Record {
            yes: bool,
        }

        let records = vec![
            Record { yes: false }
        ];

        let writer = TableWriterBuilder::new()
            .build_with_dest(Cursor::new(Vec::<u8>::new()));

        let result = writer.write(&records);

        match result {
            Err(Error::EndOfRecord) => assert!(true),
            _ => assert!(false)
        }
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

        let records = vec![
            Record {
                datetime: dbase::DateTime::new(
                    dbase::Date::new(12, 05, 2130).unwrap(),
                    dbase::Time::new(15, 52, 12)
                ),
                currency: 79841.156846,
                double: 976114.1846,
                integer: -15315
            }
        ];

        write_read_compare(&records, writer_builder);
    }
}

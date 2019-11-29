extern crate dbase;
#[cfg(feature = "serde")]
extern crate serde;

#[cfg(feature = "serde")]
mod serde_tests {
    use serde::{Deserialize, Serialize};
    use dbase::{TableWriterBuilder, Reader, FieldName, Error, FieldValue, FieldWriter, WritableRecord, ReadableRecord};
    use std::io::{Cursor, Write};
    use std::convert::TryFrom;
    use serde::export::fmt::Debug;

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
}

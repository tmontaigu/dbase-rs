extern crate dbase;
#[cfg(feature = "serde")]
extern crate serde;

#[cfg(feature = "serde")]
mod serde_tests {

    use serde::Deserialize;
    use dbase::{FieldValueCollector, TableWriterBuilder, Reader, FieldName};
    use std::io::Cursor;
    use std::convert::TryFrom;

    #[derive(Deserialize, Clone, PartialEq, Debug)]
    struct DeserializableRecord {
        name: String,
        price: f64,
        date: dbase::Date,
        available: bool,
        score: f32,
    }

    impl dbase::WritableRecord for DeserializableRecord {
        fn values_for_fields(self, field_names: &[&str], values: &mut FieldValueCollector) {
            values.push(self.name.into());
            values.push(self.price.into());
            values.push(self.date.into());
            values.push(self.available.into());
            values.push(self.score.into());
        }
    }

    #[test]
    fn test_deserialize() {
        let records = vec![
            DeserializableRecord {
                name: "Holy Fawn".to_string(),
                price: 10.2,
                date: dbase::Date::new(01, 01, 2012).unwrap(),
                available: true,
                score: 79.87,
            }
        ];

        let writer = TableWriterBuilder::new()
            .add_character_field(FieldName::try_from("name").unwrap(), 25)
            .add_numeric_field(FieldName::try_from("price").unwrap(), 7, 4)
            .add_date_field(FieldName::try_from("date").unwrap())
            .add_logical_field(FieldName::try_from("available").unwrap())
            .add_float_field(FieldName::try_from("score").unwrap(), 7, 5)
            .build_with_dest(Cursor::new(Vec::<u8>::new()));

        let mut cursor = writer.write(records.clone()).unwrap();
        cursor.set_position(0);

        let mut reader = Reader::new(cursor).unwrap();
        let r = reader.read_as::<DeserializableRecord>().unwrap();
        assert_eq!(r, records);
    }
}

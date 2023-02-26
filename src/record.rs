use crate::{FieldIOError, FieldIterator, FieldValue, NamedValue, ReadableRecord};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Type definition of a generic record.
/// A .dbf file is composed of many records
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Record {
    map: HashMap<String, FieldValue>,
}

impl ReadableRecord for Record {
    fn read_using<Source, MemoSource>(
        field_iterator: &mut FieldIterator<Source, MemoSource>,
    ) -> Result<Self, FieldIOError>
    where
        Source: Read + Seek,
        MemoSource: Read + Seek,
    {
        let mut map = HashMap::<String, FieldValue>::new();
        for result in field_iterator {
            let NamedValue { name, value } = result?;
            map.insert(name.to_owned(), value);
        }
        Ok(Self { map })
    }
}

impl Record {
    /// Inserts a new value in the record, returning the old one if there was any
    ///
    /// # Example
    ///
    /// ```
    /// let mut record = dbase::Record::default();
    /// record.insert("FirstName".to_owned(), dbase::FieldValue::Character(Some("Yoshi".to_owned())));
    /// ```
    pub fn insert(&mut self, field_name: String, value: FieldValue) -> Option<FieldValue> {
        self.map.insert(field_name, value)
    }

    /// Returns the [FieldValue](enum.FieldValue.html) for the given field name
    pub fn get(&self, field_name: &str) -> Option<&FieldValue> {
        self.map.get(field_name)
    }

    /// Returns the mutable [FieldValue](enum.FieldValue.html) for the given field name
    pub fn get_mut(&mut self, field_name: &str) -> Option<&mut FieldValue> {
        self.map.get_mut(field_name)
    }

    /// Removes the [FieldValue](enum.FieldValue.html) for the given field name
    pub fn remove(&mut self, field_name: &str) -> Option<FieldValue> {
        self.map.remove(field_name)
    }
}

impl IntoIterator for Record {
    type Item = (String, FieldValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, FieldValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl From<HashMap<String, FieldValue>> for Record {
    fn from(map: HashMap<String, FieldValue, RandomState>) -> Self {
        Self { map }
    }
}

impl From<Record> for HashMap<String, FieldValue> {
    fn from(record: Record) -> HashMap<String, FieldValue> {
        record.map
    }
}

impl AsRef<HashMap<String, FieldValue>> for Record {
    fn as_ref(&self) -> &HashMap<String, FieldValue> {
        &self.map
    }
}

impl AsMut<HashMap<String, FieldValue>> for Record {
    fn as_mut(&mut self) -> &mut HashMap<String, FieldValue> {
        &mut self.map
    }
}

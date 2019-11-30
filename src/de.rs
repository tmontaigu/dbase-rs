use std::fmt::Display;
use std::io::{Read, Seek, SeekFrom};

use serde::Deserializer;
use serde::de::{DeserializeOwned, Visitor, DeserializeSeed, SeqAccess};

use ::ReadableRecord;

use crate::Error;
use crate::FieldIterator;
use FieldValue;


impl<'de, 'a, 'f, R: Read + Seek> SeqAccess<'de> for &mut FieldIterator<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<<T as DeserializeSeed<'de>>::Value>, Self::Error> where
        T: DeserializeSeed<'de> {
        if self.fields_info.peek().is_none() {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}


//TODO maybe we can deserialize numbers other than f32 & f64 by converting using TryFrom
impl<'de, 'a, 'f , T: Read + Seek> Deserializer<'de> for &mut FieldIterator<'a, T> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
       unimplemented!("Dbase cannot deserialize any")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        let value = self.read_next_field_as::<bool>().ok_or(Error::EndOfRecord)??.value;
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize i8")
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize i16")
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize i23")
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize i64")
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize u8")
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize u16")
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize u32")
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize u64")
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        let value = self.read_next_field_as::<f32>().ok_or(Error::EndOfRecord)??.value;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        let value = self.read_next_field_as::<f64>().ok_or(Error::EndOfRecord)??.value;
        visitor.visit_f64(value)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize char")
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize str")
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        let value = self.read_next_field_as::<String>().ok_or(Error::EndOfRecord)??.value;
        visitor.visit_string(value)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize bytes")
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        let value = self.read_next_field_raw().ok_or(Error::EndOfRecord)??;
        visitor.visit_byte_buf(value)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        //FIXME this is actually terrible, this means we read the field twice
        // Would just peeking the first fex bytes and checking they are not padding bytes be better ?
        let value: FieldValue = self.peek_next_field()?.value;
        match value {
            FieldValue::Character(Some(_)) => visitor.visit_some(self),
            FieldValue::Logical(Some(_)) => visitor.visit_some(self),
            FieldValue::Numeric(Some(_)) => visitor.visit_some(self),
            FieldValue::Float(Some(_)) => visitor.visit_some(self),
            FieldValue::Date(Some(_)) => visitor.visit_some(self),
            FieldValue::Character(None) => visitor.visit_none(),
            FieldValue::Logical(None) => visitor.visit_none(),
            FieldValue::Numeric(None) => visitor.visit_none(),
            FieldValue::Float(None) => visitor.visit_none(),
            FieldValue::Date(None) => visitor.visit_none(),
            _ => visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize unit")
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize unit struct")
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        _visitor.visit_seq(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize sequence")
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        visitor.visit_seq(self)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize map")
    }

    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        visitor.visit_seq(self)
    }

    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize enum")
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize identifiers")
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error> where
        V: Visitor<'de> {
        unimplemented!("DBase cannot deserialize ignored any")
    }
}


impl<S: DeserializeOwned> ReadableRecord for S {
    fn read_using<T>(field_iterator: &mut FieldIterator<T>) -> Result<Self, Error> where T: Read + Seek {
        S::deserialize(field_iterator)
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

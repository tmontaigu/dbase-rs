use std::fmt::Display;
use std::io::{Read, Seek};

use serde::de::{DeserializeOwned, DeserializeSeed, IntoDeserializer, SeqAccess, Visitor};
use serde::Deserializer;

use crate::{
    ErrorKind, FieldConversionError, FieldIOError, FieldIterator, FieldValue, ReadableRecord,
};

impl<'de, 'a, R1, R2> SeqAccess<'de> for &mut FieldIterator<'a, R1, R2>
where
    R1: Read + Seek,
    R2: Read + Seek,
{
    type Error = FieldIOError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> Result<Option<<T as DeserializeSeed<'de>>::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.fields_info.peek().is_none() {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

//TODO maybe we can deserialize numbers other than f32 & f64 by converting using TryFrom
impl<'de, 'a, T, R> Deserializer<'de> for &mut FieldIterator<'a, T, R>
where
    T: Read + Seek,
    R: Read + Seek,
{
    type Error = FieldIOError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(FieldConversionError::IncompatibleType.into())
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<bool>()?.value;
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<i32>()?.value;
        visitor.visit_i8(
            value
                .try_into()
                .ok()
                .ok_or(FieldConversionError::IncompatibleType)?,
        )
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<i32>()?.value;
        visitor.visit_i16(
            value
                .try_into()
                .ok()
                .ok_or(FieldConversionError::IncompatibleType)?,
        )
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<i32>()?.value;
        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<i32>()?.value;
        visitor.visit_i64(value.into())
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<i32>()?.value;
        visitor.visit_u8(
            value
                .try_into()
                .ok()
                .ok_or(FieldConversionError::IncompatibleType)?,
        )
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize u16")
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize u32")
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize u64")
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<f32>()?.value;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<f64>()?.value;
        visitor.visit_f64(value)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize char")
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize str")
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<String>()?.value;
        visitor.visit_string(value)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize bytes")
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_raw()?;
        visitor.visit_byte_buf(value)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        //FIXME this is actually terrible, this means we read the field twice
        // Would just peeking the first few bytes and checking they are not padding bytes be better ?
        let value: FieldValue = self.peek_next_field()?.value;
        match value {
            FieldValue::Character(Some(_)) => visitor.visit_some(self),
            FieldValue::Logical(Some(_)) => visitor.visit_some(self),
            FieldValue::Numeric(Some(_)) => visitor.visit_some(self),
            FieldValue::Float(Some(_)) => visitor.visit_some(self),
            FieldValue::Date(Some(_)) => visitor.visit_some(self),
            FieldValue::Character(None) => {
                self.skip_next_field()?;
                visitor.visit_none()
            }
            FieldValue::Logical(None) => {
                self.skip_next_field()?;
                visitor.visit_none()
            }
            FieldValue::Numeric(None) => {
                self.skip_next_field()?;
                visitor.visit_none()
            }
            FieldValue::Float(None) => {
                self.skip_next_field()?;
                visitor.visit_none()
            }
            FieldValue::Date(None) => {
                self.skip_next_field()?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize unit")
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize unit struct")
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("dBase cannot deserialize sequence")
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize map")
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.read_next_field_as::<String>()?.value;
        visitor.visit_enum(value.into_deserializer())
    }

    fn deserialize_identifier<V>(
        self,
        _visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize identifiers")
    }

    fn deserialize_ignored_any<V>(
        self,
        _visitor: V,
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("DBase cannot deserialize ignored any")
    }
}

impl<S: DeserializeOwned> ReadableRecord for S {
    fn read_using<T, R>(field_iterator: &mut FieldIterator<T, R>) -> Result<Self, FieldIOError>
    where
        T: Read + Seek,
        R: Read + Seek,
    {
        S::deserialize(field_iterator)
    }
}

impl serde::de::Error for FieldIOError {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            field: None,
            kind: ErrorKind::Message(msg.to_string()),
        }
    }
}

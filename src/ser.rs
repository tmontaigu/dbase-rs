use serde::{Serialize, Serializer};
use std::io::Write;

use crate::field::types::FieldType;
use crate::writing::FieldWriter;
use crate::{Date, FieldError};
use crate::{ErrorKind, WritableRecord};

impl<T> WritableRecord for T
where
    T: Serialize,
{
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldError> {
        self.serialize(field_writer)
    }
}

impl<'a, W: Write> Serializer for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&i32::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&i32::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let v: i32 = v.try_into().map_err(|_| -> FieldError {
            serde::ser::Error::custom("i64 value out of range for dBase integer field")
        })?;
        self.write_next_field_value(&v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&i32::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&i32::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        let v: i32 = v.try_into().map_err(|_| -> FieldError {
            serde::ser::Error::custom("u32 value out of range for dBase integer field")
        })?;
        self.write_next_field_value(&v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let v: i32 = v.try_into().map_err(|_| -> FieldError {
            serde::ser::Error::custom("u64 value out of range for dBase integer field")
        })?;
        self.write_next_field_value(&v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        let s: &str = v.encode_utf8(&mut buf);
        self.write_next_field_value(&s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_raw(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        if let Some(field_info) = self.fields_info.get(self.next_index.0) {
            match field_info.field_type {
                FieldType::Character => self.write_next_field_value::<Option<String>>(&None),
                FieldType::Numeric => self.write_next_field_value::<Option<f64>>(&None),
                FieldType::Float => self.write_next_field_value::<Option<f32>>(&None),
                FieldType::Date => self.write_next_field_value::<Option<Date>>(&None),
                FieldType::Logical => self.write_next_field_value::<Option<bool>>(&None),
                _ => Err(FieldError::from_info(
                    self.next_index,
                    field_info,
                    ErrorKind::Message("This field cannot store None values".to_string()),
                )),
            }
        } else {
            Err(FieldError::end_of_record())
        }
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support unit types",
        ))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support unit structs",
        ))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support unit variants",
        ))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(serde::ser::Error::custom(
            "dBase does not support newtype variants",
        ))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self as Self::SerializeSeq)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support tuple variants",
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(serde::ser::Error::custom("dBase does not support maps"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support struct variants",
        ))
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display + ?Sized,
    {
        let s = value.to_string();
        self.write_next_field_value(&s.as_str())
    }
}

impl<'a, W: Write> serde::ser::SerializeStructVariant for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(serde::ser::Error::custom(
            "dBase does not support struct variants",
        ))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support struct variants",
        ))
    }
}

impl<'a, W: Write> serde::ser::SerializeStruct for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeSeq for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeMap for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(serde::ser::Error::custom("dBase does not support maps"))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(serde::ser::Error::custom("dBase does not support maps"))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("dBase does not support maps"))
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleVariant for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(serde::ser::Error::custom(
            "dBase does not support tuple variants",
        ))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom(
            "dBase does not support tuple variants",
        ))
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleStruct for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeTuple for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl serde::ser::Error for FieldError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self {
            context: None,
            kind: ErrorKind::Message(msg.to_string()),
        }
    }
}

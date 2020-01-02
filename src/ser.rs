use serde::{Serialize, Serializer};
use std::io::{Write};

use record::field::FieldType;
use writing::FieldWriter;
use ::{Date, FieldIOError};
use {WritableRecord, ErrorKind};

impl<T> WritableRecord for T
where
    T: Serialize,
{
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldIOError> {
        self.serialize(field_writer)
    }
}

impl<'a, W: Write> Serializer for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;
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

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize i16")
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        //        let field_info = self.fields_info.next().ok_or(Error::EndOfRecord)?;
        //        if field_info.field_type == FieldType::Integer {
        self.write_next_field_value(&v)
        //        } else {
        //            Err(Error::IncompatibleType)
        //        }
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize i64")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize u64")
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Dbase cannot serialize char")
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_value(&v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_next_field_raw(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        if let Some(field_info) = self.fields_info.peek() {
            match field_info.field_type {
                FieldType::Character => self.write_next_field_value::<Option<String>>(&None),
                FieldType::Numeric => self.write_next_field_value::<Option<f64>>(&None),
                FieldType::Float => self.write_next_field_value::<Option<f32>>(&None),
                FieldType::Date => self.write_next_field_value::<Option<Date>>(&None),
                FieldType::Logical => self.write_next_field_value::<Option<bool>>(&None),
                _ => Err(FieldIOError::new(
                    ErrorKind::Message(format!(
                        "This field cannot store None values")),
                    Some((*field_info).to_owned())
                ))
            }
        } else {
            Err(FieldIOError::end_of_record())
        }
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        unimplemented!("dBase cannot serialize unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!("dBase cannot serialize unit_variant")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
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
        unimplemented!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
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
        unimplemented!()
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display,
    {
        unimplemented!()
    }
}

impl<'a, W: Write> serde::ser::SerializeStructVariant for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write> serde::ser::SerializeStruct for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeSeq for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeMap for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleVariant for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleStruct for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeTuple for &mut FieldWriter<'a, W> {
    type Ok = ();
    type Error = FieldIOError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl serde::ser::Error for FieldIOError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self {
            field: None,
            kind: ErrorKind::Message(msg.to_string())
        }
    }
}

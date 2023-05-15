use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::{Read, Seek, Write};
use std::str::FromStr;

use crate::Encoding;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::ErrorKind;
use crate::field::FieldInfo;
use crate::memo::MemoReader;
use crate::writing::WritableAsDbaseField;

/// Enum listing all the field types we know of
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FieldType {
    // dBASE III
    Character,
    Date,
    Float,
    Numeric,
    Logical,
    // Visual FoxPro
    Currency,
    DateTime,
    Integer,
    // Unknown
    Double,
    Memo,
    //General,
    //BinaryCharacter,
    //BinaryMemo,
}

impl From<FieldType> for u8 {
    fn from(t: FieldType) -> Self {
        let v = match t {
            FieldType::Character => 'C',
            FieldType::Date => 'D',
            FieldType::Float => 'F',
            FieldType::Numeric => 'N',
            FieldType::Logical => 'L',
            FieldType::Currency => 'Y',
            FieldType::DateTime => 'T',
            FieldType::Integer => 'I',
            FieldType::Double => 'B',
            FieldType::Memo => 'M',
        };
        v as u8
    }
}

impl FieldType {
    pub fn from(c: char) -> Option<FieldType> {
        match c {
            // dBASE III field types
            // All stored as strings
            'C' => Some(FieldType::Character),
            'D' => Some(FieldType::Date),
            'F' => Some(FieldType::Float),
            'N' => Some(FieldType::Numeric),
            'L' => Some(FieldType::Logical),
            // Visual FoxPro field types
            // stored in binary formats
            'Y' => Some(FieldType::Currency),
            'T' => Some(FieldType::DateTime),
            'I' => Some(FieldType::Integer),
            // unknown version
            'B' => Some(FieldType::Double),
            'M' => Some(FieldType::Memo),
            //'G' => Some(FieldType::General),
            //'C' => Some(FieldType::BinaryCharacter), ??
            //'M' => Some(FieldType::BinaryMemo),
            _ => None,
        }
    }

    /// Returns the size when stored in a file
    ///
    /// None is returned when the size cannot be known statically
    /// (the in-file size depends on the field data)
    ///
    /// This could/should be a const fn but they are not stable yet
    pub(crate) fn size(self) -> Option<u8> {
        match self {
            FieldType::Logical => Some(1),
            FieldType::Date => Some(8),
            FieldType::Integer => Some(std::mem::size_of::<i32>() as u8),
            FieldType::Currency => Some(std::mem::size_of::<f64>() as u8),
            FieldType::DateTime => Some(2 * std::mem::size_of::<i32>() as u8),
            FieldType::Double => Some(std::mem::size_of::<f64>() as u8),
            _ => None,
        }
    }
}

impl TryFrom<char> for FieldType {
    type Error = ErrorKind;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match Self::from(c) {
            Some(t) => Ok(t),
            None => Err(ErrorKind::InvalidFieldType(c)),
        }
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "dbase::{:?}", self)
    }
}

/// Enum where each variant stores the record value
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    // dBase III fields
    // Stored as strings, fully padded (ie only space char) strings
    // are interpreted as None
    /// dBase String type
    ///
    /// A string full of 'pad bytes' is considered `None`
    Character(Option<String>),
    /// dBase type to represent numbers, stored as String in the file
    Numeric(Option<f64>),
    /// dBase type for boolean values, stored as a character in the file
    Logical(Option<bool>),
    /// dBase type for dates, stored as a string in the file
    Date(Option<Date>),
    /// Another dBase type to represent numbers, stored as String in the file
    Float(Option<f32>),
    //Visual FoxPro fields
    Integer(i32),
    Currency(f64),
    DateTime(DateTime),
    Double(f64),

    /// Memo is a dBase type that allows to store Strings
    /// that are longer than 255 bytes.
    /// These strings are stored in an external file
    /// called the `Memo file`
    Memo(String),
}

impl FieldValue {
    pub(crate) fn read_from<T: Read + Seek, E: Encoding>(
        mut field_bytes: &[u8],
        memo_reader: &mut Option<MemoReader<T>>,
        field_info: &FieldInfo,
        encoding: &E,
        character_option: TrimOption,
    ) -> Result<Self, ErrorKind> {
        debug_assert_eq!(field_bytes.len(), field_info.length() as usize);
        let value = match field_info.field_type {
            FieldType::Logical => match field_bytes[0] as char {
                ' ' | '?' => FieldValue::Logical(None),
                '1' | '0' | 'T' | 't' | 'Y' | 'y' => FieldValue::Logical(Some(true)),
                'N' | 'n' | 'F' | 'f' => FieldValue::Logical(Some(false)),
                _ => FieldValue::Logical(None),
            },
            FieldType::Character => {
                // let value = read_string_of_len(&mut source, field_info.field_length)?;
                let value = trim_field_data(field_bytes, character_option);
                if value.is_empty() {
                    FieldValue::Character(None)
                } else {
                    FieldValue::Character(Some(encoding.decode(value)?.to_string()))
                }
            }
            FieldType::Numeric => {
                // let value = read_string_of_len(&mut source, field_info.field_length)?;
                let value = trim_field_data(field_bytes, TrimOption::BeginEnd);
                if value.is_empty() || value.iter().all(|c| c == &b'*') {
                    FieldValue::Numeric(None)
                } else {
                    let value_str = encoding.decode(value)?;
                    FieldValue::Numeric(Some(value_str.parse::<f64>()?))
                }
            }
            FieldType::Float => {
                // let value = read_string_of_len(&mut source, field_info.field_length)?;
                let value = trim_field_data(field_bytes, TrimOption::BeginEnd);
                if value.is_empty() || value.iter().all(|c| c == &b'*') {
                    FieldValue::Float(None)
                } else {
                    let value_str = encoding.decode(value)?;
                    FieldValue::Float(Some(value_str.parse::<f32>()?))
                }
            }
            FieldType::Date => {
                // let value = read_string_of_len(&mut source, field_info.field_length)?;
                let value = trim_field_data(field_bytes, TrimOption::BeginEnd);
                if value.iter().all(|c| c == &b' ') {
                    FieldValue::Date(None)
                } else {
                    let value_str = encoding.decode(value)?;
                    FieldValue::Date(Some(value_str.parse::<Date>()?))
                }
            }
            FieldType::Integer => {
                let mut le_bytes = [0u8; std::mem::size_of::<i32>()];
                le_bytes.copy_from_slice(&field_bytes[..std::mem::size_of::<i32>()]);
                FieldValue::Integer(i32::from_le_bytes(le_bytes))
            }
            FieldType::Double => {
                let mut le_bytes = [0u8; std::mem::size_of::<f64>()];
                le_bytes.copy_from_slice(&field_bytes[..std::mem::size_of::<f64>()]);
                FieldValue::Double(f64::from_le_bytes(le_bytes))
            }
            FieldType::Currency => {
                let mut le_bytes = [0u8; std::mem::size_of::<f64>()];
                le_bytes.copy_from_slice(&field_bytes[..std::mem::size_of::<f64>()]);
                FieldValue::Currency(f64::from_le_bytes(le_bytes))
            }
            FieldType::DateTime => {
                let mut source = std::io::Cursor::new(&mut field_bytes);
                FieldValue::DateTime(DateTime::read_from(&mut source)?)
            }
            FieldType::Memo => {
                let index_in_memo = if field_info.field_length > 4 {
                    // let string = read_string_of_len(&mut source, field_info.field_length)?;
                    let trimmed_value = trim_field_data(field_bytes, TrimOption::End);
                    if trimmed_value.is_empty() {
                        return Ok(FieldValue::Memo(String::from("")));
                    } else {
                        encoding.decode(trimmed_value)?.parse::<u32>()?
                        // string.parse::<u32>()?
                    }
                } else {
                    let mut le_bytes = [0u8; std::mem::size_of::<u32>()];
                    le_bytes.copy_from_slice(&field_bytes[..std::mem::size_of::<u32>()]);
                    u32::from_le_bytes(le_bytes)
                };

                if let Some(memo_reader) = memo_reader {
                    let data_from_memo = memo_reader.read_data_at(index_in_memo)?;
                    FieldValue::Memo(encoding.decode(data_from_memo)?.to_string())
                } else {
                    return Err(ErrorKind::MissingMemoFile);
                }
            }
        };
        Ok(value)
    }

    /// Returns the corresponding field type of the contained value
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::Character(_) => FieldType::Character,
            FieldValue::Numeric(_) => FieldType::Numeric,
            FieldValue::Logical(_) => FieldType::Logical,
            FieldValue::Integer(_) => FieldType::Integer,
            FieldValue::Float(_) => FieldType::Float,
            FieldValue::Double(_) => FieldType::Double,
            FieldValue::Date(_) => FieldType::Date,
            FieldValue::Memo(_) => FieldType::Memo,
            FieldValue::Currency(_) => FieldType::Currency,
            FieldValue::DateTime(_) => FieldType::DateTime,
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// dBase representation of date
///
/// # Note
///
/// This is really really naive date, it just holds the day, moth, year value
/// with just a very few checks.
///
/// Also, dBase files do not have concept of timezones.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Date {
    pub(crate) year: u32,
    pub(crate) month: u32,
    pub(crate) day: u32,
}

impl Date {
    /// Creates a new dbase::Date
    /// # panic
    ///
    /// panics if the year has more than 4 digits or if the day is greater than 31 or
    /// the month greater than 12
    pub const fn new(day: u32, month: u32, year: u32) -> Self {
        if year > 9999 {
            panic!("Year cannot have more than 4 digits")
        }
        if day > 31 {
            panic!("Day cannot be greater than 31")
        }
        if month > 12 {
            panic!("Month cannot be greater than 12")
        }
        Self { year, month, day }
    }

    /// Returns the year
    pub fn year(&self) -> u32 {
        self.year
    }

    /// Returns the month
    pub fn month(&self) -> u32 {
        self.month
    }

    /// Returns the day
    pub fn day(&self) -> u32 {
        self.day
    }

    pub fn to_unix_days(&self) -> i32 {
        let julian_day = self.to_julian_day_number();
        return julian_day - 2440588;
    }

    // https://en.wikipedia.org/wiki/Julian_day
    // at "Julian or Gregorian calendar from Julian day number"
    fn julian_day_number_to_gregorian_date(jdn: i32) -> Date {
        const Y: i32 = 4716;
        const J: i32 = 1401;
        const M: i32 = 2;
        const N: i32 = 12;
        const R: i32 = 4;
        const P: i32 = 1461;
        const V: i32 = 3;
        const U: i32 = 5;
        const S: i32 = 153;
        const W: i32 = 2;
        const B: i32 = 274_277;
        const C: i32 = -38;

        let f = jdn + J + ((4 * jdn + B) / 146_097 * 3) / 4 + C;
        let e = R * f + V;
        let g = (e % P) / R;
        let h = U * g + W;

        let day = (h % S) / U + 1;
        let month = ((h / S + M) % (N)) + 1;
        let year = (e / P) - Y + (N + M - month) / N;

        Date {
            year: year as u32,
            month: month as u32,
            day: day as u32,
        }
    }

    fn to_julian_day_number(&self) -> i32 {
        let (month, year) = if self.month > 2 {
            (self.month - 3, self.year)
        } else {
            (self.month + 9, self.year - 1)
        };

        let century = year / 100;
        let decade = year - 100 * century;

        ((146_097 * century) / 4
            + (1461 * decade) / 4
            + (153 * month + 2) / 5
            + self.day
            + 1_721_119) as i32
    }
}

impl FromStr for Date {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let year = s[0..4].parse::<u32>()?;
        let month = s[4..6].parse::<u32>()?;
        let day = s[6..8].parse::<u32>()?;

        Ok(Self { year, month, day })
    }
}

impl std::string::ToString for Date {
    fn to_string(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }
}

impl std::convert::TryFrom<Date> for time::Date {
    type Error = time::error::ComponentRange;

    fn try_from(d: Date) -> Result<Self, Self::Error> {
        let month: time::Month = (d.month as u8).try_into()?;
        Self::from_calendar_date(d.year as i32, month, d.day as u8)
    }
}

impl From<time::Date> for Date {
    fn from(d: time::Date) -> Self {
        let month: u8 = d.month().into();
        Self {
            year: d.year() as u32,
            month: month as u32,
            day: d.day() as u32,
        }
    }
}

/// FoxBase representation of a time
/// # note
///
/// This is a very naive Time struct, very minimal verifications are done.
///
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Time {
    hours: u32,
    minutes: u32,
    seconds: u32,
}

impl Time {
    const HOURS_FACTOR: i32 = 3_600_000;
    const MINUTES_FACTOR: i32 = 60_000;
    const SECONDS_FACTOR: i32 = 1_000;

    /// Creates a new Time
    ///
    /// # panics
    /// will panic if the  minutes or seconds are greater than 60 or
    /// if the hours are greater than 24
    pub fn new(hours: u32, minutes: u32, seconds: u32) -> Self {
        if hours > 24 || minutes > 60 || seconds > 60 {
            panic!("Invalid Time")
        }
        Self {
            hours,
            minutes,
            seconds,
        }
    }

    /// Returns the hours.
    pub fn hours(&self) -> u32 {
        self.hours
    }

    /// Returns the minutes.
    pub fn minutes(&self) -> u32 {
        self.minutes
    }

    /// Returns the seconds.
    pub fn seconds(&self) -> u32 {
        self.seconds
    }

    fn from_word(mut time_word: i32) -> Self {
        let hours: u32 = (time_word / Self::HOURS_FACTOR) as u32;
        time_word -= (hours * Self::HOURS_FACTOR as u32) as i32;
        let minutes: u32 = (time_word / Self::MINUTES_FACTOR) as u32;
        time_word -= (minutes * Self::MINUTES_FACTOR as u32) as i32;
        let seconds: u32 = (time_word / Self::SECONDS_FACTOR) as u32;
        Self {
            hours,
            minutes,
            seconds,
        }
    }

    fn to_time_word(&self) -> i32 {
        let mut time_word = self.hours * Self::HOURS_FACTOR as u32;
        time_word += self.minutes * Self::MINUTES_FACTOR as u32;
        time_word += self.seconds * Self::SECONDS_FACTOR as u32;
        time_word as i32
    }
}

/// FoxBase representation of a DateTime
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DateTime {
    date: Date,
    time: Time,
}

impl DateTime {
    /// Creates a new DateTime from a date and a time
    pub fn new(date: Date, time: Time) -> Self {
        Self { date, time }
    }

    /// Returns the [Date] part.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Returns the [Time] part.
    pub fn time(&self) -> Time {
        self.time
    }

    pub fn to_unix_timestamp(&self) -> i64 {
        return self.date().to_unix_days() as i64 * 86400
            + self.time().hours() as i64 * 3600
            + self.time().minutes() as i64 * 60
            + self.time().seconds() as i64;
    }

    fn read_from<T: Read>(src: &mut T) -> Result<Self, ErrorKind> {
        let julian_day_number = src.read_i32::<LittleEndian>()?;
        let time_word = src.read_i32::<LittleEndian>()?;
        let time = Time::from_word(time_word);
        let date = Date::julian_day_number_to_gregorian_date(julian_day_number);
        Ok(Self { date, time })
    }

    fn write_to<W: Write>(&self, dest: &mut W) -> std::io::Result<()> {
        dest.write_i32::<LittleEndian>(self.date.to_julian_day_number())?;
        dest.write_i32::<LittleEndian>(self.time.to_time_word())?;
        Ok(())
    }
}

impl WritableAsDbaseField for FieldValue {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if self.field_type() != field_info.field_type {
            Err(ErrorKind::IncompatibleType)
        } else {
            match self {
                FieldValue::Character(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Numeric(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Logical(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Date(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Float(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Integer(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Currency(value) => value.write_as(field_info, encoding, dst),
                FieldValue::DateTime(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Double(value) => value.write_as(field_info, encoding, dst),
                FieldValue::Memo(_) => unimplemented!("Cannot write memo"),
            }
        }
    }
}

impl WritableAsDbaseField for f64 {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        match field_info.field_type {
            FieldType::Numeric => {
                let string = format!(
                    "{value:.precision$}",
                    value = self,
                    precision = field_info.num_decimal_places as usize
                );
                let encoded_string = encoding.encode(&string)?;
                dst.write_all(&*encoded_string)?;
                Ok(())
            }
            FieldType::Currency | FieldType::Double => {
                dst.write_f64::<LittleEndian>(*self)?;
                Ok(())
            }
            _ => Err(ErrorKind::IncompatibleType),
        }
    }
}

impl WritableAsDbaseField for Date {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Date {
            let string = format!("{:04}{:02}{:02}", self.year, self.month, self.day);
            let encoded_string = encoding.encode(&string)?;
            dst.write_all(&*encoded_string)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for Option<Date> {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Date {
            if let Some(date) = self {
                date.write_as(field_info, encoding, dst)?;
            } else {
                for _ in 0..8 {
                    dst.write_u8(b' ')?;
                }
            }
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for Option<f64> {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Numeric {
            if let Some(value) = self {
                value.write_as(field_info, encoding, dst)
            } else {
                Ok(())
            }
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for f32 {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Float {
            let string = format!(
                "{value:.precision$}",
                value = self,
                precision = field_info.num_decimal_places as usize
            );
            let encoded_string = encoding.encode(&string)?;
            dst.write_all(&*encoded_string)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for Option<f32> {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Float {
            if let Some(value) = self {
                value.write_as(field_info, encoding, dst)?;
            }
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for String {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Character {
            let encoded_bytes = encoding.encode(self.as_str())?;
            dst.write_all(&*encoded_bytes)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for Option<String> {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Character {
            if let Some(s) = self {
                s.write_as(field_info, encoding, dst)?;
            }
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for &str {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Character {
            let encoded_bytes = encoding.encode(self)?;
            dst.write_all(&*encoded_bytes)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for bool {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Logical {
            if *self {
                let encoded_bytes = encoding.encode("t")?;
                dst.write_all(&*encoded_bytes)?;
            } else {
                let encoded_bytes = encoding.encode("f")?;
                dst.write_all(&*encoded_bytes)?;
            }
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for Option<bool> {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Logical {
            if let Some(v) = self {
                v.write_as(field_info, encoding, dst)?;
            }
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for i32 {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        _encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::Integer {
            dst.write_i32::<LittleEndian>(*self)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

impl WritableAsDbaseField for DateTime {
    fn write_as<E: Encoding, W: Write>(
        &self,
        field_info: &FieldInfo,
        _encoding: &E,
        dst: &mut W,
    ) -> Result<(), ErrorKind> {
        if field_info.field_type == FieldType::DateTime {
            self.write_to(dst)?;
            Ok(())
        } else {
            Err(ErrorKind::IncompatibleType)
        }
    }
}

#[cfg(feature = "serde")]
mod de {
    use super::*;
    use serde::de::{Deserialize, Visitor};
    use serde::Deserializer;
    use std::io::Cursor;

    impl<'de> Deserialize<'de> for Date {
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct DateVisitor;
            impl<'de> Visitor<'de> for DateVisitor {
                type Value = Date;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct Date")
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let string = String::from_utf8(v).unwrap();
                    Ok(Date::from_str(&string).unwrap())
                }
            }
            deserializer.deserialize_byte_buf(DateVisitor)
        }
    }

    struct DateTimeVisitor;

    impl<'de> Visitor<'de> for DateTimeVisitor {
        type Value = DateTime;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct dbase::DateTime")
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut cursor = Cursor::new(v);
            match DateTime::read_from(&mut cursor) {
                Ok(d) => Ok(d),
                Err(e) => Err(E::custom(e)),
            }
        }
    }

    impl<'de> Deserialize<'de> for DateTime {
        fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_byte_buf(DateTimeVisitor)
        }
    }
}

#[cfg(feature = "serde")]
mod ser {
    use super::*;

    use serde::ser::Serialize;
    use serde::Serializer;

    impl Serialize for Date {
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(self.to_string().as_bytes())
        }
    }

    impl Serialize for DateTime {
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            let mut bytes = [0u8; 8];
            bytes[..4].copy_from_slice(&self.date.to_julian_day_number().to_le_bytes());
            bytes[4..8].copy_from_slice(&self.time.to_time_word().to_le_bytes());
            serializer.serialize_bytes(&bytes)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TrimOption {
    Begin,
    End,
    BeginEnd,
}

fn trim_field_data(bytes: &[u8], option: TrimOption) -> &[u8] {
    // Value in the dbf file is surrounded by space characters (32u8). We discard them before
    // parsing the bytes into string. Doing so doubles the performance in comparison to
    // using String::trim() afterwards.
    let mut first = usize::MAX;
    let mut last = 0;
    let ptr = bytes.as_ptr();

    // Using unchecked indexing of the vector provides around 30% increase of reading speed.
    // SAFETY: index is always between 0 and bytes.len(), so using pointers here is safe.
    unsafe {
        for i in 0..bytes.len() {
            if *ptr.add(i) == 0u8 {
                break;
            }

            if *ptr.add(i) != 32 {
                if first == usize::MAX {
                    first = i;
                }

                last = i;
            }
        }
    }

    // Input starts with zero character or consists only of spaces.
    if first == usize::MAX {
        return &[];
    }

    // Discarding spaces in front and at the end. The space character (32u8) is 00110000 in binary
    // format, which makes it safe to drop without checking, as it cannot be part of the multi-byte
    // UTF-8 symbol (all such bytes must start with 10).
    match option {
        TrimOption::Begin => &bytes[first..],
        TrimOption::End => &bytes[..last + 1],
        TrimOption::BeginEnd => &bytes[first..last + 1],
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::encoding::UnicodeLossy;
    use crate::field::FieldFlags;
    use std::io::Cursor;

    fn create_temp_field_info(field_type: FieldType, len: u8) -> FieldInfo {
        FieldInfo {
            name: "".to_owned(),
            field_type,
            displacement_field: [0u8; 4],
            field_length: len,
            num_decimal_places: 0,
            flags: FieldFlags { 0: 0u8 },
            autoincrement_next_val: [0u8; 5],
            autoincrement_step: 0u8,
        }
    }

    fn test_we_can_read_back(field_info: &FieldInfo, value: &FieldValue) {
        let mut out = Cursor::new(Vec::<u8>::with_capacity(field_info.field_length as usize));
        let encoding = UnicodeLossy;

        value.write_as(field_info, &encoding, &mut out).unwrap();
        out.set_position(0);

        let read_value = FieldValue::read_from::<std::io::Cursor<Vec<u8>>, _>(
            out.get_mut(),
            &mut None,
            field_info,
            &encoding,
            TrimOption::BeginEnd,
        )
        .unwrap();
        assert_eq!(value, &read_value);
    }

    #[test]
    fn write_read_date() {
        let date = FieldValue::from(Date {
            year: 2019,
            month: 01,
            day: 01,
        });

        let field_info = create_temp_field_info(FieldType::Date, FieldType::Date.size().unwrap());
        test_we_can_read_back(&field_info, &date);
    }

    #[test]
    fn test_write_read_empty_date() {
        let date = FieldValue::Date(None);

        let field_info = create_temp_field_info(FieldType::Date, FieldType::Date.size().unwrap());
        test_we_can_read_back(&field_info, &date);
    }

    #[test]
    fn write_read_ascii_char() {
        let field = FieldValue::Character(Some(String::from("Only ASCII")));

        let record_info = create_temp_field_info(FieldType::Character, 10);
        test_we_can_read_back(&record_info, &field);
    }

    #[test]
    fn test_write_read_integer_via_enum() {
        use crate::field::FieldName;

        let value = FieldValue::Integer(1457);

        let field_info = FieldInfo::new(
            FieldName::try_from("Integer").unwrap(),
            FieldType::Integer,
            FieldType::Integer.size().unwrap(),
        );

        test_we_can_read_back(&field_info, &value);
    }

    #[test]
    fn test_from_julian_day_number() {
        let date = Date::julian_day_number_to_gregorian_date(2458685);
        assert_eq!(date.year, 2019);
        assert_eq!(date.month, 07);
        assert_eq!(date.day, 20);
    }

    #[test]
    fn test_to_julian_day_number() {
        let date = Date {
            year: 2019,
            month: 07,
            day: 20,
        };
        assert_eq!(date.to_julian_day_number(), 2458685);
    }

    #[test]
    fn test_to_unix_days() {
        let date = Date {
            year: 1970,
            month: 1,
            day: 1,
        };
        assert_eq!(date.to_unix_days(), 0);
    }

    #[test]
    fn test_to_unix_timestamp() {
        let datetime = DateTime::new(Date::new(1, 1, 1970), Time::new(1, 1, 1));
        assert_eq!(datetime.to_unix_timestamp(), 3661);
    }
}

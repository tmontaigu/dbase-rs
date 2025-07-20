use super::{types, FieldType, FieldValue};

/// Errors that can happen when trying to convert a FieldValue into
/// a more concrete type
#[derive(Debug)]
pub enum FieldConversionError {
    /// Happens when the conversion could not be mode because the FieldType
    /// does not mat the expected one
    FieldTypeNotAsExpected {
        /// The expected FieldType of the FieldValue the conversion was tried on
        expected: FieldType,
        /// The actual FieldType of the FieldValue the conversion was tried on
        actual: FieldType,
    },
    IncompatibleType,
    /// The value written is the file was only pad bytes / uninitialized
    /// and the user tried to convert it into a non Option-Type
    NoneValue,
}

impl std::fmt::Display for FieldConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldConversionError::FieldTypeNotAsExpected { expected, actual } => {
                write!(f, "Cannot convert from {expected} to {actual}")
            }
            FieldConversionError::IncompatibleType => write!(f, "The type is not compatible"),
            FieldConversionError::NoneValue => {
                write!(f, "Value is not initialized, which is not allowed")
            }
        }
    }
}

impl std::error::Error for FieldConversionError {}

macro_rules! impl_try_from_field_value_for_ {
    (FieldValue::$variant:ident => $out_type:ty) => {
        impl TryFrom<FieldValue> for $out_type {
            type Error = FieldConversionError;

            fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
                if let FieldValue::$variant(v) = value {
                    Ok(v)
                } else {
                    Err(FieldConversionError::FieldTypeNotAsExpected {
                        expected: FieldType::$variant,
                        actual: value.field_type(),
                    })
                }
            }
        }
    };
    (FieldValue::$variant:ident(Some($v:ident)) => $out_type:ty) => {
        impl TryFrom<FieldValue> for $out_type {
            type Error = FieldConversionError;

            fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
                match value {
                    FieldValue::$variant(Some($v)) => Ok($v),
                    FieldValue::$variant(None) => Err(FieldConversionError::NoneValue),
                    _ => Err(FieldConversionError::FieldTypeNotAsExpected {
                        expected: FieldType::$variant,
                        actual: value.field_type(),
                    }),
                }
            }
        }
    };
}

impl_try_from_field_value_for_!(FieldValue::Numeric => Option<f64>);

impl_try_from_field_value_for_!(FieldValue::Float => Option<f32>);
impl_try_from_field_value_for_!(FieldValue::Float(Some(v)) => f32);

impl_try_from_field_value_for_!(FieldValue::Date => Option<types::Date>);
impl_try_from_field_value_for_!(FieldValue::Date(Some(v)) => types::Date);

impl_try_from_field_value_for_!(FieldValue::Character => Option<String>);
impl_try_from_field_value_for_!(FieldValue::Character(Some(string)) => String);

impl_try_from_field_value_for_!(FieldValue::Logical => Option<bool>);
impl_try_from_field_value_for_!(FieldValue::Logical(Some(b)) => bool);

impl_try_from_field_value_for_!(FieldValue::Integer => i32);

impl TryFrom<FieldValue> for f64 {
    type Error = FieldConversionError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        match value {
            FieldValue::Numeric(Some(v)) => Ok(v),
            FieldValue::Numeric(None) => Err(FieldConversionError::NoneValue),
            FieldValue::Currency(c) => Ok(c),
            FieldValue::Double(d) => Ok(d),
            _ => Err(FieldConversionError::IncompatibleType),
        }
    }
}

// Fox Pro types
impl_try_from_field_value_for_!(FieldValue::DateTime => types::DateTime);

macro_rules! impl_from_type_for_field_value (
    ($t:ty => FieldValue::$variant:ident) => {
        impl From<$t> for FieldValue {
            fn from(v: $t) -> Self {
                FieldValue::$variant(v)
            }
        }
    };
    ($t:ty => FieldValue::$variant:ident(Some($v:ident))) => {
        impl From<$t> for FieldValue {
            fn from(v: $t) -> Self {
                FieldValue::$variant(Some(v))
            }
        }
    }
);

impl_from_type_for_field_value!(Option<String> => FieldValue::Character);
impl_from_type_for_field_value!(String => FieldValue::Character(Some(s)));

impl_from_type_for_field_value!(Option<f64> => FieldValue::Numeric);
impl_from_type_for_field_value!(f64 => FieldValue::Numeric(Some(v)));

impl_from_type_for_field_value!(Option<f32> => FieldValue::Float);
impl_from_type_for_field_value!(f32 => FieldValue::Float(Some(v)));

impl_from_type_for_field_value!(Option<bool> => FieldValue::Logical);
impl_from_type_for_field_value!(bool => FieldValue::Logical(Some(v)));

impl_from_type_for_field_value!(Option<types::Date> => FieldValue::Date);
impl_from_type_for_field_value!(types::Date => FieldValue::Date(Some(v)));

// Fox Pro types
impl_from_type_for_field_value!(types::DateTime => FieldValue::DateTime);

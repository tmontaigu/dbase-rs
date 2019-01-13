use std::io::{Seek, SeekFrom, Read};
use std::fs::File;

extern crate nom;
extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt};

use nom::*;


struct Reader<T: Read> {
    source: T
}

named!(parse_header<(i32, i16, i16)>,
   do_parse!(
       take!(1) >>   // level?
       take!(3) >>   // date last modified
       num_recs: le_i32         >>
       bytes_in_header: le_i16  >>
       bytes_in_rec: le_i16     >>
       take!(2) >>  // res. fill w/ zero
       take!(1) >>  // flag: incomplete transaction
       take!(1) >>  // encryption flag
       take!(12) >> // res. multi-user proc.
       take!(1) >>  // prod. mdx flag
       take!(1) >>  // lang. drv. id
       take!(2) >>  // res.

       ( (num_recs, bytes_in_header, bytes_in_rec) )
   )
);


#[derive(Debug)]
enum FieldType {
    Character,
    Currency,
    Numeric,
    Float,
    Date,
    DateTime,
    Double,
    Integer,
    Logical,
    Memo,
    General,
    BinaryCharacter,
    BinaryMemo,
    Picture,
    Varbinary,
    BinaryVarchar,
}

impl FieldType {
    fn from(c: char) -> Option<FieldType> {
        match c {
            'C' => Some(FieldType::Character),
            'Y' => Some(FieldType::Currency),
            'N' => Some(FieldType::Numeric),
            'F' => Some(FieldType::Float),
            'D' => Some(FieldType::Date),
            'T' => Some(FieldType::DateTime),
            'B' => Some(FieldType::Double),
            'I' => Some(FieldType::Integer),
            'L' => Some(FieldType::Logical),
            'M' => Some(FieldType::Memo),
            'G' => Some(FieldType::General),
            //'C' => Some(FieldType::BinaryCharacter), ??
            //'M' => Some(FieldType::BinaryMemo),
            _  => None,
        }
    }
}

struct Date {
    yea
}

#[derive(Debug)]
enum FieldValue {
    Character(String),
    Float(f32),
    Double(f64),
    Integer(i32),
    Numeric(String),
    Logical(bool),
}


struct RecordFieldInfo {
    name: String,
    field_type: FieldType,
    record_length: u8,
    num_decimal_places: u8,
}


impl RecordFieldInfo {
    fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let mut name = [0u8; 11];
        source.read_exact(&mut name)?;
        let field_type = source.read_u8()?;

        let mut displacement_field = [0u8; 4];
        source.read_exact(&mut displacement_field)?;

        let record_length = source.read_u8()?;
        let num_decimal_places = source.read_u8()?;

        let mut skip = [0u8; 14];
        source.read_exact(&mut skip)?;


        let s = String::from_utf8_lossy(&name).into_owned();
        let field_type = FieldType::from(field_type as char).unwrap();
        Ok(Self{
            name: s,
            field_type,
            record_length,
            num_decimal_places
        })
    }
}

fn read_string_of_len<T: Read>(source: &mut T, len: u8) -> Result<String, std::io::Error> {
    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0u8);
    source.read_exact(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

use std::collections::HashMap;

type Record = HashMap<String, Vec<FieldValue>>;

fn read() {
    let mut f = File::open("").unwrap();
    //let reader = Reader{source: f};

    // NOM
    //let mut header_bytes= [0u8; 32];
    //f.read_exact(&mut header_bytes).unwrap(); //level + last date
    //let (_, (num_recs, bytes_in_header, bytes_in_rec)) = parse_header(&header_bytes).unwrap();

    // HEADER READING
    let mut skip = [0u8; 4];
    f.read_exact(&mut skip).unwrap(); //level + last date
    let num_records = f.read_u32::<LittleEndian>().unwrap();
    let offset_to_first_record = f.read_u16::<LittleEndian>().unwrap();
    let size_of_record = f.read_u16::<LittleEndian>().unwrap();
    let mut skip = [0u8; 20];
    f.read_exact(&mut skip).unwrap(); //level + last date

    assert_eq!(f.seek(SeekFrom::Current(0)).unwrap(), 32);

    let numfields = (offset_to_first_record - 32) / 32;
    println!("Numfields: {}", numfields);


    let mut fields_info = Vec::<RecordFieldInfo>::with_capacity(numfields as usize + 1);
    fields_info.push(RecordFieldInfo{
        name: "DeletionFlag".to_string(),
        field_type: FieldType::Character,
        record_length: 1,
        num_decimal_places: 0
    });
    for i in 0..numfields {
        let info = RecordFieldInfo::read_from(&mut f).unwrap();
        println!("{} -> {}, {:?}, length: {}", i, info.name, info.field_type, info.record_length);
        fields_info.push(info);
        assert_eq!(f.seek(SeekFrom::Current(0)).unwrap(), 32 + ((i as u64 + 1) * 32));
    }

    let terminator = f.read_u8().unwrap() as char;
    println!("terminator: {}", terminator);


    //if terminator != 'r' {
    //  panic!("unexpected terminator");
    //}
    let records = Vec::<Record>::with_capacity(num_records as usize);
    for _ in 0..num_records {
        let mut current_record = Vec::<FieldValue>::with_capacity(numfields as usize);
        for field_info in &fields_info {
            let value = match field_info.field_type {
                FieldType::Logical => {
                    let value = f.read_u8().unwrap() as char;
                    match value {
                        '1' | 'T' | 't' | 'Y' | 'y' => FieldValue::Logical(true),
                        _ => FieldValue::Logical(false),
                    }
                },
                FieldType::Integer => {
                    let string = read_string_of_len(&mut f, field_info.record_length).unwrap();
                    FieldValue::Integer(string.parse::<i32>().unwrap())
                },
                FieldType::Float => FieldValue::Float(f.read_f32::<LittleEndian>().unwrap()),
                FieldType::Double => FieldValue::Double(f.read_f64::<LittleEndian>().unwrap()),
                FieldType::Character => FieldValue::Character(read_string_of_len(&mut f, field_info.record_length).unwrap()),
                FieldType::Numeric => {
                    let value = read_string_of_len(&mut f, field_info.record_length).unwrap();
                    //println!("numeric value: '{}'", value.trim());
                    //FieldValue::Numeric(value.trim().parse::<f64>().unwrap())
                    FieldValue::Numeric(value.trim().to_owned())
                },
                _ => panic!("unhandled type")
            };q
            //println!("{:?}", value);
            current_record.push(value);
        }
    }
    println!("Pos after reading: {}", f.seek(SeekFrom::Current(0)).unwrap());
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        read()
    }
}

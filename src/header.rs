use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::encoding::DynEncoding;
use std::io::{Read, Write};

use crate::field::field::{Date, MemoFileType};

// Used this as source: https://blog.codetitans.pl/post/dbf-and-language-code-page/
// also https://github.com/ethanfurman/dbf/blob/4f8ff35bec18ca167981ba741bfe353f5f362f99/dbf/__init__.py#L8299
#[derive(Copy, Clone, Debug)]
pub enum CodePageMark {
    Undefined,
    // OEM United States
    CP437,
    // OEM Multilingual Latin 1; Western European (DOS
    CP850,
    // ANSI Latin 1; Western European (Windows)
    CP1252,
    // StandardMacIntosh, // 10000
    // OEM Latin 2; Central European (DOS)
    CP852, // 852
    // OEM Russian; Cyrillic (DOS)
    CP866,
    // OEM Nordic; Nordic (DOS)
    CP865,
    // OEM Icelandic; Icelandic (DOS)
    CP861,
    CP895,
    CP620,
    CP737,
    CP857,
    CP950,
    CP949,
    CP936,
    CP932,
    // Thai (Windows)
    CP874,
    // ANSI Hebrew; Hebrew (Windows)
    CP1255,
    // ANSI Arabic; Arabic (Windows)
    CP1256, // 1256
    // RussianMacIntosh, // 10007
    // MacIntoshEE,      // 10029
    // GreekMacIntosh,   // 10006
    // ANSI Central European; Central European (Windows)
    CP1250,
    // ANSI Cyrillic; Cyrillic (Windows)
    CP1251,
    // ANSI Turkish; Turkish (Windows)
    CP1254,
    // ANSI Greek; Greek (Windows)
    CP1253,
    Utf8,
    Invalid,
}

impl CodePageMark {
    pub(crate) fn to_encoding(self) -> Option<DynEncoding> {
        #[cfg(feature = "yore")]
        {
            use crate::encoding::{LossyCodePage, Unicode};
            use yore::code_pages;
            Some(match self {
                CodePageMark::CP437 => DynEncoding::new(code_pages::CP437),
                CodePageMark::CP850 => DynEncoding::new(code_pages::CP850),
                CodePageMark::CP1252 => DynEncoding::new(code_pages::CP1252),
                CodePageMark::CP852 => DynEncoding::new(code_pages::CP852),
                CodePageMark::CP866 => DynEncoding::new(code_pages::CP866),
                CodePageMark::CP865 => DynEncoding::new(code_pages::CP865),
                CodePageMark::CP861 => DynEncoding::new(code_pages::CP861),
                CodePageMark::CP874 => DynEncoding::new(code_pages::CP874),
                CodePageMark::CP1255 => DynEncoding::new(code_pages::CP1255),
                CodePageMark::CP1256 => DynEncoding::new(code_pages::CP1256),
                CodePageMark::CP1250 => DynEncoding::new(code_pages::CP1250),
                CodePageMark::CP1251 => DynEncoding::new(code_pages::CP1251),
                CodePageMark::CP1254 => DynEncoding::new(code_pages::CP1254),
                CodePageMark::CP1253 => DynEncoding::new(code_pages::CP1253),
                CodePageMark::Utf8 => DynEncoding::new(Unicode),
                CodePageMark::Undefined | CodePageMark::Invalid => {
                    DynEncoding::new(LossyCodePage(code_pages::CP1252))
                }
                CodePageMark::CP895
                | CodePageMark::CP620
                | CodePageMark::CP737
                | CodePageMark::CP857
                | CodePageMark::CP950
                | CodePageMark::CP949
                | CodePageMark::CP936
                | CodePageMark::CP932 => {
                    return None;
                }
            })
        }
        #[cfg(not(feature = "yore"))]
        {
            Some(DynEncoding::new(crate::encoding::UnicodeLossy))
        }
    }
}

impl From<u8> for CodePageMark {
    fn from(code: u8) -> Self {
        match code {
            0x00 => Self::Undefined,
            0x01 => Self::CP437,
            0x02 => Self::CP850,
            0x03 => Self::CP1252,
            // 0x04 => Self::StandardMacIntosh,
            0x64 => Self::CP852,
            0x65 => Self::CP866,
            0x66 => Self::CP865,
            0x67 => Self::CP861,
            0x68 => Self::CP895,
            0x69 => Self::CP620,
            0x6A => Self::CP737,
            0x6B => Self::CP857,
            0x78 => Self::CP950,
            0x79 => Self::CP949,
            0x7A => Self::CP936,
            0x7B => Self::CP932,
            0x7C => Self::CP874,
            0x7D => Self::CP1255,
            0x7E => Self::CP1256,
            // 0x96 => Self::RussianMacIntosh,
            // 0x98 => Self::GreekMacIntosh,
            0xC8 => Self::CP1250,
            0xC9 => Self::CP1251,
            0xCA => Self::CP1254,
            0xCB => Self::CP1253,
            0xf0 => Self::Utf8,
            _ => Self::Invalid,
        }
    }
}

impl From<CodePageMark> for u8 {
    fn from(code: CodePageMark) -> Self {
        match code {
            CodePageMark::Undefined => 0x00,
            CodePageMark::CP437 => 0x01,
            CodePageMark::CP850 => 0x02,
            CodePageMark::CP1252 => 0x03,
            // CodePageMark::StandardMacIntosh => 0x04,
            CodePageMark::CP852 => 0x64,
            CodePageMark::CP866 => 0x65,
            CodePageMark::CP865 => 0x66,
            CodePageMark::CP861 => 0x67,
            CodePageMark::CP895 => 0x68,
            CodePageMark::CP620 => 0x69,
            CodePageMark::CP737 => 0x6A,
            CodePageMark::CP857 => 0x6B,
            CodePageMark::CP950 => 0x78,
            CodePageMark::CP949 => 0x79,
            CodePageMark::CP936 => 0x7A,
            CodePageMark::CP932 => 0x7B,
            CodePageMark::CP874 => 0x7C,
            CodePageMark::CP1255 => 0x7D,
            CodePageMark::CP1256 => 0x7E,
            // CodePageMark::RussianMacIntosh => 0x96,
            // CodePageMark::GreekMacIntosh => 0x98,
            CodePageMark::CP1250 => 0xC8,
            CodePageMark::CP1251 => 0xC9,
            CodePageMark::CP1254 => 0xCA,
            CodePageMark::CP1253 => 0xCB,
            CodePageMark::Utf8 => 0xf0,
            _ => 0,
        }
    }
}

/// Known version of dBase files
#[derive(Debug, Copy, Clone)]
pub enum Version {
    FoxBase,
    DBase3 { supports_memo: bool },
    VisualFoxPro,
    DBase4 { supports_memo: bool },
    FoxPro2 { supports_memo: bool },
    Unknown(u8),
}

impl Version {
    pub(crate) fn supported_memo_type(self) -> Option<MemoFileType> {
        match self {
            Version::FoxBase => Some(MemoFileType::FoxBaseMemo),
            Version::DBase3 {
                supports_memo: true,
            } => Some(MemoFileType::DbaseMemo),
            Version::DBase3 {
                supports_memo: false,
            } => None,
            Version::VisualFoxPro => Some(MemoFileType::FoxBaseMemo),
            Version::DBase4 {
                supports_memo: true,
            } => Some(MemoFileType::DbaseMemo4),
            Version::DBase4 {
                supports_memo: false,
            } => None,
            Version::FoxPro2 {
                supports_memo: false,
            } => None,
            Version::FoxPro2 {
                supports_memo: true,
            } => Some(MemoFileType::FoxBaseMemo),
            _ => None,
        }
    }

    pub(crate) fn is_visual_fox_pro(self) -> bool {
        matches!(self, Version::VisualFoxPro)
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> u8 {
        match v {
            Version::FoxBase => 0x02,
            Version::DBase3 {
                supports_memo: false,
            } => 0x03,
            Version::DBase3 {
                supports_memo: true,
            } => 0x83,
            Version::VisualFoxPro => 0x30,
            Version::DBase4 {
                supports_memo: true,
            } => 0x8b,
            Version::DBase4 {
                supports_memo: false,
            } => 0x43,
            Version::FoxPro2 {
                supports_memo: false,
            } => 0xfb,
            Version::FoxPro2 {
                supports_memo: true,
            } => 0xf5,
            Version::Unknown(v) => v,
        }
    }
}

impl From<u8> for Version {
    fn from(b: u8) -> Self {
        match b {
            0x02 => Version::FoxBase,
            0x03 => Version::DBase3 {
                supports_memo: false,
            },
            0x83 => Version::DBase3 {
                supports_memo: true,
            },
            // Each version has different feature (varchar / autoincrement)
            // but we don't support that for now
            0x30 | 0x31 | 0x32 => Version::VisualFoxPro,
            // Same here these different version num means that some features are different
            0x8b | 0xcb => Version::DBase4 {
                supports_memo: true,
            },
            0x43 | 0x63 => Version::DBase4 {
                supports_memo: false,
            },
            0xfb => Version::FoxPro2 {
                supports_memo: false,
            },
            0xf5 => Version::FoxPro2 {
                supports_memo: true,
            },
            b => Version::Unknown(b),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TableFlags(u8);

impl TableFlags {
    pub fn has_structural_cdx(&self) -> bool {
        (self.0 & 0x01) == 1
    }

    pub fn has_memo_field(&self) -> bool {
        (self.0 & 0x02) == 2
    }

    pub fn is_a_database(&self) -> bool {
        (self.0 & 0x03) == 1
    }
}

/// Definition of the header struct stored at the beginning
/// of each dBase file
#[derive(Debug, Copy, Clone)]
pub struct Header {
    pub file_type: Version,
    pub last_update: Date,
    pub num_records: u32,
    pub offset_to_first_record: u16,
    pub size_of_record: u16,
    pub is_transaction_incomplete: bool,
    pub encryption_flag: u8,
    pub table_flags: TableFlags,
    pub code_page_mark: CodePageMark,
}

impl Header {
    pub(crate) const SIZE: usize = 32;

    pub(crate) fn new(num_records: u32, offset: u16, size_of_records: u16) -> Self {
        let current_date = Self::get_today_date();
        Self {
            file_type: Version::DBase3 {
                supports_memo: false,
            },
            last_update: current_date,
            num_records,
            offset_to_first_record: offset,
            size_of_record: size_of_records,
            is_transaction_incomplete: false,
            encryption_flag: 0,
            table_flags: TableFlags(0),
            code_page_mark: CodePageMark::Undefined,
        }
    }

    fn get_today_date() -> Date {
        let current_date = time::OffsetDateTime::now_utc().date();
        // The year will be saved a a u8 offset from 1900
        if current_date.year() < 1900 || current_date.year() > 2155 {
            panic!("the year current date is out of range");
        } else {
            current_date.into()
        }
    }

    pub(crate) fn update_date(&mut self) {
        self.last_update = Self::get_today_date();
    }

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let file_type = Version::from(source.read_u8()?);

        let mut date_bytes = [0u8; 3];
        source.read_exact(&mut date_bytes)?;
        let last_update = Date {
            year: 1900u32 + date_bytes[0] as u32,
            month: date_bytes[1] as u32,
            day: date_bytes[2] as u32,
        };

        let num_records = source.read_u32::<LittleEndian>()?;
        let offset_to_first_record = source.read_u16::<LittleEndian>()?;
        let size_of_record = source.read_u16::<LittleEndian>()?;

        let _reserved = source.read_u16::<LittleEndian>()?;

        let is_transaction_incomplete = (source.read_u8()? != 0) as bool;
        let encryption_flag = source.read_u8()?;

        let mut _reserved = [0u8; 12];
        source.read_exact(&mut _reserved)?;

        let table_flags = TableFlags(source.read_u8()?);

        let code_page_mark = source.read_u8().map(From::from)?;

        let _reserved = source.read_u8()?;
        let _reserved = source.read_u8()?;

        Ok(Self {
            file_type,
            last_update,
            num_records,
            offset_to_first_record,
            is_transaction_incomplete,
            encryption_flag,
            size_of_record,
            table_flags,
            code_page_mark,
        })
    }

    pub(crate) fn write_to<T: Write>(&self, dest: &mut T) -> std::io::Result<()> {
        dest.write_u8(u8::from(self.file_type))?;

        dest.write_u8((self.last_update.year() - 1900) as u8)?;
        dest.write_u8(self.last_update.month() as u8)?;
        dest.write_u8(self.last_update.day() as u8)?;

        dest.write_u32::<LittleEndian>(self.num_records)?;
        dest.write_u16::<LittleEndian>(self.offset_to_first_record)?;
        dest.write_u16::<LittleEndian>(self.size_of_record)?;

        // Reserved
        dest.write_u16::<LittleEndian>(0)?;
        dest.write_u8(u8::from(self.is_transaction_incomplete))?;
        dest.write_u8(self.encryption_flag)?;

        let _reserved = [0u8; 12];
        dest.write_all(&_reserved)?;

        dest.write_u8(self.table_flags.0)?;
        dest.write_u8(self.code_page_mark.into())?;
        // Reserved
        dest.write_u8(0)?;
        dest.write_u8(0)?;
        Ok(())
    }

    pub(crate) fn record_position(&self, index: usize) -> Option<usize> {
        if index >= self.num_records as usize {
            None
        } else {
            let offset =
                self.offset_to_first_record as usize + (index * self.size_of_record as usize);
            Some(offset)
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::{Cursor, Seek, SeekFrom};

    use super::*;

    #[test]
    fn pos_after_reading_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();
        let _hdr = Header::read_from(&mut file).unwrap();
        let pos_after_reading = file.seek(SeekFrom::Current(0)).unwrap();
        assert_eq!(pos_after_reading, Header::SIZE as u64);
    }

    #[test]
    fn pos_after_writing_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();
        let hdr = Header::read_from(&mut file).unwrap();

        let mut out = Cursor::new(Vec::<u8>::with_capacity(Header::SIZE));
        hdr.write_to(&mut out).unwrap();
        let pos_after_writing = out.seek(SeekFrom::Current(0)).unwrap();
        assert_eq!(pos_after_writing, Header::SIZE as u64);
    }

    #[test]
    fn read_write_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();

        let mut hdr_bytes = [0u8; Header::SIZE];
        file.read_exact(&mut hdr_bytes).unwrap();
        let hdr_bytes: Vec<u8> = hdr_bytes.to_vec();

        let mut cursor = Cursor::new(hdr_bytes);
        let hdr = Header::read_from(&mut cursor).unwrap();
        let hdr_bytes = cursor.into_inner();

        let mut cursor = Cursor::new(Vec::<u8>::with_capacity(Header::SIZE));
        hdr.write_to(&mut cursor).unwrap();
        let hdr_bytes_written = cursor.into_inner();

        assert_eq!(hdr_bytes_written, hdr_bytes);
    }
}

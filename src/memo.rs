use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

/// The different types of Memo file structure there seem to exist
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum MemoFileType {
    DbaseMemo,
    DbaseMemo4,
    FoxBaseMemo,
}

/// Although there are different memo file type with each a different
/// header organisation, we use the same struct internally
#[derive(Debug, Copy, Clone)]
pub(crate) struct MemoHeader {
    next_available_block_index: u32,
    block_size: u32,
}

impl MemoHeader {
    pub(crate) fn read_from<R: Read>(
        src: &mut R,
        memo_type: MemoFileType,
    ) -> std::io::Result<Self> {
        let next_available_block_index = src.read_u32::<LittleEndian>()?;
        let block_size = match memo_type {
            MemoFileType::DbaseMemo | MemoFileType::DbaseMemo4 => {
                match src.read_u16::<LittleEndian>()? {
                    0 => 512,
                    v => u32::from(v),
                }
            }
            MemoFileType::FoxBaseMemo => {
                let _ = src.read_u16::<BigEndian>();
                u32::from(src.read_u16::<BigEndian>()?)
            }
        };

        Ok(Self {
            next_available_block_index,
            block_size,
        })
    }
}

/// Struct that reads knows how to read data from a memo source
#[derive(Debug, Clone)]
pub(crate) struct MemoReader<T: Read + Seek> {
    memo_file_type: MemoFileType,
    header: MemoHeader,
    source: T,
    internal_buffer: Vec<u8>,
}

impl<T: Read + Seek> MemoReader<T> {
    pub(crate) fn new(memo_type: MemoFileType, mut src: T) -> std::io::Result<Self> {
        let header = MemoHeader::read_from(&mut src, memo_type)?;
        let internal_buffer = vec![0u8; header.block_size as usize];
        Ok(Self {
            memo_file_type: memo_type,
            header,
            source: src,
            internal_buffer,
        })
    }

    pub(crate) fn read_data_at(&mut self, index: u32) -> std::io::Result<&[u8]> {
        let byte_offset = index * self.header.block_size;
        self.source.seek(SeekFrom::Start(u64::from(byte_offset)))?;

        match self.memo_file_type {
            MemoFileType::FoxBaseMemo => {
                let _type = self.source.read_u32::<BigEndian>()?;
                let length = self.source.read_u32::<BigEndian>()?;
                if length as usize > self.internal_buffer.len() {
                    self.internal_buffer.resize(length as usize, 0);
                }
                let buf_slice = &mut self.internal_buffer[..length as usize];
                self.source.read_exact(buf_slice)?;
                match buf_slice.iter().rposition(|b| *b != 0) {
                    Some(pos) => Ok(&buf_slice[..=pos]),
                    None => {
                        if buf_slice.iter().all(|b| *b == 0) {
                            Ok(&buf_slice[..0])
                        } else {
                            Ok(buf_slice)
                        }
                    }
                }
            }
            MemoFileType::DbaseMemo4 => {
                let _ = self.source.read_u32::<LittleEndian>()?;
                let length = self.source.read_u32::<LittleEndian>()?;
                self.source
                    .read_exact(&mut self.internal_buffer[..length as usize])?;
                match self.internal_buffer[..length as usize]
                    .iter()
                    .position(|b| *b == 0x1F)
                {
                    Some(pos) => Ok(&self.internal_buffer[..pos]),
                    None => Ok(&self.internal_buffer),
                }
            }
            MemoFileType::DbaseMemo => {
                if let Err(e) = self.source.read_exact(&mut self.internal_buffer) {
                    if index != self.header.next_available_block_index - 1
                        && e.kind() != std::io::ErrorKind::UnexpectedEof
                    {
                        return Err(e);
                    }
                }
                match self.internal_buffer.iter().position(|b| *b == 0x1A) {
                    Some(pos) => Ok(&self.internal_buffer[..pos]),
                    None => Ok(&self.internal_buffer),
                }
            }
        }
    }
}

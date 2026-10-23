//! Bounded dependency hints, not a Windows loader or a compatibility verdict.
use super::assessment::Library;
use crate::{require, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Imports {
    pub machine: u16,
    pub ordinary: Vec<Library>,
    pub delayed: Vec<Library>,
}

struct Image<'a, R> {
    source: &'a mut R,
    length: u64,
    headers: u64,
    sections: Vec<(u64, u64, u64)>,
}
impl<R: Read + Seek> Image<'_, R> {
    fn bytes(&mut self, offset: u64, count: usize) -> Result<Vec<u8>> {
        require(
            offset
                .checked_add(count as u64)
                .is_some_and(|end| end <= self.length),
            "graphics_pe_range",
        )?;
        self.source.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; count];
        self.source.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    fn at(&mut self, rva: u64, count: usize) -> Result<Vec<u8>> {
        let end = rva.checked_add(count as u64).ok_or("graphics_pe_range")?;
        let mut offsets = Vec::new();
        if end <= self.headers {
            offsets.push(rva);
        }
        for &(start, length, offset) in &self.sections {
            if rva >= start && end <= start + length {
                offsets.push(offset + rva - start);
            }
        }
        require(offsets.len() == 1, "graphics_pe_unmapped_or_ambiguous_rva")?;
        self.bytes(offsets[0], count)
    }
    fn name(&mut self, rva: u64) -> Result<String> {
        let mut name = Vec::new();
        for index in 0..260 {
            let byte = self.at(rva + index, 1)?[0];
            if byte == 0 {
                require(!name.is_empty(), "graphics_pe_import_name")?;
                return Ok(String::from_utf8(name)?);
            }
            require(
                byte.is_ascii_alphanumeric() || b"._-".contains(&byte),
                "graphics_pe_import_name",
            )?;
            name.push(byte.to_ascii_lowercase());
        }
        Err("graphics_pe_import_name_bound".into())
    }
    fn imports(&mut self, rva: u32, size: u32, delayed: bool, base: u64) -> Result<Vec<Library>> {
        if rva == 0 && size == 0 {
            return Ok(Vec::new());
        }
        let stride = if delayed { 32 } else { 20 };
        require(
            rva != 0 && size as usize >= stride && size <= 65536,
            "graphics_pe_directory_bound",
        )?;
        let mut found = BTreeSet::new();
        for index in 0..(size as usize / stride).min(1025) {
            let row = self.at(u64::from(rva) + (index * stride) as u64, stride)?;
            if row.iter().all(|v| *v == 0) {
                return Ok(found.into_iter().collect());
            }
            require(index < 1024, "graphics_pe_import_count")?;
            let name = if delayed {
                let flags = u32at(&row, 0);
                require(flags <= 1, "graphics_pe_delay_attributes")?;
                let address = u64::from(u32at(&row, 4));
                if flags == 1 {
                    address
                } else {
                    address
                        .checked_sub(base)
                        .ok_or("graphics_pe_delay_address")?
                }
            } else {
                u64::from(u32at(&row, 12))
            };
            if let Some(library) = Library::from_dll(&self.name(name)?) {
                found.insert(library);
            }
        }
        Err("graphics_pe_import_terminator".into())
    }
}
fn u16at(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes(b[p..p + 2].try_into().unwrap())
}
fn u32at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
pub fn inspect(source: &mut (impl Read + Seek)) -> Result<Imports> {
    let length = source.seek(SeekFrom::End(0))?;
    require(
        (64..=2_147_483_648).contains(&length),
        "graphics_pe_file_bound",
    )?;
    let mut image = Image {
        source,
        length,
        headers: 0,
        sections: Vec::new(),
    };
    let dos = image.bytes(0, 64)?;
    require(&dos[..2] == b"MZ", "graphics_pe_dos")?;
    let pe = u64::from(u32at(&dos, 60));
    let coff = image.bytes(pe, 24)?;
    require(&coff[..4] == b"PE\0\0", "graphics_pe_signature")?;
    let count = usize::from(u16at(&coff, 6));
    let optsize = usize::from(u16at(&coff, 20));
    require(
        (1..=96).contains(&count) && (96..=4096).contains(&optsize),
        "graphics_pe_headers",
    )?;
    let opt = image.bytes(pe + 24, optsize)?;
    let (directory, base) = match u16at(&opt, 0) {
        0x10b => (96, u64::from(u32at(&opt, 28))),
        0x20b if opt.len() >= 112 => (112, u64::from_le_bytes(opt[24..32].try_into()?)),
        _ => return Err("graphics_pe_optional_header".into()),
    };
    image.headers = u64::from(u32at(&opt, 60));
    let table = pe + 24 + optsize as u64;
    require(
        image.headers >= table + (count * 40) as u64 && image.headers <= length,
        "graphics_pe_headers",
    )?;
    for index in 0..count {
        let row = image.bytes(table + (index * 40) as u64, 40)?;
        let (start, size, offset) = (
            u64::from(u32at(&row, 12)),
            u64::from(u32at(&row, 16)),
            u64::from(u32at(&row, 20)),
        );
        require(
            offset + size <= length && start + size <= u64::from(u32::MAX),
            "graphics_pe_section",
        )?;
        image.sections.push((start, size, offset));
    }
    let entries = u32at(&opt, directory - 4) as usize;
    require(
        entries <= 16 && directory + entries * 8 <= opt.len(),
        "graphics_pe_directories",
    )?;
    let mut imports = |index, delayed| {
        if entries <= index {
            Ok(Vec::new())
        } else {
            image.imports(
                u32at(&opt, directory + index * 8),
                u32at(&opt, directory + index * 8 + 4),
                delayed,
                base,
            )
        }
    };
    Ok(Imports {
        machine: u16at(&coff, 4),
        ordinary: imports(1, false)?,
        delayed: imports(13, true)?,
    })
}

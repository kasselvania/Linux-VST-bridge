//! Optional, bounded installer classification. Version strings are declarations,
//! never publisher authentication or permission to execute an installer.
use crate::{require, Result};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Metadata {
    pub container: &'static str,
    pub strings: BTreeMap<String, String>,
}

fn u16at(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes.get(at..at + 2).ok_or("installer_metadata_range")?.try_into()?))
}
fn u32at(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes.get(at..at + 4).ok_or("installer_metadata_range")?.try_into()?))
}
fn read(source: &mut (impl Read + Seek), offset: u64, count: usize, length: u64) -> Result<Vec<u8>> {
    require(offset.checked_add(count as u64).is_some_and(|end| end <= length), "installer_metadata_range")?;
    source.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; count];
    source.read_exact(&mut bytes)?;
    Ok(bytes)
}
fn entries(resource: &[u8], offset: usize) -> Result<Vec<(u32, u32)>> {
    let count = usize::from(u16at(resource, offset + 12)?) + usize::from(u16at(resource, offset + 14)?);
    require(count <= 128, "installer_metadata_resource_count")?;
    (0..count).map(|index| {
        let at = offset + 16 + index * 8;
        Ok((u32at(resource, at)?, u32at(resource, at + 4)?))
    }).collect()
}
fn align(at: usize) -> usize { (at + 3) & !3 }
fn text(bytes: &[u8], at: &mut usize, end: usize) -> Result<String> {
    let mut units = Vec::new();
    loop {
        require(*at + 2 <= end && units.len() <= 256, "installer_metadata_text_bound")?;
        let unit = u16at(bytes, *at)?;
        *at += 2;
        if unit == 0 { break; }
        units.push(unit);
    }
    let value = String::from_utf16(&units)?;
    require(!value.chars().any(char::is_control), "installer_metadata_text_control")?;
    Ok(value)
}
#[derive(Clone, Copy)]
enum Parent { Resource, VersionInfo, StringFileInfo, StringTable, VarFileInfo }
fn blocks(bytes: &[u8], start: usize, end: usize, parent: Parent, depth: usize, count: &mut usize,
    strings: &mut BTreeMap<String, String>) -> Result<()> {
    require(depth <= 4 && end <= bytes.len(), "installer_metadata_version_depth")?;
    let mut at = start;
    while at < end {
        // Alignment padding is allowed only at the end of the enclosing block.
        if bytes[at..end].iter().all(|byte| *byte == 0) { break; }
        *count += 1;
        require(*count <= 128, "installer_metadata_version_count")?;
        let length = usize::from(u16at(bytes, at)?);
        let value_length = usize::from(u16at(bytes, at + 2)?);
        let kind = u16at(bytes, at + 4)?;
        let block_end = at.checked_add(length).ok_or("installer_metadata_range")?;
        require(length >= 6 && block_end <= end && kind <= 1, "installer_metadata_version_block")?;
        let mut value_at = at + 6;
        let key = text(bytes, &mut value_at, block_end)?;
        let child_parent = match parent {
            Parent::Resource => {
                require(key == "VS_VERSION_INFO" && kind == 0, "installer_metadata_version_root")?;
                Parent::VersionInfo
            }
            Parent::VersionInfo => {
                require(kind == 1 && value_length == 0, "installer_metadata_version_container")?;
                match key.as_str() {
                    "StringFileInfo" => Parent::StringFileInfo,
                    "VarFileInfo" => Parent::VarFileInfo,
                    _ => return Err("installer_metadata_version_container".into()),
                }
            }
            Parent::StringFileInfo => {
                require(key.len() == 8 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && kind == 1 && value_length == 0, "installer_metadata_language_codepage_table")?;
                Parent::StringTable
            }
            Parent::StringTable => {
                require(kind == 1 && value_length > 0, "installer_metadata_version_string")?;
                Parent::StringTable
            }
            Parent::VarFileInfo => {
                require(key == "Translation" && kind == 0 && value_length.is_multiple_of(4), "installer_metadata_translation")?;
                Parent::VarFileInfo
            }
        };
        value_at = align(value_at);
        let value_end = value_at.checked_add(value_length * if kind == 1 {2} else {1})
            .ok_or("installer_metadata_range")?;
        require(value_end <= block_end, "installer_metadata_version_value")?;
        if matches!(key.as_str(), "CompanyName" | "ProductName" | "ProductVersion" | "FileVersion") {
            require(matches!(parent, Parent::StringTable), "installer_metadata_version_string")?;
            let value = text(bytes, &mut value_at, value_end)?;
            require(value_at == value_end && strings.insert(key, value).is_none(), "installer_metadata_duplicate_string")?;
        }
        let child = align(value_end);
        if child < block_end {
            require(!matches!(parent, Parent::StringTable | Parent::VarFileInfo), "installer_metadata_version_leaf")?;
            blocks(bytes, child, block_end, child_parent, depth + 1, count, strings)?;
        }
        at = align(block_end);
    }
    Ok(())
}

pub(super) fn inspect(source: &mut (impl Read + Seek)) -> Result<Metadata> {
    let length = source.seek(SeekFrom::End(0))?;
    require((512..=4_294_967_296).contains(&length), "installer_metadata_file_bound")?;
    let dos = read(source, 0, 64, length)?;
    require(&dos[..2] == b"MZ", "installer_metadata_dos")?;
    let pe = u64::from(u32at(&dos, 60)?);
    let coff = read(source, pe, 24, length)?;
    let count = usize::from(u16at(&coff, 6)?);
    let flags = u16at(&coff, 22)?;
    let optional = usize::from(u16at(&coff, 20)?);
    require(&coff[..4] == b"PE\0\0" && u16at(&coff, 4)? == 0x14c
        && flags & 2 != 0 && flags & 0x2000 == 0 && (1..=96).contains(&count)
        && (120..=4096).contains(&optional), "installer_metadata_pe32")?;
    let opt = read(source, pe + 24, optional, length)?;
    require(u16at(&opt, 0)? == 0x10b && (3..=16).contains(&u32at(&opt, 92)?), "installer_metadata_pe32")?;
    let resource_rva = u64::from(u32at(&opt, 112)?);
    let resource_size = usize::try_from(u32at(&opt, 116)?)?;
    require(resource_rva != 0 && (16..=1_048_576).contains(&resource_size), "installer_metadata_resources")?;
    let table = pe + 24 + optional as u64;
    let headers = u64::from(u32at(&opt, 60)?);
    require(headers >= table + (count * 40) as u64 && headers <= length, "installer_metadata_headers")?;
    let mut sections = Vec::new();
    let mut overlay = headers;
    for index in 0..count {
        let row = read(source, table + (index * 40) as u64, 40, length)?;
        let (rva, size, offset) = (u64::from(u32at(&row, 12)?), u64::from(u32at(&row, 16)?), u64::from(u32at(&row, 20)?));
        require(offset + size <= length && rva + size <= u64::from(u32::MAX), "installer_metadata_section")?;
        overlay = overlay.max(offset + size);
        sections.push((rva, size, offset));
    }
    let mapped = |rva: u64, count: usize| -> Result<u64> {
        let end = rva.checked_add(count as u64).ok_or("installer_metadata_range")?;
        let offsets: Vec<_> = sections.iter().filter(|(start, size, _)| rva >= *start && end <= start + size)
            .map(|(start, _, offset)| offset + rva - start).collect();
        require(offsets.len() == 1, "installer_metadata_resource_mapping")?;
        Ok(offsets[0])
    };
    let nsis = read(source, overlay, 28, length)?;
    let data_size = u64::from(u32at(&nsis, 24)?);
    require(&nsis[4..20] == b"\xef\xbe\xad\xdeNullsoftInst" && u32at(&nsis, 20)? > 0
        && u64::from(u32at(&nsis, 20)?) <= data_size && data_size >= 28
        && overlay + data_size <= length, "installer_metadata_nsis_container")?;
    let resource = read(source, mapped(resource_rva, resource_size)?, resource_size, length)?;
    let versions: Vec<_> = entries(&resource, 0)?.into_iter().filter(|(kind, _)| *kind == 16).collect();
    require(versions.len() == 1 && versions[0].1 & 0x8000_0000 != 0, "installer_metadata_version_resource")?;
    let mut found = None;
    let mut leaves = 0;
    for (_, name) in entries(&resource, (versions[0].1 & 0x7fff_ffff) as usize)? {
        require(name & 0x8000_0000 != 0, "installer_metadata_version_resource")?;
        for (_, language) in entries(&resource, (name & 0x7fff_ffff) as usize)? {
            leaves += 1;
            require(leaves <= 16 && language & 0x8000_0000 == 0, "installer_metadata_version_resource")?;
            let at = language as usize;
            let rva = u64::from(u32at(&resource, at)?);
            let size = usize::try_from(u32at(&resource, at + 4)?)?;
            require((6..=65_536).contains(&size), "installer_metadata_version_bound")?;
            let bytes = read(source, mapped(rva, size)?, size, length)?;
            let mut strings = BTreeMap::new();
            blocks(&bytes, 0, bytes.len(), Parent::Resource, 0, &mut 0, &mut strings)?;
            require(strings.len() == 4 && found.as_ref().is_none_or(|previous| previous == &strings), "installer_metadata_version_ambiguous")?;
            found = Some(strings);
        }
    }
    Ok(Metadata { container: "nsis_pe32", strings: found.ok_or("installer_metadata_version_missing")? })
}

#[cfg(test)]
mod fixture_data { include!("../../tests/fixtures/nsis_installer.rs"); }
#[cfg(test)]
pub(super) use fixture_data::fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn structural_resources_and_container_are_required_not_embedded_text() {
        let bytes = fixture(None);
        let parsed = inspect(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(parsed.container, "nsis_pe32");
        assert_eq!(parsed.strings["CompanyName"], "Native Instruments");
        assert_eq!(parsed.strings["FileVersion"], "3.26.0.962");
        for offset in [132, 152, 532, 556, 580, 584, 588, 2564] {
            let mut malformed = bytes.clone();
            malformed[offset..offset+4].fill(0xff);
            assert!(inspect(&mut Cursor::new(malformed)).is_err(), "offset {offset}");
        }
        for length in [63, 511, 580, 603, 2580] {
            assert!(inspect(&mut Cursor::new(&bytes[..length])).is_err());
        }
        let mut executable = bytes.clone(); executable[150..152].copy_from_slice(&0x2002_u16.to_le_bytes());
        assert!(inspect(&mut Cursor::new(executable)).is_err());
    }
    #[test]
    fn bounded_parser_refuses_resource_cycles_and_duplicate_version_strings() {
        let mut cycle = fixture(None);
        cycle[580..584].copy_from_slice(&0x8000_0030_u32.to_le_bytes());
        assert!(inspect(&mut Cursor::new(cycle)).is_err());
        let mut strings = BTreeMap::new();
        strings.insert("FileVersion".into(), "different".into());
        let bytes = fixture(None);
        assert!(blocks(&bytes[600..], 0, u16at(&bytes, 600).unwrap() as usize, Parent::Resource, 0, &mut 0, &mut strings).is_err());
    }
    #[test]
    fn matching_strings_require_their_version_resource_parents() {
        for (from, to) in [("StringFileInfo", "XtringFileInfo"), ("040904b0", "notlang!")] {
            let mut bytes = fixture(None);
            let from: Vec<_> = from.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let to: Vec<_> = to.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let at = bytes.windows(from.len()).position(|value| value == from).unwrap();
            bytes[at..at + to.len()].copy_from_slice(&to);
            assert!(inspect(&mut Cursor::new(bytes)).is_err());
        }
    }
}

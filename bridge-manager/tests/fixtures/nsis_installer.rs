// Source-owned synthetic installer bytes; no vendor executable or resources.
pub fn fixture(replacement: Option<(&str, &str)>) -> Vec<u8> {
    fixture_with_container_types(replacement, [0; 3])
}
pub fn fixture_with_container_types(replacement: Option<(&str, &str)>, kinds: [u16; 3]) -> Vec<u8> {
    fn align(at: usize) -> usize { (at + 3) & !3 }
    fn wide(value: &str) -> Vec<u8> { value.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect() }
    fn block(key: &str, value: &[u8], kind: u16, children: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![0; 6];
        bytes.extend(wide(key));
        bytes.resize(align(bytes.len()), 0);
        bytes.extend(value);
        for child in children {
            bytes.resize(align(bytes.len()), 0);
            bytes.extend(child);
        }
        let length = bytes.len() as u16;
        bytes[..2].copy_from_slice(&length.to_le_bytes());
        bytes[2..4].copy_from_slice(&((value.len() / if kind == 1 {2} else {1}) as u16).to_le_bytes());
        bytes[4..6].copy_from_slice(&kind.to_le_bytes());
        bytes
    }
    let fields: Vec<_> = [("CompanyName", "Native Instruments"), ("ProductName", "Native Access"),
        ("ProductVersion", "3.26.0"), ("FileVersion", "3.26.0.962")].into_iter().map(|(key, value)| {
            let value = replacement.filter(|(replace, _)| *replace == key).map_or(value, |(_, value)| value);
            block(key, &wide(value), 1, &[])
        }).collect();
    let table = block("040904b0", &[], kinds[1], &fields);
    let strings = block("StringFileInfo", &[], kinds[0], &[table]);
    let translation = block("Translation", &[9, 4, 176, 4], 0, &[]);
    let vars = block("VarFileInfo", &[], kinds[2], &[translation]);
    let version = block("VS_VERSION_INFO", &[0; 52], 0, &[strings, vars]);
    let mut bytes = vec![0_u8; 512 + 2048 + 28];
    fn set16(bytes: &mut [u8], at: usize, value: u16) { bytes[at..at+2].copy_from_slice(&value.to_le_bytes()); }
    fn set32(bytes: &mut [u8], at: usize, value: u32) { bytes[at..at+4].copy_from_slice(&value.to_le_bytes()); }
    bytes[..2].copy_from_slice(b"MZ"); set32(&mut bytes, 60, 128);
    bytes[128..132].copy_from_slice(b"PE\0\0"); set16(&mut bytes, 132, 0x14c);
    set16(&mut bytes, 134, 1); set16(&mut bytes, 148, 224); set16(&mut bytes, 150, 2);
    set16(&mut bytes, 152, 0x10b); set32(&mut bytes, 212, 512); set32(&mut bytes, 244, 16);
    set32(&mut bytes, 264, 0x1000); set32(&mut bytes, 268, (88 + version.len()) as u32);
    bytes[376..381].copy_from_slice(b".rsrc"); set32(&mut bytes, 388, 0x1000);
    set32(&mut bytes, 392, 2048); set32(&mut bytes, 396, 512);
    for offset in [0, 24, 48] { set16(&mut bytes, 512 + offset + 14, 1); }
    set32(&mut bytes, 528, 16); set32(&mut bytes, 532, 0x8000_0018);
    set32(&mut bytes, 552, 1); set32(&mut bytes, 556, 0x8000_0030);
    set32(&mut bytes, 576, 0x409); set32(&mut bytes, 580, 72);
    set32(&mut bytes, 584, 0x1058); set32(&mut bytes, 588, version.len() as u32);
    bytes[600..600 + version.len()].copy_from_slice(&version);
    bytes[2564..2580].copy_from_slice(b"\xef\xbe\xad\xdeNullsoftInst");
    set32(&mut bytes, 2580, 1); set32(&mut bytes, 2584, 28);
    bytes
}

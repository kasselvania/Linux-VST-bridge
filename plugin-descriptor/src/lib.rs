//! Validated discovery data shared by preparation and the native factory.
//! Parsing, allocation and file access belong strictly before activation.
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
pub const LIMIT: usize = 8 * 1024 * 1024;
pub const FILE_NAME: &str = "plugin-descriptor.json";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub schema: u32,
    pub engine_sha256: String,
    pub class_id: String,
    pub module_sha256: String,
    pub class_name: String,
    pub vendor: String,
    pub version: String,
    pub subcategories: String,
    pub buses: Vec<Bus>,
    pub parameters: Vec<Parameter>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    pub media: u32,
    pub direction: u32,
    pub index: u32,
    pub channels: u32,
    pub r#type: u32,
    pub flags: u32,
    pub arrangement: u64,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: u32,
    pub title: String,
    pub units: String,
    pub steps: i32,
    pub flags: i32,
    pub initial: f64,
    pub available: bool,
}
fn check(ok: bool, error: &'static str) -> Result<(), &'static str> {
    if ok {
        Ok(())
    } else {
        Err(error)
    }
}
fn hex<const N: usize>(s: &str) -> Result<[u8; N], &'static str> {
    check(
        s.len() == N * 2 && s.bytes().all(|b| b.is_ascii_hexdigit()),
        "descriptor_identity",
    )?;
    let mut out = [0; N];
    for (i, value) in out.iter_mut().enumerate() {
        *value = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| "descriptor_identity")?;
    }
    Ok(out)
}
fn text(s: &str, max: usize, empty: bool) -> Result<(), &'static str> {
    check(
        (empty || !s.is_empty()) && s.len() <= max && !s.chars().any(char::is_control),
        "descriptor_text",
    )
}
fn wide(s: &str) -> Result<(), &'static str> {
    check(
        !s.contains('\0') && s.encode_utf16().count() <= 127,
        "descriptor_wide_text",
    )
}
impl Descriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        check(bytes.len() <= LIMIT, "descriptor_size")?;
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "descriptor_json")?;
        value.validate()?;
        Ok(value)
    }
    pub fn effect(&self) -> bool {
        self.subcategories.split('|').any(|s| s == "Fx")
    }
    pub fn identity(&self) -> Result<[u8; 48], &'static str> {
        let mut out = [0; 48];
        out[..16].copy_from_slice(&hex::<16>(&self.class_id)?);
        out[16..].copy_from_slice(&hex::<32>(&self.module_sha256)?);
        Ok(out)
    }
    pub fn external_ids(&self) -> [[u8; 16]; 2] {
        // Existing immutable UUIDv5 namespace; never bind DAW identity to bytes/path.
        let namespace = [
            0x93, 0x89, 0x48, 0x0f, 0xb4, 0xb0, 0x5e, 0x02, 0xa1, 0xd7, 0x68, 0x7a, 0x57, 0xb5,
            0x4b, 0x3f,
        ];
        [":processor", ":controller"].map(|suffix| {
            let mut sha = Sha1::new();
            sha.update(namespace);
            sha.update(self.class_id.to_ascii_uppercase());
            sha.update(suffix);
            let mut id = [0; 16];
            id.copy_from_slice(&sha.finalize()[..16]);
            id[6] = (id[6] & 15) | 0x50;
            id[8] = (id[8] & 63) | 0x80;
            id
        })
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        check(self.schema == 1, "descriptor_schema")?;
        self.identity()?;
        hex::<32>(&self.engine_sha256)?;
        text(&self.class_name, 63, false)?;
        text(&self.vendor, 63, false)?;
        text(&self.version, 63, false)?;
        text(&self.subcategories, 127, false)?;
        check(
            self.effect() != self.subcategories.split('|').any(|s| s == "Instrument"),
            "descriptor_role",
        )?;
        check(
            self.buses.len() <= 56 && self.parameters.len() <= 8192,
            "descriptor_capacity",
        )?;
        let mut counts = [[0; 2]; 2];
        let mut mains = [[0; 2]; 2];
        for b in &self.buses {
            check(
                b.media <= 1 && b.direction <= 1 && b.r#type <= 1,
                "descriptor_bus_kind",
            )?;
            wide(&b.name)?;
            let (m, d) = (b.media as usize, b.direction as usize);
            check(b.index == counts[m][d], "descriptor_bus_index")?;
            counts[m][d] += 1;
            let bound = if m == 0 && d == 1 {
                32
            } else if m == 1 && d == 0 {
                1
            } else {
                8
            };
            check(counts[m][d] <= bound, "descriptor_bus_count")?;
            if b.r#type == 0 {
                mains[m][d] += 1;
            }
            if m == 0 {
                check(
                    b.channels == 2 && b.arrangement == 3,
                    "descriptor_stereo_required",
                )?;
                check(
                    d != 0 || b.r#type != 0 || b.index == 0,
                    "descriptor_main_input",
                )?;
                check(
                    d != 1 || b.index != 0 || b.r#type == 0,
                    "descriptor_main_output",
                )?;
            } else {
                check(
                    b.channels <= 16 && b.arrangement == 0,
                    "descriptor_event_channels",
                )?;
            }
        }
        check(
            counts[0][1] > 0 && mains[0][0] <= 1 && (!self.effect() || mains[0][0] == 1),
            "descriptor_audio_layout",
        )?;
        let mut ids = BTreeSet::new();
        for p in &self.parameters {
            check(
                ids.insert(p.id) && p.id != u32::MAX,
                "descriptor_parameter_identity",
            )?;
            wide(&p.title)?;
            wide(&p.units)?;
            check(
                p.steps >= 0 && p.initial.is_finite() && (0.0..=1.0).contains(&p.initial),
                "descriptor_parameter_value",
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Descriptor {
        Descriptor {
            schema: 1,
            engine_sha256: "aa".repeat(32),
            class_id: "01".repeat(16),
            module_sha256: "bb".repeat(32),
            class_name: "Unseen instrument".into(),
            vendor: "Reference".into(),
            version: "1".into(),
            subcategories: "Instrument|Synth".into(),
            buses: vec![Bus {
                media: 0,
                direction: 1,
                index: 0,
                channels: 2,
                r#type: 0,
                flags: 1,
                arrangement: 3,
                name: "Output".into(),
            }],
            parameters: vec![Parameter {
                id: 7,
                title: "Level".into(),
                units: "".into(),
                steps: 0,
                flags: 1,
                initial: 0.25,
                available: true,
            }],
        }
    }
    #[test]
    fn update_preserves_logical_ids_and_actual_parameter_readback() {
        let first = fixture();
        let mut updated = first.clone();
        updated.module_sha256 = "cc".repeat(32);
        updated.parameters[0].initial = 0.75;
        let parsed = Descriptor::parse(&serde_json::to_vec(&updated).unwrap()).unwrap();
        assert_eq!(parsed.parameters[0].initial, 0.75);
        assert_eq!(first.external_ids(), parsed.external_ids());
        assert_ne!(first.identity(), parsed.identity());
        assert_eq!(
            first.external_ids()[0],
            [48, 54, 83, 46, 112, 13, 91, 151, 186, 52, 15, 32, 252, 93, 63, 20]
        );
    }
    #[test]
    fn malformed_capacity_identity_and_unsupported_layouts_are_refused() {
        let base = fixture();
        for change in 0..7 {
            let mut d = base.clone();
            match change {
                0 => d.parameters.push(d.parameters[0].clone()),
                1 => d.parameters[0].initial = 1.5,
                2 => d.buses[0].index = 1,
                3 => d.buses[0].channels = 4,
                4 => d.class_id = "x".repeat(32),
                5 => d.parameters[0].title = "x".repeat(128),
                _ => d.subcategories = "Fx|Instrument".into(),
            }
            assert!(d.validate().is_err(), "case {change}");
        }
        let mut value = serde_json::to_value(base).unwrap();
        value["unexpected"] = true.into();
        assert_eq!(
            Descriptor::parse(&serde_json::to_vec(&value).unwrap()),
            Err("descriptor_json")
        );
        assert_eq!(
            Descriptor::parse(&vec![0; LIMIT + 1]),
            Err("descriptor_size")
        );
    }
}

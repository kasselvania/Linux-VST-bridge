//! Exact, pre-activation plug-in identity and the buses this stereo host supports.
use crate::config::{hex32, PinnedFile};
use ap2_backend::rpi0::Message;
use serde_json::Value;
use std::{collections::BTreeSet, fs, io};

#[derive(Clone, Debug)]
pub struct Control {
    pub id: u32,
    pub title: String,
    pub units: String,
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub class: [u8; 16],
    pub buses: Vec<u8>,
    pub stereo_input: bool,
    pub midi_input: bool,
    pub zero_event_channels_unspecified: bool,
    pub legacy_master: bool,
    pub controls: Vec<Control>,
    pub surface: Option<crate::panel::Surface>,
}

impl Binding {
    pub fn pigments() -> Self {
        Self {
            class: crate::contract::PIGMENTS_CLASS,
            buses: crate::contract::pigments_bus_contract().to_vec(),
            stereo_input: false,
            midi_input: true,
            zero_event_channels_unspecified: true,
            legacy_master: true,
            controls: Vec::new(),
            surface: None,
        }
    }

    pub fn load(file: &PinnedFile, module: &[u8; 32]) -> io::Result<Self> {
        if fs::metadata(&file.path)?.len() > 65536 {
            return Err(invalid("plugin binding size"));
        }
        file.verify("plugin binding")?;
        let bytes = fs::read(&file.path)?;
        use sha2::{Digest, Sha256};
        if Sha256::digest(&bytes).as_slice() != file.sha256 {
            return Err(invalid("plugin binding changed during read"));
        }
        Self::parse(&bytes, module)
    }

    fn parse(bytes: &[u8], module: &[u8; 32]) -> io::Result<Self> {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| invalid("plugin binding JSON"))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid("plugin binding object"))?;
        let keys = [
            "schema",
            "module_sha256",
            "class_id",
            "input",
            "buses",
            "controls",
            "zero_event_channels_unspecified",
        ];
        if object.len() != keys.len() + usize::from(object.contains_key("surface"))
            || keys.iter().any(|k| !object.contains_key(*k))
            || value["schema"] != "lvb-arm-plugin-binding/v1"
            || hex32(
                value["module_sha256"]
                    .as_str()
                    .ok_or_else(|| invalid("binding module"))?,
            )? != *module
        {
            return Err(invalid("plugin binding schema/module identity"));
        }
        let class_text = value["class_id"]
            .as_str()
            .ok_or_else(|| invalid("binding class"))?;
        if class_text.len() != 32 || !class_text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("binding class syntax"));
        }
        let mut class = [0; 16];
        for (i, byte) in class.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&class_text[i * 2..i * 2 + 2], 16)
                .map_err(|_| invalid("binding class"))?;
        }
        if class == [0; 16] {
            return Err(invalid("empty binding class"));
        }
        let stereo_input = match value["input"].as_str() {
            Some("stereo") => true,
            Some("silence") => false,
            _ => return Err(invalid("binding input mode")),
        };
        let zero_event_channels_unspecified = value["zero_event_channels_unspecified"]
            .as_bool()
            .ok_or_else(|| invalid("binding event policy"))?;
        let rows = value["buses"]
            .as_array()
            .ok_or_else(|| invalid("binding buses"))?;
        if rows.is_empty() || rows.len() > 5 {
            return Err(invalid("unsupported bus count"));
        }
        let mut buses = (rows.len() as u32).to_le_bytes().to_vec();
        let mut identities = BTreeSet::new();
        let mut main_input = false;
        let mut audio_output = false;
        let mut midi_input = false;
        for row in rows {
            let row = row
                .as_array()
                .filter(|r| r.len() == 7)
                .ok_or_else(|| invalid("binding bus row"))?;
            let mut fields = [0u64; 7];
            for (target, source) in fields.iter_mut().zip(row) {
                *target = source
                    .as_u64()
                    .ok_or_else(|| invalid("binding bus integer"))?;
            }
            let [media, direction, index, channels, kind, active, arrangement] = fields;
            if media > 1
                || direction > 1
                || index > 1
                || kind > 1
                || active > 1
                || !identities.insert((media, direction, index))
            {
                return Err(invalid("unsupported/duplicate binding bus"));
            }
            // The extra sidechain is represented but inactive: the transport
            // carries only the selected stereo bus, never an invented second input.
            let inactive_sidechain =
                media == 0 && direction == 0 && index == 1 && kind == 1 && active == 0;
            if !inactive_sidechain && (index != 0 || active != 1) {
                return Err(invalid("unsupported inactive/additional bus"));
            }
            if media == 0 {
                if channels != 2 || arrangement != 3 || (direction == 1 && kind != 0) {
                    return Err(invalid("host requires stereo audio buses"));
                }
                main_input |= direction == 0 && kind == 0;
                audio_output |= direction == 1;
            } else {
                if !(1..=16).contains(&channels) || arrangement != 0 || kind != 0 {
                    return Err(invalid("unsupported event bus"));
                }
                midi_input |= direction == 0;
            }
            for field in &fields[..6] {
                buses.extend_from_slice(&(*field as u32).to_le_bytes());
            }
            buses.extend_from_slice(&arrangement.to_le_bytes());
        }
        if !audio_output || (stereo_input && !main_input) {
            return Err(invalid("binding lacks required audio bus"));
        }
        let rows = value["controls"]
            .as_array()
            .filter(|v| v.len() <= 32)
            .ok_or_else(|| invalid("binding controls"))?;
        let mut controls = Vec::new();
        let mut ids = BTreeSet::new();
        for row in rows {
            let object = row.as_object().ok_or_else(|| invalid("binding control"))?;
            if object.len() != 3
                || !["id", "title", "units"]
                    .iter()
                    .all(|k| object.contains_key(*k))
            {
                return Err(invalid("binding control fields"));
            }
            let id = row["id"]
                .as_u64()
                .filter(|v| *v < u32::MAX as u64)
                .ok_or_else(|| invalid("control id"))? as u32;
            let title = row["title"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| invalid("control title"))?;
            let units = row["units"]
                .as_str()
                .ok_or_else(|| invalid("control units"))?;
            if !ids.insert(id)
                || [title, units]
                    .iter()
                    .any(|s| s.encode_utf16().count() > 127 || s.chars().any(char::is_control))
            {
                return Err(invalid("duplicate/invalid control"));
            }
            controls.push(Control {
                id,
                title: title.into(),
                units: units.into(),
            });
        }
        let surface = value
            .get("surface")
            .map(|v| crate::panel::Surface::parse(v, &controls))
            .transpose()?;
        Ok(Self {
            class,
            buses,
            stereo_input,
            midi_input,
            zero_event_channels_unspecified,
            legacy_master: false,
            controls,
            surface,
        })
    }

    pub fn control(&self, id: u32) -> io::Result<&Control> {
        self.controls
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("parameter absent from selected binding"))
    }

    pub fn readback(&self, message: &Message) -> io::Result<Readback> {
        let control = self.control(message.id)?;
        let text = |s: &[u16]| {
            String::from_utf16(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]).ok()
        };
        if message.kind != 110 || message.result > 1 {
            return Err(invalid("parameter readback identity/value"));
        }
        // A VST3 parameter ID is the control identity. The plug-in may rename
        // its title or units when a preset changes, so the initial census
        // labels in the binding cannot be used as a runtime identity check.
        let metadata_changed = text(&message.title).as_deref() != Some(&control.title)
            || text(&message.units).as_deref() != Some(&control.units);
        let value = if message.result == 0 {
            if !message.value.is_finite() || !(0.0..=1.0).contains(&message.value) {
                return Err(invalid("parameter readback identity/value"));
            }
            Some(message.value)
        } else {
            // The protocol defines an unavailable value as canonical +0 bits.
            // It is missing information, not a request to set the control to 0.
            if message.value.to_bits() != 0 {
                return Err(invalid("parameter readback identity/value"));
            }
            None
        };
        Ok(Readback {
            value,
            metadata_changed,
        })
    }
}

pub struct Readback {
    pub value: Option<f64>,
    pub metadata_changed: bool,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::hex;
    #[test]
    fn digitalis_census_binding_matches_stereo_effect_and_selected_controls() {
        let bytes = include_bytes!("../../../rpi2/digitalis-binding.json");
        let module =
            hex32("fb51eec9bda65f5c8ac3f4c7f9a99f6fc0034df3cd54fc1b54e32e598e2af335").unwrap();
        let binding = Binding::parse(bytes, &module).unwrap();
        assert!(binding.stereo_input);
        assert!(!binding.midi_input);
        assert_eq!(binding.controls.len(), 4);
        assert_eq!(
            binding.class,
            [
                0xAB, 0xCD, 0xEF, 0x01, 0x91, 0x82, 0xFA, 0xEB, 0x41, 0x62, 0x72, 0x6E, 0x44, 0x69,
                0x67, 0x69
            ]
        );
        assert!(binding.surface.is_some());
    }
    #[test]
    fn serum2_census_binding_selects_instrument_and_declared_event_buses() {
        let bytes = include_bytes!("../../../rpi2/serum2-binding.json");
        let module =
            hex32("501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283").unwrap();
        let binding = Binding::parse(bytes, &module).unwrap();
        assert_eq!(hex(&binding.class), "56534558667350736572756d20320000");
        assert!(!binding.stereo_input);
        assert!(binding.midi_input);
        assert!(!binding.zero_event_channels_unspecified);
        assert_eq!(binding.buses.len(), 4 + 3 * 32);
        let fields: Vec<u32> = binding
            .buses
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        assert_eq!(
            fields,
            [3, 0, 1, 0, 2, 0, 1, 3, 0, 1, 0, 0, 16, 0, 1, 0, 0, 1, 1, 0, 16, 0, 1, 0, 0]
        );
        assert_eq!(
            binding.controls.iter().map(|c| c.id).collect::<Vec<_>>(),
            [0, 2_000_000, 2_000_003, 7_000_000]
        );
        assert!(binding.surface.is_some());
    }
    fn manifest() -> Value {
        serde_json::json!({"schema":"lvb-arm-plugin-binding/v1", "module_sha256":"11".repeat(32),
            "class_id":"22".repeat(16), "input":"stereo", "zero_event_channels_unspecified":false,
            "buses":[[0,0,0,2,0,1,3],[0,1,0,2,0,1,3]],
            "controls":[{"id":7,"title":"Mix","units":"%"}]})
    }
    #[test]
    fn effect_binding_preserves_identity_and_does_not_inherit_pigments_policy() {
        let binding =
            Binding::parse(&serde_json::to_vec(&manifest()).unwrap(), &[0x11; 32]).unwrap();
        assert_eq!(binding.class, [0x22; 16]);
        assert!(binding.stereo_input);
        assert!(
            !binding.midi_input
                && !binding.zero_event_channels_unspecified
                && !binding.legacy_master
        );
        assert_eq!(binding.buses.len(), 68);
        assert!(Binding::parse(&serde_json::to_vec(&manifest()).unwrap(), &[0x33; 32]).is_err());
    }
    #[test]
    fn sidechain_census_is_preserved_but_cannot_be_activated() {
        let mut value = manifest();
        value["buses"]
            .as_array_mut()
            .unwrap()
            .insert(1, serde_json::json!([0, 0, 1, 2, 1, 0, 3]));
        let binding = Binding::parse(&serde_json::to_vec(&value).unwrap(), &[0x11; 32]).unwrap();
        assert_eq!(binding.buses.len(), 100);
        assert_eq!(&binding.buses[56..60], &0u32.to_le_bytes());
        value["buses"][1][5] = 1.into();
        assert!(Binding::parse(&serde_json::to_vec(&value).unwrap(), &[0x11; 32]).is_err());
    }
    #[test]
    fn control_readback_uses_id_when_preset_changes_metadata() {
        let binding =
            Binding::parse(&serde_json::to_vec(&manifest()).unwrap(), &[0x11; 32]).unwrap();
        let mut message = Message {
            kind: 110,
            id: 7,
            value: 0.5,
            ..Message::default()
        };
        for (slot, c) in message.title.iter_mut().zip("Mix".encode_utf16()) {
            *slot = c;
        }
        message.units[0] = b'%' as u16;
        assert_eq!(binding.readback(&message).unwrap().value, Some(0.5));
        message.title[0] = b'X' as u16;
        let renamed = binding.readback(&message).unwrap();
        assert_eq!(renamed.value, Some(0.5));
        assert!(renamed.metadata_changed);
        message.result = 1;
        message.value = 0.0;
        assert_eq!(binding.readback(&message).unwrap().value, None);
        message.value = 0.5;
        assert!(binding.readback(&message).is_err());
        message.result = 0;
        message.value = f64::NAN;
        assert!(binding.readback(&message).is_err());
        message.value = 0.5;
        message.id = 8;
        assert!(binding.readback(&message).is_err());
        assert!(binding.control(0).is_err());
    }
    #[test]
    fn unsupported_channels_missing_input_and_duplicate_buses_are_refused() {
        for change in 0..3 {
            let mut value = manifest();
            match change {
                0 => value["buses"][0][3] = 1.into(),
                1 => {
                    value["buses"].as_array_mut().unwrap().remove(0);
                }
                _ => value["buses"][1] = value["buses"][0].clone(),
            }
            assert!(Binding::parse(&serde_json::to_vec(&value).unwrap(), &[0x11; 32]).is_err());
        }
    }
}

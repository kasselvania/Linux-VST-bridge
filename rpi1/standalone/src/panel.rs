//! Reusable physical-to-parameter mapping. All work is outside the audio callback.
use crate::binding::Control;
use serde_json::Value;
use std::{
    collections::{BTreeSet, VecDeque},
    io,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Encoder {
    pub index: usize,
    pub parameter: u32,
    pub step: f64,
}
#[derive(Clone, Debug)]
pub struct Button {
    pub index: usize,
    pub parameter: u32,
    pub momentary: bool,
}
#[derive(Clone, Debug)]
pub struct Surface {
    pub label: String,
    pub encoders: Vec<Encoder>,
    pub buttons: Vec<Button>,
    pub save_button: usize,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn fields(value: &Value, keys: &[&str]) -> io::Result<()> {
    let object = value.as_object().ok_or_else(|| invalid("surface object"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid("surface fields"));
    }
    Ok(())
}
fn index(value: &Value) -> io::Result<usize> {
    value
        .as_u64()
        .filter(|v| (1..=3).contains(v))
        .map(|v| v as usize - 1)
        .ok_or_else(|| invalid("physical control must be 1..3"))
}
impl Surface {
    pub fn parse(value: &Value, controls: &[Control]) -> io::Result<Self> {
        fields(value, &["label", "encoders", "buttons", "save_button"])?;
        let label = value["label"]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 20
                    && s.bytes().all(|c| c.is_ascii_graphic() || c == b' ')
            })
            .ok_or_else(|| invalid("surface label"))?
            .to_owned();
        let parameter = |row: &Value| -> io::Result<u32> {
            let id = row["parameter"]
                .as_u64()
                .filter(|v| *v < u32::MAX as u64)
                .ok_or_else(|| invalid("surface parameter"))? as u32;
            if !controls.iter().any(|p| p.id == id) {
                return Err(invalid("surface parameter lacks selected metadata"));
            }
            Ok(id)
        };
        let mut encoders = Vec::new();
        let mut buttons = Vec::new();
        let mut encoder_ids = BTreeSet::new();
        let mut button_ids = BTreeSet::new();
        let rows = value["encoders"]
            .as_array()
            .filter(|v| v.len() <= 3)
            .ok_or_else(|| invalid("surface encoders"))?;
        for row in rows {
            fields(row, &["index", "parameter", "step"])?;
            let index = index(&row["index"])?;
            let step = row["step"]
                .as_f64()
                .filter(|s| s.is_finite() && *s > 0.0 && *s <= 1.0)
                .ok_or_else(|| invalid("encoder step"))?;
            if !encoder_ids.insert(index) {
                return Err(invalid("duplicate encoder"));
            }
            encoders.push(Encoder {
                index,
                parameter: parameter(row)?,
                step,
            });
        }
        let rows = value["buttons"]
            .as_array()
            .filter(|v| v.len() <= 2)
            .ok_or_else(|| invalid("surface buttons"))?;
        for row in rows {
            if row.get("mode").is_some() {
                fields(row, &["index", "parameter", "mode"])?;
            } else {
                fields(row, &["index", "parameter"])?;
            }
            let momentary = match row.get("mode").and_then(Value::as_str) {
                None if row.get("mode").is_none() => false,
                Some("toggle") => false,
                Some("momentary") => true,
                _ => return Err(invalid("button mode")),
            };
            let index = index(&row["index"])?;
            if !button_ids.insert(index) {
                return Err(invalid("duplicate button"));
            }
            buttons.push(Button {
                index,
                parameter: parameter(row)?,
                momentary,
            });
        }
        let save_button = index(&value["save_button"])?;
        if !button_ids.insert(save_button) {
            return Err(invalid("save button also mapped to parameter"));
        }
        Ok(Self {
            label,
            encoders,
            buttons,
            save_button,
        })
    }
}
#[derive(Debug, PartialEq)]
pub enum Action {
    Set(u32, f64),
    Save,
}
struct Parameter {
    id: u32,
    title: String,
    toggle: bool,
    momentary: bool,
    confirmed: Option<f64>,
    desired: Option<f64>,
    in_flight: Option<(f64, Instant)>,
}
pub struct Model {
    pub surface: Surface,
    parameters: Vec<Parameter>,
    pressed: [bool; 3],
    save_requested: bool,
    edges: VecDeque<(u32, f64)>,
    last_edge: Option<Instant>,
    pub status: String,
    pub faulted: bool,
}
impl Model {
    pub fn new(surface: Surface, controls: &[Control]) -> Self {
        let ids = surface
            .encoders
            .iter()
            .map(|p| p.parameter)
            .chain(surface.buttons.iter().map(|p| p.parameter))
            .collect::<BTreeSet<_>>();
        let parameters = controls
            .iter()
            .filter(|p| ids.contains(&p.id))
            .map(|p| Parameter {
                id: p.id,
                title: p.title.clone(),
                toggle: surface.buttons.iter().any(|b| b.parameter == p.id),
                momentary: surface
                    .buttons
                    .iter()
                    .any(|b| b.parameter == p.id && b.momentary),
                confirmed: None,
                desired: None,
                in_flight: None,
            })
            .collect();
        Self {
            surface,
            parameters,
            pressed: [false; 3],
            save_requested: false,
            edges: VecDeque::with_capacity(32),
            last_edge: None,
            status: "SYNCING".into(),
            faulted: false,
        }
    }
    pub fn parameter_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.parameters.iter().map(|p| p.id)
    }
    pub fn readback(&mut self, id: u32, value: f64) {
        if let Some(p) = self.parameters.iter_mut().find(|p| p.id == id) {
            let previous = p.confirmed;
            p.confirmed = Some(value);
            if let Some((sent, _)) = p.in_flight {
                if (sent - value).abs() < 1e-7 {
                    p.in_flight = None;
                }
            } else if p.desired == previous || p.desired.is_none() {
                p.desired = Some(value);
            }
        }
        if self.status == "SYNCING" && self.parameters.iter().all(|p| p.confirmed.is_some()) {
            self.status = "READY".into();
        }
    }
    pub fn encoder(&mut self, index: usize, delta: i32) {
        if self.faulted {
            return;
        }
        if let Some(mapping) = self.surface.encoders.iter().find(|p| p.index == index) {
            if let Some(p) = self
                .parameters
                .iter_mut()
                .find(|p| p.id == mapping.parameter)
            {
                if let Some(value) = p.desired {
                    p.desired = Some((value + mapping.step * f64::from(delta)).clamp(0.0, 1.0));
                    self.status = "EDITED".into();
                }
            }
        }
    }
    pub fn button(&mut self, index: usize, pressed: bool) {
        if self.faulted || index >= 3 {
            return;
        }
        let changed = pressed != self.pressed[index];
        let edge = pressed && changed;
        self.pressed[index] = pressed;
        if changed {
            if let Some(mapping) = self
                .surface
                .buttons
                .iter()
                .find(|b| b.index == index && b.momentary)
            {
                if self.edges.len() == 32 {
                    self.faulted = true;
                    self.status = "INPUT OVERFLOW".into();
                } else {
                    self.edges
                        .push_back((mapping.parameter, if pressed { 1.0 } else { 0.0 }));
                }
                return;
            }
        }
        if !edge {
            return;
        }
        if index == self.surface.save_button {
            self.save_requested = true;
            return;
        }
        if let Some(mapping) = self.surface.buttons.iter().find(|p| p.index == index) {
            if let Some(p) = self
                .parameters
                .iter_mut()
                .find(|p| p.id == mapping.parameter)
            {
                if let Some(value) = p.desired {
                    p.desired = Some(if value >= 0.5 { 0.0 } else { 1.0 });
                    self.status = "EDITED".into();
                }
            }
        }
    }
    pub fn actions(&mut self, now: Instant) -> Vec<Action> {
        if self.faulted {
            return Vec::new();
        }
        if self.parameters.iter().any(|p| {
            p.in_flight
                .is_some_and(|(_, at)| now.duration_since(at) > Duration::from_secs(2))
        }) {
            self.faulted = true;
            self.status = "CONTROL TIMEOUT".into();
            return Vec::new();
        }
        let mut actions = Vec::new();
        // Preserve even a complete short click read in one evdev batch. Wait
        // for controller readback and pace edges rather than coalescing them.
        // This model runs on the control thread, never the audio callback.
        if self
            .last_edge
            .is_none_or(|at| now.duration_since(at) >= Duration::from_millis(50))
        {
            if let Some(&(id, value)) = self.edges.front() {
                if let Some(p) = self.parameters.iter_mut().find(|p| p.id == id) {
                    if p.confirmed.is_some() && p.in_flight.is_none() {
                        self.edges.pop_front();
                        p.in_flight = Some((value, now));
                        actions.push(Action::Set(id, value));
                        self.last_edge = Some(now);
                        self.status = "CONTROL SENT".into();
                    }
                }
            }
        }
        for p in &mut self.parameters {
            if p.momentary {
                continue;
            }
            if let (Some(actual), Some(desired)) = (p.confirmed, p.desired) {
                if p.in_flight.is_none() && (desired - actual).abs() > 1e-7 {
                    p.in_flight = Some((desired, now));
                    actions.push(Action::Set(p.id, desired));
                }
            }
        }
        if self.save_requested
            && self.edges.is_empty()
            && actions.is_empty()
            && self
                .parameters
                .iter()
                .all(|p| p.confirmed.is_some() && p.in_flight.is_none())
        {
            self.save_requested = false;
            self.status = "SAVING".into();
            actions.push(Action::Save);
        }
        actions
    }
    pub fn lines(&self, temperature: Option<f64>) -> Vec<String> {
        let mut lines = vec![self.surface.label.clone()];
        for p in self.parameters.iter().take(3) {
            let value = match p.confirmed {
                None => "?".to_owned(),
                Some(v) if p.toggle => {
                    if v >= 0.5 {
                        "ON".into()
                    } else {
                        "OFF".into()
                    }
                }
                Some(v) => format!("{:.0}%", v * 100.0),
            };
            lines.push(format!(
                "{} {}{}",
                p.title,
                value,
                if p.in_flight.is_some() { " *" } else { "" }
            ));
        }
        lines.push(match temperature {
            Some(v) => format!("TEMP {v:.1}C"),
            None => "TEMP ?".into(),
        });
        lines.push(self.status.clone());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Model {
        let controls = vec![
            Control {
                id: 17,
                title: "Mix".into(),
                units: "%".into(),
            },
            Control {
                id: 42,
                title: "Hold".into(),
                units: "".into(),
            },
        ];
        let surface = Surface::parse(&serde_json::json!({"label":"TEST", "encoders":[{"index":1,"parameter":17,"step":0.01}],"buttons":[{"index":1,"parameter":42}],"save_button":2}), &controls).unwrap();
        Model::new(surface, &controls)
    }
    #[test]
    fn confirmed_display_and_accumulated_rotations_do_not_fabricate_readback() {
        let mut m = model();
        let now = Instant::now();
        m.encoder(0, 1);
        assert!(m.actions(now).is_empty());
        m.readback(17, 0.5);
        m.readback(42, 0.0);
        m.encoder(0, 4);
        assert_eq!(m.actions(now), vec![Action::Set(17, 0.54)]);
        assert!(m.lines(None)[1].contains("50% *"));
        m.encoder(0, 3);
        m.readback(17, 0.54);
        m.readback(17, 0.54);
        assert_eq!(m.actions(now), vec![Action::Set(17, 0.5700000000000001)]);
        m.readback(17, 0.57);
        assert!(m.actions(now).is_empty());
    }
    #[test]
    fn button_edges_toggle_once_and_save_waits_for_confirmation() {
        let mut m = model();
        let now = Instant::now();
        m.readback(17, 0.5);
        m.readback(42, 0.0);
        m.button(0, true);
        m.button(0, true);
        m.button(0, false);
        m.button(1, true);
        assert_eq!(m.actions(now), vec![Action::Set(42, 1.0)]);
        m.readback(42, 1.0);
        assert_eq!(m.actions(now), vec![Action::Save]);
        assert!(m.actions(now).is_empty());
    }
    #[test]
    fn missing_confirmation_disables_controls_without_claiming_new_value() {
        let mut m = model();
        let now = Instant::now();
        m.readback(17, 0.5);
        m.readback(42, 0.0);
        m.encoder(0, -100);
        assert_eq!(m.actions(now), vec![Action::Set(17, 0.0)]);
        assert!(m.actions(now + Duration::from_secs(3)).is_empty());
        assert!(m.faulted);
        assert!(m.lines(None)[1].contains("50%"));
    }
    #[test]
    fn mapping_requires_metadata_and_distinct_save_button() {
        let mut v = serde_json::json!({"label":"T", "encoders":[{"index":1,"parameter":99,"step":0.01}],"buttons":[],"save_button":2});
        assert!(Surface::parse(&v, &[]).is_err());
        let c = vec![Control {
            id: 99,
            title: "Other plugin".into(),
            units: "".into(),
        }];
        assert!(Surface::parse(&v, &c).is_ok());
        v["buttons"] = serde_json::json!([{"index":2,"parameter":99}]);
        assert!(Surface::parse(&v, &c).is_err());
    }

    #[test]
    fn momentary_click_preserves_both_edges_and_waits_for_readback() {
        let mut m = model();
        m.surface.buttons[0].momentary = true;
        m.parameters
            .iter_mut()
            .find(|p| p.id == 42)
            .unwrap()
            .momentary = true;
        m.readback(17, 0.5);
        m.readback(42, 0.0);
        let now = Instant::now();
        m.button(0, true);
        m.button(0, true); // repeated key-down is not another command
        m.button(0, false); // both edges may arrive in one hardware read
        m.button(1, true);
        assert_eq!(m.actions(now), vec![Action::Set(42, 1.0)]);
        assert!(m.actions(now + Duration::from_millis(60)).is_empty());
        m.readback(42, 1.0);
        assert_eq!(
            m.actions(now + Duration::from_millis(61)),
            vec![Action::Set(42, 0.0)]
        );
        assert!(m.actions(now + Duration::from_millis(120)).is_empty());
        m.readback(42, 0.0);
        assert_eq!(
            m.actions(now + Duration::from_millis(121)),
            vec![Action::Save]
        );
    }

    #[test]
    fn button_mode_is_explicit_and_invalid_modes_are_refused() {
        let c = vec![Control {
            id: 42,
            title: "CC helper".into(),
            units: "".into(),
        }];
        let mut v = serde_json::json!({"label":"T", "encoders":[], "buttons":[{"index":2,"parameter":42,"mode":"momentary"}],"save_button":1});
        assert!(Surface::parse(&v, &c).unwrap().buttons[0].momentary);
        v["buttons"][0]["mode"] = serde_json::json!("unknown");
        assert!(Surface::parse(&v, &c).is_err());
    }
}

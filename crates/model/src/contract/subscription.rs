//! Bounded replacement observations. This stream never grants or replays actions.
use super::*;
use serde::{Deserialize, Serialize};

pub const GREETING: &[u8; 5] = b"LVS1\n";
pub const FRAME_LIMIT: usize = 256 * 1024;
pub const HEARTBEAT: Duration = Duration::from_secs(1);
pub const STALE_AFTER: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub service_epoch: String,
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub cursor: Cursor,
    /// None means the file watch cannot cover current records. It never means zero changes.
    pub records_revision: Option<u64>,
    pub activity: ui::Observation<ui::Activity>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    State(State),
    Heartbeat { cursor: Cursor },
    FrontendSuperseded,
    Unavailable { reason: String },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Subscribe { operator_schema: u32 }

pub fn accept(peer: &mut impl io::Read) -> crate::Result<()> {
    let request: Subscribe = read_frame(peer, REQUEST_LIMIT)?;
    crate::require(request.operator_schema == ui::OPERATOR_SCHEMA, &paired_model_required())
}
pub fn write(peer: &mut impl io::Write, event: &Event) -> io::Result<()> {
    write_frame(peer, event, FRAME_LIMIT)
}
fn write_frame(peer: &mut impl io::Write, value: &impl Serialize, limit: usize) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > limit { return Err(io::Error::new(io::ErrorKind::InvalidData, "observation_frame_bound")); }
    peer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    peer.write_all(&bytes)
}
fn read_frame<T: serde::de::DeserializeOwned>(peer: &mut impl io::Read, limit: usize) -> io::Result<T> {
    let mut size = [0; 4];
    peer.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    if size == 0 || size > limit { return Err(io::Error::new(io::ErrorKind::InvalidData, "observation_frame_bound")); }
    let mut bytes = vec![0; size];
    peer.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub struct Subscription { peer: UnixStream, cursor: Option<Cursor> }
impl Subscription {
    pub fn installed(home: &Path) -> io::Result<Self> {
        Self::connect(&home.join(".local/share/linux-vst-bridge/managed/runtime/owner.sock"))
    }
    pub fn connect(socket: &Path) -> io::Result<Self> {
        let mut peer = super::connect(socket, CONNECT_DEADLINE)?;
        peer.set_write_timeout(Some(HEARTBEAT))?;
        peer.set_read_timeout(Some(STALE_AFTER))?;
        peer.write_all(GREETING)?;
        write_frame(&mut peer, &Subscribe { operator_schema: ui::OPERATOR_SCHEMA }, REQUEST_LIMIT)?;
        Ok(Self { peer, cursor: None })
    }
    /// Replacement states may skip revisions. A heartbeat cannot establish a
    /// first state, change service identity, or renew a newer unseen revision.
    pub fn next(&mut self) -> io::Result<Event> {
        let event: Event = read_frame(&mut self.peer, FRAME_LIMIT)?;
        self.validate(&event)?;
        Ok(event)
    }
    fn validate(&mut self, event: &Event) -> io::Result<()> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "observation_cursor_invalid");
        match event {
            Event::State(state) => {
                let cursor = &state.cursor;
                if cursor.service_epoch.len() != 32 || !cursor.service_epoch.bytes().all(|b| b.is_ascii_hexdigit())
                    || cursor.revision == 0
                    || self.cursor.as_ref().is_some_and(|old| old.service_epoch != cursor.service_epoch || cursor.revision <= old.revision)
                    || matches!(&state.activity, ui::Observation::Observed(activity) if activity.schema != ui::OPERATOR_SCHEMA)
                { return Err(invalid()); }
                self.cursor = Some(cursor.clone());
            }
            Event::Heartbeat { cursor } if self.cursor.as_ref() != Some(cursor) => return Err(invalid()),
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(revision: u64) -> Event {
        Event::State(State { cursor: Cursor { service_epoch: "ab".repeat(16), revision },
            records_revision: Some(4), activity: ui::Observation::Unavailable("fixture".into()) })
    }
    #[test]
    fn complete_replacements_allow_gaps_but_not_replay_or_foreign_heartbeats() {
        let (peer, _) = UnixStream::pair().unwrap();
        let mut subscription = Subscription { peer, cursor: None };
        assert!(subscription.validate(&Event::Heartbeat { cursor: Cursor { service_epoch: "ab".repeat(16), revision: 1 } }).is_err());
        subscription.validate(&state(1)).unwrap();
        subscription.validate(&state(7)).unwrap();
        assert!(subscription.validate(&state(7)).is_err());
        assert!(subscription.validate(&state(6)).is_err());
        subscription.validate(&Event::Heartbeat { cursor: subscription.cursor.clone().unwrap() }).unwrap();
        let mut foreign = subscription.cursor.clone().unwrap(); foreign.service_epoch = "cd".repeat(16);
        assert!(subscription.validate(&Event::Heartbeat { cursor: foreign }).is_err());
    }
    #[test]
    fn refuses_oversize_before_reading_payload_and_incompatible_subscribers() {
        assert!(read_frame::<Event>(&mut ((FRAME_LIMIT + 1) as u32).to_le_bytes().as_slice(), FRAME_LIMIT).is_err());
        let mut request = vec![];
        write_frame(&mut request, &Subscribe { operator_schema: ui::OPERATOR_SCHEMA - 1 }, REQUEST_LIMIT).unwrap();
        assert!(accept(&mut request.as_slice()).is_err());
        let mut bytes = vec![]; write(&mut bytes, &state(1)).unwrap();
        assert!(matches!(read_frame::<Event>(&mut bytes.as_slice(), FRAME_LIMIT).unwrap(), Event::State(_)));
    }
}

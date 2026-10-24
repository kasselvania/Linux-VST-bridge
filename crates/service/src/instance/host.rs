//! Ephemeral host self-reports. Admission/retirement authority never comes from
//! these strings, and a service restart deliberately forgets every report.
use super::*;
use std::{io::{self, Read, Write}, os::unix::net::UnixStream, sync::Mutex, time::Duration};
pub use ap1_native_client::host_identity::GREETING;

#[derive(Clone, PartialEq, Eq)]
struct Binding {
    consumer: ConsumerIdentity,
    product: ProductIdentity,
    transport: transport_storage::MemoryTransport,
}
struct Claim { binding: Binding, application: HostApplication }
#[derive(Default)]
pub struct HostNames { names: Mutex<BTreeMap<String, Claim>> }

impl HostNames {
    /// Called only after the service reads GREETING. One small frame, one exact
    /// admitted peer, no persistent source extension or mutation authority.
    pub fn receive(&self, manager: &Manager, peer: &mut UnixStream) -> Result<()> {
        let authenticated = transport_storage::peer_process(peer)?;
        let (session, name) = ap1_native_client::host_identity::read(&mut Deadline {
            peer, end:Instant::now() + Duration::from_millis(250),
        })?;
        let session = hex(&session);
        let records = capture(manager)?;
        let record = records.iter().find(|record| record.owner.session == session
            && record.owner.kind == capacity::Kind::Dsp).ok_or("host_session_not_admitted")?;
        let binding = binding(manager, record)?;
        require(binding.consumer.pid == authenticated.pid && binding.consumer.uid == authenticated.uid
            && binding.consumer.start_ticks == authenticated.start_ticks
            && transport_storage::consumer_current(&binding.consumer)?, "host_peer_binding")?;
        require(transport_storage::peer_process(peer)? == authenticated, "host_peer_changed")?;
        require(same_records(&records, &capture(manager)?), "host_custody_changed")?;
        // No observer waits behind a slow socket; this mutex covers only small
        // in-memory comparisons. Contention is unavailable, not a guessed name.
        let mut names = self.names.try_lock().map_err(|_| "host_observation_busy")?;
        names.retain(|session, _| records.iter().any(|record| &record.owner.session == session));
        if let Some(old) = names.get(&session) {
            require(old.binding == binding && old.application.name == name, "host_report_changed")?;
        } else {
            require(names.len() < 64, "host_observation_bound")?;
            names.insert(session, Claim { binding, application:HostApplication {
                name, source:HostIdentitySource::Vst3HostApplication,
            }});
        }
        drop(names);
        peer.set_write_timeout(Some(Duration::from_millis(250)))?;
        peer.write_all(b"H")?;
        Ok(())
    }

    pub(super) fn observed(&self, manager: &Manager, record: &capacity::ActiveRecord,
        consumer: &Observation<ConsumerObservation>) -> Observation<HostApplication> {
        let value = (|| -> Result<HostApplication> {
            require(matches!(consumer, Observation::Observed(value)
                if value.current == Observation::Observed(true)), "host_consumer_unavailable")?;
            let binding = binding(manager, record)?;
            let names = self.names.try_lock().map_err(|_| "host_observation_busy")?;
            let claim = names.get(&record.owner.session).ok_or("host_report_absent")?;
            require(claim.binding == binding, "host_report_binding")?;
            Ok(claim.application.clone())
        })();
        observed(value, "The admitted host has not supplied a current VST3 name")
    }
}

fn binding(manager: &Manager, record: &capacity::ActiveRecord) -> Result<Binding> {
    Ok(Binding {
        consumer:owner::lease_consumer(manager, &record.owner.session, &record.report, &record.document)?,
        product:product(record)?,
        transport:serde_json::from_value(record.document["transport"].clone())?,
    })
}
struct Deadline<'a> { peer: &'a mut UnixStream, end: Instant }
impl Read for Deadline<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let remaining = self.end.checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "host report deadline"))?;
        self.peer.set_read_timeout(Some(remaining))?;
        self.peer.read(bytes)
    }
}

//! Recovery holds complete, confirmed bytes, independent of a transport lifetime.
//! No plug-in parameters or fixture state layout are interpreted here.
use crate::invalid;
use sha2::{Digest, Sha256};
use std::io;

#[derive(Clone)]
pub struct Snapshot {
    pub bytes: Vec<u8>,
    pub revision: u64,
    pub source: u32,
    pub generation: u64,
    // Admission barrier completed before capture began. Later audio may run
    // concurrently with a read-only capture; this is a lower bound, not a
    // fabricated sample-exact snapshot timestamp.
    pub through: u64,
    pub digest: [u8; 32],
}
#[derive(Default)]
pub struct Store {
    confirmed: Option<Snapshot>,
}
impl Store {
    // Call only after real getState, or exact setState/readback, succeeds.
    pub fn confirm(
        &mut self,
        bytes: Vec<u8>,
        source: u32,
        generation: u64,
        through: u64,
    ) -> io::Result<()> {
        let revision = self
            .confirmed
            .as_ref()
            .map_or(0, |s| s.revision)
            .checked_add(1)
            .ok_or_else(|| invalid("snapshot revision exhausted"))?;
        self.confirmed = Some(Snapshot {
            digest: Sha256::digest(&bytes).into(),
            bytes,
            revision,
            source,
            generation,
            through,
        });
        Ok(())
    }
    pub fn select(&self, revision: u64) -> io::Result<Snapshot> {
        let snapshot = self
            .confirmed
            .as_ref()
            .ok_or_else(|| invalid("no confirmed complete snapshot"))?;
        if snapshot.revision != revision
            || Sha256::digest(&snapshot.bytes).as_slice() != snapshot.digest
        {
            return Err(invalid("snapshot changed or integrity failed"));
        }
        Ok(snapshot.clone())
    }
    pub fn latest(&self) -> Option<&Snapshot> {
        self.confirmed.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_snapshot_isolation_identity_and_stale_selection() {
        let mut a = Store::default();
        let mut b = Store::default();
        assert!(a.select(0).is_err());
        let complete = vec![0xff, 9, 0, 0x83, 0x1a, 0, 67];
        a.confirm(complete.clone(), 16, 1, 40).unwrap();
        b.confirm(vec![3; 201], 18, 1, 0).unwrap();
        assert_eq!(a.select(1).unwrap().bytes, complete);
        assert_eq!(b.select(1).unwrap().bytes, vec![3; 201]);
        a.confirm(vec![5; 37], 16, 1, 50).unwrap();
        assert!(a.select(1).is_err());
        assert_eq!(a.select(2).unwrap().source, 16);
        a.confirmed.as_mut().unwrap().bytes[0] ^= 1;
        assert!(a.select(2).is_err());
        assert_eq!(b.select(1).unwrap().bytes, vec![3; 201]);
    }
}

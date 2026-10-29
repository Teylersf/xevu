//! Recoverable retry journal, not a tamper-resistant/offline guessing boundary.
use crate::{Disk, Error, Result};
use sha2::{Digest, Sha256};

pub const MAX_FAILURES: u8 = 3;
pub const RECORD_LEN: usize = 512;
pub const JOURNAL_LBAS: [u64; 2] = [16, 17];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryRecord {
    pub generation: u64,
    pub failures: u8,
    pub key_hash: [u8; 32],
}
impl RetryRecord {
    pub fn new(header: &[u8]) -> Self {
        Self {
            generation: 1,
            failures: 0,
            key_hash: Sha256::digest(header).into(),
        }
    }
    pub fn bound_to(&self, header: &[u8]) -> bool {
        self.key_hash == <[u8; 32]>::from(Sha256::digest(header))
    }
    pub fn remaining(&self) -> u8 {
        MAX_FAILURES - self.failures
    }
    pub fn triggered(&self) -> bool {
        self.failures == MAX_FAILURES
    }
    pub fn failed(&self) -> Result<Self> {
        if self.triggered() {
            return Err(Error::Policy);
        }
        Ok(Self {
            generation: self.generation.checked_add(1).ok_or(Error::Bounds)?,
            failures: self.failures + 1,
            ..*self
        })
    }
    pub fn reset(&self) -> Result<Self> {
        if self.triggered() {
            return Err(Error::Policy);
        }
        Ok(Self {
            generation: self.generation.checked_add(1).ok_or(Error::Bounds)?,
            failures: 0,
            ..*self
        })
    }
    pub fn encode(&self) -> [u8; RECORD_LEN] {
        let mut b = [0; RECORD_LEN];
        b[..8].copy_from_slice(b"NGTRY001");
        b[8..16].copy_from_slice(&self.generation.to_le_bytes());
        b[16] = self.failures;
        b[32..64].copy_from_slice(&self.key_hash);
        let digest = Sha256::digest(&b[..64]);
        b[64..96].copy_from_slice(&digest);
        b
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() != RECORD_LEN
            || &b[..8] != b"NGTRY001"
            || b[16] > MAX_FAILURES
            || b[17..32].iter().chain(b[96..].iter()).any(|&v| v != 0)
            || Sha256::digest(&b[..64])[..] != b[64..96]
        {
            return Err(Error::Format);
        }
        let generation = u64::from_le_bytes(b[8..16].try_into().map_err(|_| Error::Format)?);
        if generation == 0 {
            return Err(Error::Format);
        }
        Ok(Self {
            generation,
            failures: b[16],
            key_hash: b[32..64].try_into().map_err(|_| Error::Format)?,
        })
    }
}

/// Both blank is an unarmed legacy/new image. Nonblank corruption never resets it.
/// A torn update may fall back to the last committed slot; completed attempts are
/// reported only after a flush and readback of the next slot.
pub fn select(a: &[u8], b: &[u8]) -> Result<Option<RetryRecord>> {
    if a.len() != RECORD_LEN || b.len() != RECORD_LEN {
        return Err(Error::Format);
    }
    let x = RetryRecord::decode(a).ok();
    let y = RetryRecord::decode(b).ok();
    match (x, y) {
        (Some(x), Some(y)) => {
            if x.key_hash != y.key_hash || (x.generation == y.generation && x != y) {
                return Err(Error::Policy);
            }
            Ok(Some(if x.generation >= y.generation { x } else { y }))
        }
        (Some(x), None) | (None, Some(x)) => Ok(Some(x)),
        (None, None) if a.iter().chain(b.iter()).all(|&v| v == 0) => Ok(None),
        _ => Err(Error::Format),
    }
}

/// Never turn a clone/import grant into authority to erase another disk.
pub fn authorize_erasure(d: &Disk) -> Result<()> {
    if !d.boot
        || !d.removable
        || d.read_only
        || d.block_size != 512
        || !matches!(d.access, crate::Access::BootVault)
        || d.blocks < 131072
        || d.blocks > 128 * 1024 * 1024 * 1024 / 512
    {
        return Err(Error::Policy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_failures_survive_serialization_and_success_resets() {
        let mut r = RetryRecord::new(b"key capsule");
        for remaining in [2, 1] {
            r = RetryRecord::decode(&r.failed().unwrap().encode()).unwrap();
            assert_eq!(r.remaining(), remaining);
        }
        let reset = r.reset().unwrap();
        assert_eq!(reset.remaining(), 3);
        assert!(reset.generation > r.generation);
        r = r.failed().unwrap();
        assert!(r.triggered());
        assert!(r.failed().is_err());
        assert!(r.reset().is_err());
    }
    #[test]
    fn journal_torn_write_corruption_and_binding() {
        let r = RetryRecord::new(b"one");
        let next = r.failed().unwrap();
        assert_eq!(select(&r.encode(), &next.encode()).unwrap(), Some(next));
        let mut torn = next.encode();
        torn[70] ^= 1;
        assert_eq!(select(&r.encode(), &torn).unwrap(), Some(r));
        assert!(select(&[9; 512], &torn).is_err());
        assert_eq!(select(&[0; 512], &[0; 512]).unwrap(), None);
        assert!(select(&r.encode(), &RetryRecord::new(b"two").encode()).is_err());
        assert!(r.bound_to(b"one"));
        assert!(!r.bound_to(b"two"));
        let conflict = RetryRecord { failures: 1, ..r };
        assert!(select(&r.encode(), &conflict.encode()).is_err());
    }
    #[test]
    fn erase_authority_is_only_a_writable_removable_boot_medium() {
        let d = Disk {
            id: 1,
            blocks: 524288,
            block_size: 512,
            boot: true,
            removable: true,
            read_only: false,
            access: crate::Access::BootVault,
        };
        authorize_erasure(&d).unwrap();
        for bad in [
            Disk { boot: false, ..d },
            Disk {
                removable: false,
                ..d
            },
            Disk {
                read_only: true,
                ..d
            },
            Disk {
                block_size: 4096,
                ..d
            },
            Disk {
                access: crate::Access::CloneDestination,
                ..d
            },
            Disk {
                blocks: u64::MAX,
                ..d
            },
        ] {
            assert!(authorize_erasure(&bad).is_err());
        }
    }
}

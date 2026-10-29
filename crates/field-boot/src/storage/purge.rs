//! Three-failure handling. Writes are restricted to the identified USB/SD boot
//! disk. Ordinary logical writes only: never sanitize, firmware, or card locks.
use super::*;
use chacha20::{
    XChaCha20,
    cipher::{KeyIvInit, StreamCipher},
};
use vault::retry::{self, JOURNAL_LBAS, RetryRecord};

pub enum Unlock {
    Open(MasterKey),
    Denied(u8),
    Erase,
}

fn read_journal(d: &Physical) -> Result<Option<RetryRecord>> {
    let mut io = borrow_protocol::<BlockIO>(d.handle)?;
    let mut b = Aligned::new(1024, (io.media().io_align() as usize).max(4096))?;
    io.read_blocks(d.media_id, JOURNAL_LBAS[0], b.as_mut())
        .map_err(err)?;
    let bytes = b.as_mut();
    retry::select(&bytes[..512], &bytes[512..]).map_err(err)
}
fn commit(d: &Physical, record: RetryRecord, both: bool) -> Result<()> {
    let mut io = borrow_protocol::<BlockIO>(d.handle)?;
    let align = (io.media().io_align() as usize).max(4096);
    let mut b = Aligned::new(512, align)?;
    let encoded = record.encode();
    let slots: &[u64] = if both {
        &JOURNAL_LBAS
    } else {
        &JOURNAL_LBAS[(record.generation as usize & 1)..][..1]
    };
    for &lba in slots {
        b.as_mut().copy_from_slice(&encoded);
        io.write_blocks(d.media_id, lba, b.as_mut()).map_err(err)?;
        io.flush_blocks().map_err(err)?;
        b.as_mut().fill(0);
        io.read_blocks(d.media_id, lba, b.as_mut()).map_err(err)?;
        if b.as_mut() != encoded {
            return Err("Retry journal readback failed; unlock refused".to_string());
        }
    }
    Ok(())
}
impl Storage {
    fn erase_disk(&self) -> Result<Physical> {
        let disks = self.physical_disks()?;
        let mut boots = disks.into_iter().filter(|d| d.policy.boot);
        let d = boots
            .next()
            .ok_or_else(|| "No physical boot medium".to_string())?;
        if boots.next().is_some() || !external_path(&d.path) {
            return Err(
                "Exactly one USB/SD boot medium is required for the retry policy".to_string(),
            );
        }
        retry::authorize_erasure(&d.policy).map_err(err)?;
        let mut io = borrow_protocol::<BlockIO>(d.handle)?;
        let mut b = Aligned::new(512, (io.media().io_align() as usize).max(4096))?;
        io.read_blocks(d.media_id, 0, b.as_mut()).map_err(err)?;
        vault::portable_extent(b.as_mut(), 512, d.policy.blocks).map_err(err)?;
        // The raw journal is only in the gap owned by our image layout.
        if u32::from_le_bytes(b.as_mut()[454..458].try_into().map_err(err)?) != 2048 {
            return Err("Unknown boot layout; raw retry writes refused".to_string());
        }
        Ok(d)
    }
    pub fn remaining_attempts(&self) -> Result<u8> {
        Ok(read_journal(&self.erase_disk()?)?.map_or(3, |r| r.remaining()))
    }
    pub fn purge_pending(&self) -> Result<bool> {
        Ok(read_journal(&self.erase_disk()?)?.is_some_and(|r| r.triggered()))
    }
    pub(super) fn arm_retry(&self, header: &[u8]) -> Result<()> {
        let d = self.erase_disk()?;
        if let Some(r) = read_journal(&d)? {
            if !r.bound_to(header) || r.triggered() {
                return Err("Existing retry policy cannot be replaced".to_string());
            }
            return Ok(());
        }
        commit(&d, RetryRecord::new(header), true)
    }
    pub fn unlock(&mut self, password: &[u8]) -> Result<Unlock> {
        let d = self.erase_disk()?;
        let existing = read_journal(&d)?;
        if existing.is_some_and(|r| r.triggered()) {
            return Ok(Unlock::Erase);
        }
        let h = read(self.boot_root()?, KEY, 128)?;
        let record = match existing {
            Some(r) if r.bound_to(&h) => r,
            Some(_) => {
                return Err("Vault differs from its retry journal; unlock refused".to_string());
            }
            None => {
                // Arm 0.1.0 and host-prepared vaults on their first native attempt.
                let r = RetryRecord::new(&h);
                commit(&d, r, true)?;
                r
            }
        };
        match vault::unlock_key_header(password, &h) {
            Ok(k) => {
                if record.failures != 0 {
                    commit(&d, record.reset().map_err(err)?, false)?;
                }
                Ok(Unlock::Open(k))
            }
            Err(vault::Error::Authentication) => {
                let next = record.failed().map_err(err)?;
                // Persist and verify BEFORE reporting the failure or starting erasure.
                commit(&d, next, next.triggered())?;
                if next.triggered() {
                    Ok(Unlock::Erase)
                } else {
                    Ok(Unlock::Denied(next.remaining()))
                }
            }
            Err(e) => Err(format!(
                "Vault format/KDF error; attempt not charged: {e:?}"
            )),
        }
    }

    pub fn erase_boot_medium(&mut self, mut progress: impl FnMut(&str, u64, u64)) -> Result<()> {
        self.revoke_all();
        let d = self.erase_disk()?;
        if !read_journal(&d)?.is_some_and(|r| r.triggered()) {
            return Err("Three-failure trigger absent; erasure refused".to_string());
        }
        // Obtain entropy before any key or raw write. The capsule and CSPRNG
        // seed are independent. XChaCha20 uses a different nonce for each LBA batch.
        let capsule = Zeroizing::new(random::<128>()?);
        let seed = Zeroizing::new(random::<48>()?);
        progress("DESTROYING KEY CAPSULE", 0, d.policy.blocks);
        // On interrupted erasure this file may already be unreadable. The durable
        // trigger still permits the complete logical pass without unlocking it.
        if let Ok(mut f) = open(self.boot_root()?, KEY, FileMode::ReadWrite) {
            if f.get_boxed_info::<FileInfo>().map_err(err)?.file_size() != 128 {
                return Err("Unexpected key capsule size; erasure halted".to_string());
            }
            f.write(&*capsule)
                .map_err(|e| format!("Key write: {e:?}"))?;
            f.flush().map_err(|e| format!("Key flush: {e:?}"))?;
            f.set_position(0).map_err(|e| format!("Key seek: {e:?}"))?;
            let mut check = Zeroizing::new([0; 128]);
            read_exact(&mut f, &mut *check)?;
            if *check != *capsule {
                return Err("Key destruction readback failed".to_string());
            }
        }
        self.boot_root()?
            .flush()
            .map_err(|e| format!("Filesystem flush: {e:?}"))?;
        self.root = None; // Close files before invalidating their filesystem.
        self.volumes.clear();
        // Keep the BlockIO provider connected until the raw pass finishes.
        // DisconnectController can remove that provider itself on USB firmware.
        // All our filesystem handles have been flushed/closed and are never used
        // again; firmware cache correctness remains part of the trust boundary.
        let mut io =
            borrow_protocol::<BlockIO>(d.handle).map_err(|e| format!("Boot BlockIO: {e}"))?;
        let m = io.media();
        if m.media_id() != d.media_id
            || m.is_read_only()
            || !m.is_media_present()
            || m.block_size() != 512
            || m.last_block().checked_add(1) != Some(d.policy.blocks)
            || path(d.handle)? != d.path
        {
            return Err("Boot medium identity changed; erasure halted".to_string());
        }
        let align = (m.io_align() as usize).max(4096);
        let mut data = Aligned::new(1024 * 1024, align)?;
        let mut verify = Aligned::new(1024 * 1024, align)?;
        let mut end = d.policy.blocks;
        // Keep MBR + journal until the body has been readback verified. Data
        // beyond the installed partition is included, unlike normal cloning.
        while end > 18 {
            let start = end.saturating_sub(2048).max(18);
            let n = ((end - start) * 512) as usize;
            let b = &mut data.as_mut()[..n];
            b.fill(0);
            let mut nonce = [0; 24];
            nonce[..16].copy_from_slice(&seed[32..]);
            nonce[16..].copy_from_slice(&start.to_le_bytes());
            let mut cipher = XChaCha20::new_from_slices(&seed[..32], &nonce).map_err(err)?;
            cipher.try_apply_keystream(b).map_err(err)?;
            io.write_blocks(d.media_id, start, b).map_err(err)?;
            io.flush_blocks().map_err(err)?;
            let check = &mut verify.as_mut()[..n];
            io.read_blocks(d.media_id, start, check).map_err(err)?;
            if Sha256::digest(&*b) != Sha256::digest(&*check) {
                return Err(format!("Erasure verification failed at LBA {start}"));
            }
            end = start;
            progress(
                "SCRAMBLING / READBACK VERIFYING ALL LOGICAL SECTORS",
                d.policy.blocks - end,
                d.policy.blocks,
            );
        }
        // Clear the gap/journal and MBR last. Filesystem boot sectors and EFI
        // binaries were already overwritten in the body. No valid boot record remains.
        let b = &mut data.as_mut()[..18 * 512];
        b.fill(0);
        io.write_blocks(d.media_id, 0, b).map_err(err)?;
        io.flush_blocks().map_err(err)?;
        let check = &mut verify.as_mut()[..18 * 512];
        io.read_blocks(d.media_id, 0, check).map_err(err)?;
        if check.iter().any(|&v| v != 0) {
            return Err("Boot record invalidation failed".to_string());
        }
        drop(io);
        progress(
            "CARD ERASED / REMOVING DEVICE",
            d.policy.blocks,
            d.policy.blocks,
        );
        // Best-effort logical detach. Readers without mechanical eject support
        // use the caller's shutdown for safe physical removal; card remains reusable.
        let _ = boot::disconnect_controller(d.handle, None, None);
        Ok(())
    }
}

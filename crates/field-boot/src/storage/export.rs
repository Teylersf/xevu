//! Explicit plaintext release, authenticated in full before destination writes.
use super::*;
use uefi::proto::media::file::FileSystemInfo;
pub const EXPORT_DIR: &str = "\\NULLGRAV_EXPORTS";
const RECEIPTS: &str = "\\NG_EXPORT_LOG";

#[derive(Clone)]
pub struct ExportTarget {
    pub name: String,
    handle: Handle,
    path: Vec<u8>,
    disk: Physical,
}

impl Storage {
    pub fn export_targets(&self) -> Result<Vec<ExportTarget>> {
        let disks = self.physical_disks()?;
        let boots: Vec<_> = disks.iter().filter(|d| d.policy.boot).collect();
        if boots.len() != 1 {
            return Err("Cannot identify the boot disk; export refused".to_string());
        }
        let mut targets = Vec::new();
        for volume in &self.volumes {
            let vp = path(volume.handle)?;
            let candidates: Vec<_> = disks
                .iter()
                .filter(|d| !d.path.is_empty() && vp.starts_with(&d.path))
                .collect();
            if candidates.len() != 1 {
                continue;
            }
            let disk = candidates[0];
            if vault::authorize_plaintext_export(&boots[0].policy, &disk.policy).is_err() {
                continue;
            }
            let mut r = root(volume.handle)?;
            let info = r.get_boxed_info::<FileSystemInfo>().map_err(err)?;
            if info.read_only() {
                continue;
            }
            let label = info.volume_label().to_string();
            targets.push(ExportTarget {
                name: format!(
                    "{} / {} MiB FREE",
                    if label.is_empty() {
                        volume.name.clone()
                    } else {
                        label
                    },
                    info.free_space() / 1048576
                ),
                handle: volume.handle,
                path: vp,
                disk: disk.clone(),
            });
        }
        Ok(targets)
    }
    pub fn export_plaintext(
        &mut self,
        object: &str,
        key: &MasterKey,
        selected: &ExportTarget,
    ) -> Result<(String, String)> {
        self.revoke_all();
        let result = (|| {
            // No media decoding or conversion. Every AEAD chunk authenticates
            // before even creating a destination directory or plaintext file.
            let (name, data) = self.view(object, key)?;
            if !vault::windows_name(&name) {
                return Err("Original name is not Windows-compatible; export refused".to_string());
            }
            let target = self
                .export_targets()?
                .into_iter()
                .find(|d| {
                    d.handle == selected.handle
                        && d.path == selected.path
                        && d.disk.handle == selected.disk.handle
                        && d.disk.media_id == selected.disk.media_id
                        && d.disk.policy.blocks == selected.disk.policy.blocks
                })
                .ok_or_else(|| {
                    "Destination changed or is protected; select it again".to_string()
                })?;
            let mut r = root(target.handle)?;
            if r.get_boxed_info::<FileSystemInfo>()
                .map_err(err)?
                .free_space()
                < data.len() as u64 + 16384
            {
                return Err("Destination does not have enough free space".to_string());
            }
            mkdir(&mut r, EXPORT_DIR)?;
            mkdir(&mut r, RECEIPTS)?;
            let final_path = format!("{EXPORT_DIR}\\{name}");
            match r.open(&cstr(&final_path)?, FileMode::Read, FileAttribute::empty()) {
                Ok(_) => {
                    return Err("A file with that name already exists; no overwrite".to_string());
                }
                Err(e) if e.status() == uefi::Status::NOT_FOUND => {}
                Err(e) => return Err(err(e)),
            }
            let id: String = random::<12>()?.iter().map(|b| format!("{b:02X}")).collect();
            let staging = format!("{EXPORT_DIR}\\NG{id}.TMP");
            write_new(&mut r, &staging, &data)?;
            let outcome = (|| {
                let back = read(&mut r, &staging, LIMIT)?;
                let digest = Sha256::digest(&*data);
                if back.len() != data.len() || Sha256::digest(&*back) != digest {
                    return Err("Plaintext readback mismatch".to_string());
                }
                drop(back);
                // Rename only a fully verified temporary file. FileInfo needs
                // an 8-byte aligned buffer; no borrowed info outlives this call.
                let mut f = open(&mut r, &staging, FileMode::ReadWrite)?;
                let info = f.get_boxed_info::<FileInfo>().map_err(err)?;
                let mut buffer = Aligned::new(2048, 8)?;
                let new_name = cstr(&final_path)?;
                let renamed = FileInfo::new(
                    buffer.as_mut(),
                    info.file_size(),
                    info.physical_size(),
                    *info.create_time(),
                    *info.last_access_time(),
                    *info.modification_time(),
                    info.attribute(),
                    &new_name,
                )
                .map_err(err)?;
                f.set_info(renamed)
                    .map_err(|e| format!("Export rename: {e:?}"))?;
                f.flush().map_err(err)?;
                drop(f);
                let final_back = read(&mut r, &final_path, LIMIT)?;
                if final_back.len() != data.len() || Sha256::digest(&*final_back) != digest {
                    return Err("Final plaintext readback mismatch".to_string());
                }
                drop(final_back);
                let hash: String = digest.iter().map(|b| format!("{b:02x}")).collect();
                // The Windows companion accepts only this committed, hashed
                // record after the VM has shut down and released the image.
                let receipt = format!(
                    "{{\"version\":1,\"name\":\"{name}\",\"bytes\":{},\"sha256\":\"{hash}\"}}\n",
                    data.len()
                );
                write_new(
                    &mut r,
                    &format!("{RECEIPTS}\\{id}.JSON"),
                    receipt.as_bytes(),
                )?;
                r.flush().map_err(err)?;
                Ok((name, hash))
            })();
            if outcome.is_err()
                && let Ok(f) = open(&mut r, &staging, FileMode::ReadWrite)
            {
                let _ = f.delete();
            }
            outcome
        })();
        self.revoke_all();
        result
    }
}

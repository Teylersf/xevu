//! Firmware-backed storage broker. This is an application policy, NOT a
//! hardware isolation boundary; UEFI itself and every native parser are trusted.
use alloc::{
    alloc::{Layout, alloc_zeroed, dealloc},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::ptr::NonNull;
use nullgrav_vault::{self as vault, Access, Disk, FileCipher, META_LEN, MasterKey, TAG_LEN};
use sha2::{Digest, Sha256};
use uefi::{
    CString16, Handle, boot,
    proto::{
        device_path::DevicePath,
        loaded_image::LoadedImage,
        media::{
            block::BlockIO,
            file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile},
            fs::SimpleFileSystem,
        },
        rng::Rng,
    },
};
use zeroize::Zeroizing;
pub type Result<T> = core::result::Result<T, String>;
pub const VAULT: &str = "\\NULLGRAV\\VAULT";
pub const KEY: &str = "\\NULLGRAV\\VAULT\\VAULT.NGK";
pub const LIMIT: usize = 128 * 1024 * 1024;
fn err(e: impl core::fmt::Debug) -> String {
    format!("{e:?}")
}
fn cstr(s: &str) -> Result<CString16> {
    CString16::try_from(s).map_err(err)
}
fn borrow_protocol<P: uefi::proto::ProtocolPointer + ?Sized>(
    handle: Handle,
) -> Result<boot::ScopedProtocol<P>> {
    // No driver disconnects occur while these short-lived borrows are active.
    // This single-threaded broker never retains a BlockIO borrow across discovery.
    // EXCLUSIVE is deliberately avoided: it would disconnect the firmware's
    // filesystem driver and invalidate our open boot volume.
    unsafe {
        boot::open_protocol::<P>(
            boot::OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol,
        )
    }
    .map_err(err)
}
fn path(h: Handle) -> Result<Vec<u8>> {
    let p = borrow_protocol::<DevicePath>(h)?;
    let b = p.as_bytes();
    Ok(b[..b.len().saturating_sub(4)].to_vec())
}
fn external_path(p: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 <= p.len() {
        let n = u16::from_le_bytes([p[i + 2], p[i + 3]]) as usize;
        if n < 4 || i + n > p.len() {
            return false;
        }
        if p[i] == 3 && matches!(p[i + 1], 5 | 15 | 16 | 26) {
            return true;
        }
        i += n;
    }
    false
}
fn root(h: Handle) -> Result<Directory> {
    let mut f = borrow_protocol::<SimpleFileSystem>(h)?;
    f.open_volume().map_err(err)
}
fn open(root: &mut Directory, p: &str, mode: FileMode) -> Result<RegularFile> {
    root.open(&cstr(p)?, mode, FileAttribute::empty())
        .map_err(err)?
        .into_regular_file()
        .ok_or_else(|| "Not a regular file".to_string())
}
fn read_exact(f: &mut RegularFile, b: &mut [u8]) -> Result<()> {
    let mut n = 0;
    while n < b.len() {
        let got = f.read(&mut b[n..]).map_err(err)?;
        if got == 0 {
            return Err("Unexpected end of file".to_string());
        }
        n += got;
    }
    Ok(())
}
fn read(root: &mut Directory, p: &str, limit: usize) -> Result<Zeroizing<Vec<u8>>> {
    let mut f = open(root, p, FileMode::Read)?;
    let info = f.get_boxed_info::<FileInfo>().map_err(err)?;
    let len = info.file_size();
    if len > limit as u64 {
        return Err("File exceeds this build's 128 MiB memory limit".to_string());
    }
    let mut b = Zeroizing::new(vec![0; len as usize]);
    read_exact(&mut f, &mut b)?;
    Ok(b)
}
fn write_new(root: &mut Directory, p: &str, data: &[u8]) -> Result<()> {
    match root.open(&cstr(p)?, FileMode::Read, FileAttribute::empty()) {
        Ok(_) => return Err("Destination already exists; refusing overwrite".to_string()),
        Err(e) if e.status() == uefi::Status::NOT_FOUND => {}
        Err(e) => return Err(err(e)),
    }
    let mut f = open(root, p, FileMode::CreateReadWrite)?;
    if let Err(e) = f.write(data) {
        let _ = f.delete();
        return Err(err(e));
    }
    if let Err(e) = f.flush() {
        let _ = f.delete();
        return Err(err(e));
    }
    drop(f);
    Ok(())
}
fn mkdir(root: &mut Directory, p: &str) -> Result<()> {
    root.open(
        &cstr(p)?,
        FileMode::CreateReadWrite,
        FileAttribute::DIRECTORY,
    )
    .map_err(err)?;
    Ok(())
}
#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub directory: bool,
}
fn list(root: &mut Directory, p: &str) -> Result<Vec<Entry>> {
    let mut d = root
        .open(&cstr(p)?, FileMode::Read, FileAttribute::empty())
        .map_err(err)?
        .into_directory()
        .ok_or_else(|| "Not a directory".to_string())?;
    let mut out = Vec::new();
    while let Some(i) = d.read_entry_boxed().map_err(err)? {
        let name = i.file_name().to_string();
        if name == "." || name == ".." {
            continue;
        }
        if out.len() >= 512 {
            return Err("Directory exceeds 512 entries; narrow the import folder".to_string());
        }
        out.push(Entry {
            name,
            size: i.file_size(),
            directory: i.attribute().contains(FileAttribute::DIRECTORY),
        });
    }
    out.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}
#[derive(Clone)]
pub struct Volume {
    pub handle: Handle,
    pub name: String,
    pub boot: bool,
    pub allowed: bool,
}
#[derive(Clone)]
pub struct Physical {
    pub handle: Handle,
    pub policy: Disk,
    pub media_id: u32,
    pub path: Vec<u8>,
    pub name: String,
}
pub struct Storage {
    pub boot_handle: Handle,
    pub boot_path: Vec<u8>,
    pub root: Directory,
    pub volumes: Vec<Volume>,
}
impl Storage {
    pub fn new() -> Result<Self> {
        let loaded =
            boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).map_err(err)?;
        let h = loaded
            .device()
            .ok_or_else(|| "Missing boot device handle".to_string())?;
        drop(loaded);
        let bp = path(h)?;
        let mut s = Self {
            boot_handle: h,
            boot_path: bp,
            root: root(h)?,
            volumes: Vec::new(),
        };
        s.refresh_volumes()?;
        Ok(s)
    }
    pub fn refresh_volumes(&mut self) -> Result<()> {
        // Firmware may connect only the boot path. Ask it to discover remaining
        // controllers; it can probe disks here even though our broker grants no
        // file access. This is why the build does not claim kernel isolation.
        for _ in 0..2 {
            if let Ok(handles) = boot::locate_handle_buffer(boot::SearchType::AllHandles) {
                for &handle in handles.iter() {
                    let _ = boot::connect_controller(handle, &[], None, true);
                }
            }
        }
        self.volumes.clear();
        for (hidx, h) in boot::find_handles::<SimpleFileSystem>()
            .map_err(err)?
            .into_iter()
            .enumerate()
        {
            self.volumes.push(Volume {
                handle: h,
                name: format!("VOLUME {:02}", hidx + 1),
                boot: h == self.boot_handle,
                allowed: false,
            });
        }
        Ok(())
    }
    pub fn has_key(&mut self) -> bool {
        open(&mut self.root, KEY, FileMode::Read).is_ok()
    }
    pub fn initialize(&mut self, password: &[u8]) -> Result<MasterKey> {
        if self.has_key() {
            return Err("Vault already exists; refusing to replace its key".to_string());
        }
        let entropy = Zeroizing::new(random::<72>()?);
        let (h, k) = vault::create_key_header(password, &entropy).map_err(err)?;
        mkdir(&mut self.root, "\\NULLGRAV")?;
        mkdir(&mut self.root, VAULT)?;
        write_new(&mut self.root, KEY, &h)?;
        let check = read(&mut self.root, KEY, 128)?;
        let _ = vault::unlock_key_header(password, &check).map_err(err)?;
        Ok(k)
    }
    pub fn unlock(&mut self, password: &[u8]) -> Result<MasterKey> {
        let h = read(&mut self.root, KEY, 128)?;
        vault::unlock_key_header(password, &h).map_err(err)
    }
    pub fn encrypted_files(&mut self) -> Vec<Entry> {
        list(&mut self.root, VAULT)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| !e.directory && e.name.to_ascii_uppercase().ends_with(".NGV"))
            .collect()
    }
    pub fn metadata(&mut self, name: &str, key: &MasterKey) -> Result<(String, u64)> {
        if !vault::valid_name(name) {
            return Err("Invalid object name".to_string());
        }
        let mut f = open(&mut self.root, &format!("{VAULT}\\{name}"), FileMode::Read)?;
        let size = f.get_boxed_info::<FileInfo>().map_err(err)?.file_size();
        let mut h = [0; 64];
        let mut m = vec![0; META_LEN + TAG_LEN];
        read_exact(&mut f, &mut h)?;
        read_exact(&mut f, &mut m)?;
        let c = FileCipher::open(key, &h, &m).map_err(err)?;
        if c.encoded_length().map_err(err)? != size {
            return Err("Incomplete or extended object".to_string());
        }
        Ok((c.name.clone(), c.length))
    }
    pub fn view(&mut self, name: &str, key: &MasterKey) -> Result<(String, Zeroizing<Vec<u8>>)> {
        if !vault::valid_name(name) {
            return Err("Invalid object name".to_string());
        }
        let b = read(
            &mut self.root,
            &format!("{VAULT}\\{name}"),
            LIMIT + 256 * 1024,
        )?;
        vault::unseal(key, &b, LIMIT).map_err(err)
    }
    pub fn grant_volume(&mut self, i: usize) -> Result<()> {
        let v = self
            .volumes
            .get_mut(i)
            .ok_or_else(|| "Volume disappeared".to_string())?;
        if v.boot {
            return Err("Choose another volume".to_string());
        }
        v.allowed = true;
        Ok(())
    }
    pub fn revoke_all(&mut self) {
        for v in &mut self.volumes {
            v.allowed = false;
        }
    }
    pub fn browse(&self, i: usize, p: &str) -> Result<Vec<Entry>> {
        let v = self
            .volumes
            .get(i)
            .ok_or_else(|| "Volume disappeared".to_string())?;
        if !v.allowed || v.boot {
            return Err("Disk access denied by default policy".to_string());
        }
        list(&mut root(v.handle)?, p)
    }
    pub fn import(&mut self, i: usize, p: &str, name: &str, key: &MasterKey) -> Result<()> {
        let v = self
            .volumes
            .get(i)
            .ok_or_else(|| "Volume disappeared".to_string())?;
        if !v.allowed || v.boot {
            return Err("Import permission is absent".to_string());
        }
        let result = (|| {
            let data = read(&mut root(v.handle)?, p, LIMIT)?;
            let random = random::<32>()?;
            let b = vault::seal(key, &random, name, &data).map_err(err)?;
            let id = random[..16]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<String>();
            let dest = format!("{VAULT}\\{id}.NGV");
            write_new(&mut self.root, &dest, &b)?;
            let reread = read(&mut self.root, &dest, LIMIT + 256 * 1024)?;
            let (_, plain) = vault::unseal(key, &reread, LIMIT).map_err(err)?;
            if Sha256::digest(&*plain) != Sha256::digest(&*data) {
                return Err("Import readback mismatch".to_string());
            }
            Ok(())
        })();
        self.revoke_all();
        result
    }
    pub fn physical_disks(&self) -> Result<Vec<Physical>> {
        let mut out = Vec::new();
        for h in boot::find_handles::<BlockIO>().map_err(err)? {
            let io = borrow_protocol::<BlockIO>(h)?;
            let m = io.media();
            if !m.is_media_present() || m.is_logical_partition() {
                continue;
            }
            let p = path(h)?;
            let is_boot = self.boot_path.starts_with(&p) && !p.is_empty();
            let usb_or_sd = external_path(&p);
            let blocks = m
                .last_block()
                .checked_add(1)
                .ok_or_else(|| "Device size overflow".to_string())?;
            let policy = Disk {
                id: h.as_ptr() as usize as u64,
                blocks,
                block_size: m.block_size(),
                boot: is_boot,
                removable: m.is_removable_media() || usb_or_sd,
                read_only: m.is_read_only(),
                access: if is_boot {
                    Access::BootVault
                } else {
                    Access::Denied
                },
            };
            out.push(Physical {
                handle: h,
                media_id: m.media_id(),
                path: p,
                name: format!(
                    "DISK {:02} / {} MiB",
                    out.len() + 1,
                    blocks.saturating_mul(m.block_size() as u64) / 1048576
                ),
                policy,
            });
        }
        Ok(out)
    }
    pub fn clone_drive(
        &mut self,
        target: &Physical,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<String> {
        self.revoke_all();
        self.root.flush().map_err(err)?;
        // Re-enumerate and re-check the selected identity immediately before writing.
        let disks = self.physical_disks()?;
        let sources: Vec<_> = disks.iter().filter(|d| d.policy.boot).collect();
        if sources.len() != 1 {
            return Err(
                "Cannot identify exactly one physical boot disk; clone refused".to_string(),
            );
        }
        let mut src = sources[0].clone();
        let mut dst = disks
            .iter()
            .find(|d| {
                d.handle == target.handle
                    && d.path == target.path
                    && d.media_id == target.media_id
                    && d.policy.blocks == target.policy.blocks
            })
            .cloned()
            .ok_or_else(|| "Destination changed; select it again".to_string())?;
        let mut si = borrow_protocol::<BlockIO>(src.handle)?;
        let mut first = Aligned::new(4096, 4096)?;
        if src.policy.block_size != 512 {
            return Err("This image requires 512-byte-sector media".to_string());
        }
        si.read_blocks(src.media_id, 0, &mut first.as_mut()[..512])
            .map_err(err)?;
        src.policy.blocks = vault::portable_extent(
            &first.as_mut()[..512],
            src.policy.block_size,
            src.policy.blocks,
        )
        .map_err(err)?;
        vault::authorize_clone(&src.policy, &mut dst.policy).map_err(err)?;
        let mut di = borrow_protocol::<BlockIO>(dst.handle)?;
        let bytes = 1024 * 1024;
        let align = core::cmp::max(
            4096,
            core::cmp::max(si.media().io_align(), di.media().io_align()) as usize,
        );
        let mut b = Aligned::new(bytes, align)?;
        let mut verify = Aligned::new(bytes, align)?;
        let blocks_per = (bytes / src.policy.block_size as usize) as u64;
        let mut lba = 0;
        let mut hasher = Sha256::new();
        while lba < src.policy.blocks {
            let blocks = core::cmp::min(blocks_per, src.policy.blocks - lba);
            let n = blocks as usize * src.policy.block_size as usize;
            let data = &mut b.as_mut()[..n];
            si.read_blocks(src.media_id, lba, data).map_err(err)?;
            if !dst.policy.can_write() {
                return Err("Destination write grant absent".to_string());
            }
            di.write_blocks(dst.media_id, lba, data).map_err(err)?;
            di.flush_blocks().map_err(err)?;
            let check = &mut verify.as_mut()[..n];
            di.read_blocks(dst.media_id, lba, check).map_err(err)?;
            if Sha256::digest(&*data) != Sha256::digest(&*check) {
                return Err(format!("Clone verification failed at LBA {lba}"));
            }
            hasher.update(data);
            lba += blocks;
            progress(lba, src.policy.blocks);
        }
        di.flush_blocks().map_err(err)?;
        let digest = hasher.finalize();
        Ok(digest.iter().map(|x| format!("{x:02x}")).collect())
    }
}
struct Aligned {
    ptr: NonNull<u8>,
    layout: Layout,
}
impl Aligned {
    fn new(n: usize, align: usize) -> Result<Self> {
        let layout = Layout::from_size_align(n, align).map_err(err)?;
        let p = unsafe { alloc_zeroed(layout) };
        Ok(Self {
            ptr: NonNull::new(p).ok_or_else(|| "Out of memory".to_string())?,
            layout,
        })
    }
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size()) }
    }
}
impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}
pub fn random<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0u8; N];
    if let Ok(handles) = boot::find_handles::<Rng>() {
        for h in handles {
            if let Ok(mut r) = boot::open_protocol_exclusive::<Rng>(h)
                && r.get_rng(None, &mut out).is_ok()
            {
                return Ok(out);
            }
        }
    }
    // Apollo Lake supports RDRAND. Never fall back to clocks or user input.
    #[cfg(target_arch = "x86_64")]
    {
        if core::arch::x86_64::__cpuid(1).ecx & (1 << 30) != 0 {
            for chunk in out.chunks_mut(8) {
                let mut word = 0u64;
                let mut ok = false;
                for _ in 0..32 {
                    if unsafe { core::arch::x86_64::_rdrand64_step(&mut word) } == 1 {
                        ok = true;
                        break;
                    }
                }
                if !ok {
                    return Err("Hardware RNG failed; encryption refused".to_string());
                }
                chunk.copy_from_slice(&word.to_le_bytes()[..chunk.len()]);
            }
            return Ok(out);
        }
    }
    Err("No cryptographic random generator; encryption refused".to_string())
}

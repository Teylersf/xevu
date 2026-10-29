#![no_std]
//! NULLGRAV's experimental file envelope. Existing cryptographic primitives;
//! the composition and implementation have NOT received an independent audit.
extern crate alloc;
use alloc::{string::String, vec, vec::Vec};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const KEY_HEADER_LEN: usize = 128;
pub const FILE_HEADER_LEN: usize = 64;
pub const META_LEN: usize = 4096;
pub const TAG_LEN: usize = 16;
pub const CHUNK_LEN: usize = 65536;
pub const MIN_PASSWORD: usize = 16;
const KEY_MAGIC: &[u8; 8] = b"NGKEY001";
const FILE_MAGIC: &[u8; 8] = b"NGFILE01";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Format,
    Authentication,
    Kdf,
    Bounds,
    Name,
    Policy,
    Password,
}
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);
impl MasterKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

fn u32_at(b: &[u8], i: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(i..i + 4)
            .ok_or(Error::Format)?
            .try_into()
            .map_err(|_| Error::Format)?,
    ))
}
fn u64_at(b: &[u8], i: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(i..i + 8)
            .ok_or(Error::Format)?
            .try_into()
            .map_err(|_| Error::Format)?,
    ))
}
fn password_key(password: &[u8], header: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    // Read untrusted KDF parameters only after bounding CPU and memory usage.
    let m = u32_at(header, 12)?;
    let t = u32_at(header, 16)?;
    let p = u32_at(header, 20)?;
    if !(8192..=65536).contains(&m) || !(1..=3).contains(&t) || p != 1 {
        return Err(Error::Kdf);
    }
    let params = Params::new(m, t, p, Some(32)).map_err(|_| Error::Kdf)?;
    let kdf = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    kdf.hash_password_into(password, &header[24..40], &mut *key)
        .map_err(|_| Error::Kdf)?;
    Ok(key)
}

/// All random material must come from an OS/hardware cryptographic RNG.
/// `entropy`: root key (32), salt (16), nonce (24); never reuse it.
pub fn create_key_header(
    password: &[u8],
    entropy: &[u8; 72],
) -> Result<([u8; KEY_HEADER_LEN], MasterKey)> {
    if password.len() < MIN_PASSWORD {
        return Err(Error::Password);
    }
    let mut h = [0u8; KEY_HEADER_LEN];
    h[..8].copy_from_slice(KEY_MAGIC);
    h[8..12].copy_from_slice(&1u32.to_le_bytes());
    h[12..16].copy_from_slice(&65536u32.to_le_bytes());
    h[16..20].copy_from_slice(&3u32.to_le_bytes());
    h[20..24].copy_from_slice(&1u32.to_le_bytes());
    h[24..40].copy_from_slice(&entropy[32..48]);
    h[40..64].copy_from_slice(&entropy[48..72]);
    let k = password_key(password, &h)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&*k).map_err(|_| Error::Format)?;
    let wrapped = cipher
        .encrypt(
            XNonce::from_slice(&h[40..64]),
            Payload {
                msg: &entropy[..32],
                aad: &h[..64],
            },
        )
        .map_err(|_| Error::Authentication)?;
    h[64..112].copy_from_slice(&wrapped);
    let mut root = [0u8; 32];
    root.copy_from_slice(&entropy[..32]);
    Ok((h, MasterKey(root)))
}

pub fn unlock_key_header(password: &[u8], h: &[u8]) -> Result<MasterKey> {
    if h.len() != KEY_HEADER_LEN
        || &h[..8] != KEY_MAGIC
        || u32_at(h, 8)? != 1
        || h[112..].iter().any(|&b| b != 0)
    {
        return Err(Error::Format);
    }
    let k = password_key(password, h)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&*k).map_err(|_| Error::Format)?;
    let mut plain = Zeroizing::new(
        cipher
            .decrypt(
                XNonce::from_slice(&h[40..64]),
                Payload {
                    msg: &h[64..112],
                    aad: &h[..64],
                },
            )
            .map_err(|_| Error::Authentication)?,
    );
    let mut root = [0u8; 32];
    root.copy_from_slice(&plain);
    plain.zeroize();
    Ok(MasterKey(root))
}

fn derive(master: &MasterKey, id: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let hk = Hkdf::<Sha256>::new(Some(id), &master.0);
    let mut k = Zeroizing::new([0u8; 32]);
    hk.expand(b"NULLGRAV/FILE/v1", &mut *k)
        .map_err(|_| Error::Format)?;
    Ok(k)
}
fn nonce(header: &[u8; 64], index: u64) -> [u8; 24] {
    let mut n = [0u8; 24];
    n[..16].copy_from_slice(&header[24..40]);
    n[16..].copy_from_slice(&index.to_le_bytes());
    n
}
fn aad(header: &[u8; 64], index: u64) -> [u8; 72] {
    let mut a = [0u8; 72];
    a[..64].copy_from_slice(header);
    a[64..].copy_from_slice(&index.to_le_bytes());
    a
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '\0'))
}

pub struct FileCipher {
    header: [u8; 64],
    key: Zeroizing<[u8; 32]>,
    pub name: String,
    pub length: u64,
    pub chunks: u64,
    next_seal: Option<u64>,
}
impl Drop for FileCipher {
    fn drop(&mut self) {
        self.name.zeroize();
    }
}
impl FileCipher {
    /// entropy: random file id (16) and independent nonce prefix (16).
    pub fn create(
        master: &MasterKey,
        entropy: &[u8; 32],
        name: &str,
        length: u64,
    ) -> Result<(Self, Vec<u8>)> {
        if !valid_name(name) {
            return Err(Error::Name);
        }
        let chunks = length.div_ceil(CHUNK_LEN as u64);
        if chunks > u32::MAX as u64 {
            return Err(Error::Bounds);
        }
        let mut header = [0u8; 64];
        header[..8].copy_from_slice(FILE_MAGIC);
        header[8..40].copy_from_slice(entropy);
        header[40..48].copy_from_slice(&chunks.to_le_bytes());
        header[48..52].copy_from_slice(&(CHUNK_LEN as u32).to_le_bytes());
        let key = derive(master, &header[8..24])?;
        let f = Self {
            header,
            key,
            name: String::from(name),
            length,
            chunks,
            next_seal: Some(1),
        };
        let mut meta = Zeroizing::new(vec![0u8; META_LEN]);
        meta[..8].copy_from_slice(&length.to_le_bytes());
        meta[8..10].copy_from_slice(&(name.len() as u16).to_le_bytes());
        meta[10..10 + name.len()].copy_from_slice(name.as_bytes());
        let c = f.encrypt(0, &meta)?;
        Ok((f, c))
    }
    pub fn open(master: &MasterKey, header: &[u8], metadata: &[u8]) -> Result<Self> {
        if header.len() != 64
            || &header[..8] != FILE_MAGIC
            || u32_at(header, 48)? != CHUNK_LEN as u32
            || header[52..].iter().any(|&b| b != 0)
            || metadata.len() != META_LEN + TAG_LEN
        {
            return Err(Error::Format);
        }
        let mut h = [0u8; 64];
        h.copy_from_slice(header);
        let chunks = u64_at(header, 40)?;
        if chunks > u32::MAX as u64 {
            return Err(Error::Bounds);
        }
        let key = derive(master, &header[8..24])?;
        let mut f = Self {
            header: h,
            key,
            name: String::new(),
            length: 0,
            chunks,
            next_seal: None,
        };
        let m = f.decrypt(0, metadata)?;
        let length = u64_at(&m, 0)?;
        let n = u16::from_le_bytes(m[8..10].try_into().map_err(|_| Error::Format)?) as usize;
        if n > 240 || 10 + n > m.len() || length.div_ceil(CHUNK_LEN as u64) != chunks {
            return Err(Error::Format);
        }
        let name = core::str::from_utf8(&m[10..10 + n]).map_err(|_| Error::Name)?;
        if !valid_name(name) || m[10 + n..].iter().any(|&b| b != 0) {
            return Err(Error::Name);
        }
        f.name = String::from(name);
        f.length = length;
        Ok(f)
    }
    pub fn header(&self) -> &[u8; 64] {
        &self.header
    }
    pub fn encoded_length(&self) -> Result<u64> {
        self.chunks
            .checked_mul((CHUNK_LEN + TAG_LEN) as u64)
            .and_then(|n| n.checked_add((64 + META_LEN + TAG_LEN) as u64))
            .ok_or(Error::Bounds)
    }
    pub fn chunk_offset(&self, index: u64) -> Result<u64> {
        if index == 0 || index > self.chunks {
            return Err(Error::Bounds);
        }
        Ok((64 + META_LEN + TAG_LEN) as u64 + (index - 1) * (CHUNK_LEN + TAG_LEN) as u64)
    }
    fn encrypt(&self, index: u64, plain: &[u8]) -> Result<Vec<u8>> {
        let c = XChaCha20Poly1305::new_from_slice(&*self.key).map_err(|_| Error::Format)?;
        c.encrypt(
            XNonce::from_slice(&nonce(&self.header, index)),
            Payload {
                msg: plain,
                aad: &aad(&self.header, index),
            },
        )
        .map_err(|_| Error::Authentication)
    }
    fn decrypt(&self, index: u64, encrypted: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let c = XChaCha20Poly1305::new_from_slice(&*self.key).map_err(|_| Error::Format)?;
        c.decrypt(
            XNonce::from_slice(&nonce(&self.header, index)),
            Payload {
                msg: encrypted,
                aad: &aad(&self.header, index),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Authentication)
    }
    pub fn seal_chunk(&mut self, index: u64, data: &[u8]) -> Result<Vec<u8>> {
        if self.next_seal != Some(index) {
            return Err(Error::Policy);
        }
        self.chunk_offset(index)?;
        let expected = core::cmp::min(
            CHUNK_LEN as u64,
            self.length - (index - 1) * CHUNK_LEN as u64,
        ) as usize;
        if data.len() != expected {
            return Err(Error::Bounds);
        }
        let mut padded = Zeroizing::new(vec![0u8; CHUNK_LEN]);
        padded[..data.len()].copy_from_slice(data);
        let result = self.encrypt(index, &padded)?;
        self.next_seal = index.checked_add(1);
        Ok(result)
    }
    pub fn open_chunk(&self, index: u64, data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        self.chunk_offset(index)?;
        if data.len() != CHUNK_LEN + TAG_LEN {
            return Err(Error::Format);
        }
        let mut out = self.decrypt(index, data)?;
        let len = core::cmp::min(
            CHUNK_LEN as u64,
            self.length - (index - 1) * CHUNK_LEN as u64,
        ) as usize;
        if out[len..].iter().any(|&b| b != 0) {
            return Err(Error::Format);
        }
        out[len..].zeroize();
        out.truncate(len);
        Ok(out)
    }
}

pub fn seal(master: &MasterKey, entropy: &[u8; 32], name: &str, data: &[u8]) -> Result<Vec<u8>> {
    let (mut f, meta) = FileCipher::create(master, entropy, name, data.len() as u64)?;
    let mut out = Vec::new();
    out.extend_from_slice(f.header());
    out.extend_from_slice(&meta);
    for (i, c) in data.chunks(CHUNK_LEN).enumerate() {
        out.extend_from_slice(&f.seal_chunk(i as u64 + 1, c)?);
    }
    Ok(out)
}
pub fn unseal(
    master: &MasterKey,
    data: &[u8],
    max_bytes: usize,
) -> Result<(String, Zeroizing<Vec<u8>>)> {
    if data.len() < 64 + META_LEN + TAG_LEN {
        return Err(Error::Format);
    }
    let f = FileCipher::open(master, &data[..64], &data[64..64 + META_LEN + TAG_LEN])?;
    if f.encoded_length()? != data.len() as u64 || f.length > max_bytes as u64 {
        return Err(Error::Bounds);
    }
    let mut out = Zeroizing::new(Vec::with_capacity(f.length as usize));
    for i in 1..=f.chunks {
        let p = f.chunk_offset(i)? as usize;
        out.extend_from_slice(&f.open_chunk(i, &data[p..p + CHUNK_LEN + TAG_LEN])?);
    }
    Ok((f.name.clone(), out))
}

/// Shared device policy. A permission never grants raw access to a viewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Denied,
    ImportReadOnly,
    BootVault,
    CloneDestination,
}
#[derive(Clone, Copy, Debug)]
pub struct Disk {
    pub id: u64,
    pub blocks: u64,
    pub block_size: u32,
    pub boot: bool,
    pub removable: bool,
    pub read_only: bool,
    pub access: Access,
}
impl Disk {
    pub fn grant_import(&mut self) -> Result<()> {
        if self.boot {
            return Err(Error::Policy);
        }
        self.access = Access::ImportReadOnly;
        Ok(())
    }
    pub fn revoke(&mut self) {
        self.access = if self.boot {
            Access::BootVault
        } else {
            Access::Denied
        };
    }
    pub fn can_read(&self) -> bool {
        self.access != Access::Denied
    }
    pub fn can_write(&self) -> bool {
        !self.read_only && matches!(self.access, Access::BootVault | Access::CloneDestination)
    }
}
pub fn authorize_clone(source: &Disk, target: &mut Disk) -> Result<()> {
    if !source.boot
        || source.id == target.id
        || target.boot
        || !target.removable
        || target.read_only
        || source.blocks == 0
        || target.blocks < source.blocks
        || source.block_size != target.block_size
        || !matches!(source.block_size, 512 | 4096)
    {
        return Err(Error::Policy);
    }
    target.access = Access::CloneDestination;
    Ok(())
}

/// Copy only the installed image extent, never stale data beyond its partition.
/// The initial installer deliberately supports a single 512-byte-sector FAT ESP.
pub fn portable_extent(mbr: &[u8], sector_size: u32, physical_blocks: u64) -> Result<u64> {
    if sector_size != 512 || mbr.len() < 512 || mbr[510..512] != [0x55, 0xaa] {
        return Err(Error::Format);
    }
    let mut extent = None;
    for i in 0..4 {
        let p = 446 + i * 16;
        let kind = mbr[p + 4];
        if kind == 0 {
            continue;
        }
        if extent.is_some() || kind != 0xef {
            return Err(Error::Policy);
        }
        let start = u32_at(mbr, p + 8)? as u64;
        let count = u32_at(mbr, p + 12)? as u64;
        let end = start.checked_add(count).ok_or(Error::Bounds)?;
        if start < 2048 || count < 65525 || end > physical_blocks {
            return Err(Error::Bounds);
        }
        extent = Some(end);
    }
    extent.ok_or(Error::Format)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> MasterKey {
        MasterKey::from_bytes([7; 32])
    }
    #[test]
    fn roundtrip_and_padding() {
        for n in [0, 1, 65535, 65536, 65537] {
            let d = vec![0x42; n];
            let c = seal(&key(), &[9; 32], "experiment.pdf", &d).unwrap();
            let (name, p) = unseal(&key(), &c, 100000).unwrap();
            assert_eq!(name, "experiment.pdf");
            assert_eq!(&*p, &d);
            assert!(!c.windows(14).any(|w| w == b"experiment.pdf"));
        }
    }
    #[test]
    fn wrong_key_and_tampering_fail() {
        let c = seal(&key(), &[9; 32], "a.txt", b"secret").unwrap();
        assert!(unseal(&MasterKey::from_bytes([8; 32]), &c, 100).is_err());
        for i in [8, 25, 40, 64, 100, c.len() - 1] {
            let mut m = c.clone();
            m[i] ^= 1;
            assert!(unseal(&key(), &m, 100).is_err());
        }
    }
    #[test]
    fn truncation_and_appending_fail() {
        let c = seal(&key(), &[9; 32], "a", b"hello").unwrap();
        assert!(unseal(&key(), &c[..c.len() - 1], 100).is_err());
        let mut m = c.clone();
        m.push(0);
        assert!(unseal(&key(), &m, 100).is_err());
    }
    #[test]
    fn reorder_and_cross_file_fail() {
        let d = vec![3; CHUNK_LEN * 2];
        let c = seal(&key(), &[9; 32], "a", &d).unwrap();
        let f = FileCipher::open(&key(), &c[..64], &c[64..64 + META_LEN + 16]).unwrap();
        let p = f.chunk_offset(1).unwrap() as usize;
        assert!(f.open_chunk(2, &c[p..p + CHUNK_LEN + 16]).is_err());
        let other = seal(&key(), &[8; 32], "a", &d).unwrap();
        assert!(f.open_chunk(1, &other[p..p + CHUNK_LEN + 16]).is_err());
    }
    #[test]
    fn names_and_limits() {
        for name in ["../x", "a/b", "a\\b", "..", "a:b", ""] {
            assert!(seal(&key(), &[9; 32], name, b"x").is_err());
        }
        let c = seal(&key(), &[9; 32], "a", b"secret").unwrap();
        assert!(unseal(&key(), &c, 5).is_err());
    }
    #[test]
    fn password_header() {
        let pass = b"twenty random words are better";
        let (h, _) = create_key_header(pass, &[3; 72]).unwrap();
        assert!(unlock_key_header(pass, &h).is_ok());
        assert!(unlock_key_header(b"wrong passphrase", &h).is_err());
        let mut corrupt = h;
        corrupt[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(unlock_key_header(pass, &corrupt), Err(Error::Kdf)));
        assert!(create_key_header(b"short", &[3; 72]).is_err());
    }
    fn disk(id: u64, boot: bool) -> Disk {
        Disk {
            id,
            blocks: 1000,
            block_size: 512,
            boot,
            removable: !boot,
            read_only: false,
            access: if boot {
                Access::BootVault
            } else {
                Access::Denied
            },
        }
    }
    #[test]
    fn disks_start_denied_and_revoke() {
        let mut d = disk(2, false);
        assert!(!d.can_read());
        assert!(!d.can_write());
        d.grant_import().unwrap();
        assert!(d.can_read());
        assert!(!d.can_write());
        d.revoke();
        assert!(!d.can_read());
    }
    #[test]
    fn clone_policy() {
        let src = disk(1, true);
        let mut dst = disk(2, false);
        assert!(authorize_clone(&src, &mut dst).is_ok());
        assert!(dst.can_write());
        for case in 0..5 {
            let mut d = disk(2, false);
            match case {
                0 => d.boot = true,
                1 => d.removable = false,
                2 => d.read_only = true,
                3 => d.blocks = 999,
                _ => d.id = 1,
            };
            assert!(authorize_clone(&src, &mut d).is_err());
        }
    }
    #[test]
    fn seal_never_reuses_a_chunk_nonce() {
        let (mut f, _) = FileCipher::create(&key(), &[9; 32], "a", 1).unwrap();
        assert!(f.seal_chunk(1, b"x").is_ok());
        assert!(matches!(f.seal_chunk(1, b"y"), Err(Error::Policy)));
    }
    #[test]
    fn clone_excludes_unallocated_tail() {
        let mut m = [0u8; 512];
        m[510..].copy_from_slice(&[0x55, 0xaa]);
        m[450] = 0xef;
        m[454..458].copy_from_slice(&2048u32.to_le_bytes());
        m[458..462].copy_from_slice(&100000u32.to_le_bytes());
        assert_eq!(portable_extent(&m, 512, 200000), Ok(102048));
        assert!(portable_extent(&m, 512, 100000).is_err());
        m[466] = 7;
        assert!(portable_extent(&m, 512, 200000).is_err());
    }
}

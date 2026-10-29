//! Host-side, streaming file preparation. No physical-drive writes.
extern crate alloc;
use nullgrav_vault::{self as vault, CHUNK_LEN, FileCipher, META_LEN, MasterKey, TAG_LEN};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[cfg(test)]
#[path = "../../field-boot/src/media.rs"]
mod native_media;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../field-boot/src/signals.rs"]
mod native_signals;
fn crypto(e: vault::Error) -> Box<dyn std::error::Error> {
    format!("vault: {e:?}").into()
}
pub fn random<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0; N];
    getrandom::getrandom(&mut out).map_err(|e| format!("secure RNG: {e}"))?;
    Ok(out)
}
pub fn init(root: &Path, password: &[u8]) -> Result<MasterKey> {
    fs::create_dir_all(root)?;
    let mut entropy = Zeroizing::new(random::<72>()?);
    let (header, key) = vault::create_key_header(password, &entropy).map_err(crypto)?;
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("VAULT.NGK"))?;
    f.write_all(&header)?;
    f.sync_all()?;
    entropy.fill(0);
    Ok(key)
}
pub fn unlock(root: &Path, password: &[u8]) -> Result<MasterKey> {
    let mut f = File::open(root.join("VAULT.NGK"))?;
    if f.metadata()?.len() != 128 {
        return Err("invalid vault header size".into());
    }
    let mut h = [0; 128];
    f.read_exact(&mut h)?;
    vault::unlock_key_header(password, &h).map_err(crypto)
}
pub fn add(root: &Path, key: &MasterKey, input: &Path) -> Result<String> {
    let name = input
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid filename")?;
    let mut src = File::open(input)?;
    let len = src.metadata()?.len();
    let entropy = random::<32>()?;
    let (mut cipher, metadata) = FileCipher::create(key, &entropy, name, len).map_err(crypto)?;
    let id = entropy[..16]
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect::<String>();
    let final_path = root.join(format!("{id}.NGV"));
    let pending = root.join(format!("{id}.PART"));
    let result = (|| -> Result<()> {
        let mut dst = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&pending)?;
        dst.write_all(cipher.header())?;
        dst.write_all(&metadata)?;
        let mut buf = Zeroizing::new(vec![0; CHUNK_LEN]);
        for i in 1..=cipher.chunks {
            let n = (len - (i - 1) * CHUNK_LEN as u64).min(CHUNK_LEN as u64) as usize;
            src.read_exact(&mut buf[..n])?;
            dst.write_all(&cipher.seal_chunk(i, &buf[..n]).map_err(crypto)?)?;
        }
        let mut extra = [0; 1];
        if src.read(&mut extra)? != 0 {
            return Err("source grew during import".into());
        }
        dst.sync_all()?;
        drop(dst);
        verify(&pending, key)?;
        fs::rename(&pending, &final_path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&pending);
    }
    result?;
    Ok(id)
}
pub fn inspect(path: &Path, key: &MasterKey) -> Result<(File, FileCipher)> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut h = [0; 64];
    let mut m = vec![0; META_LEN + TAG_LEN];
    f.read_exact(&mut h)?;
    f.read_exact(&mut m)?;
    let c = FileCipher::open(key, &h, &m).map_err(crypto)?;
    if len != c.encoded_length().map_err(crypto)? {
        return Err("truncated or extended encrypted file".into());
    }
    Ok((f, c))
}
pub fn verify(path: &Path, key: &MasterKey) -> Result<(String, u64)> {
    let (mut f, c) = inspect(path, key)?;
    let mut buf = vec![0; CHUNK_LEN + TAG_LEN];
    for i in 1..=c.chunks {
        f.read_exact(&mut buf)?;
        let _ = c.open_chunk(i, &buf).map_err(crypto)?;
    }
    Ok((c.name.clone(), c.length))
}
pub fn extract(path: &Path, key: &MasterKey, destination: &Path) -> Result<()> {
    // Verify the COMPLETE file before creating any plaintext output.
    verify(path, key)?;
    let (mut src, c) = inspect(path, key)?;
    let mut dst = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let result = (|| -> Result<()> {
        let mut b = vec![0; CHUNK_LEN + TAG_LEN];
        for i in 1..=c.chunks {
            src.read_exact(&mut b)?;
            dst.write_all(&c.open_chunk(i, &b).map_err(crypto)?)?;
        }
        dst.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        drop(dst);
        let _ = fs::remove_file(destination);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_image_and_media_bounds() {
        for bytes in [
            include_bytes!("../../../tests/fixtures/public.png").as_slice(),
            include_bytes!("../../../tests/fixtures/public.jpg").as_slice(),
            include_bytes!("../../../tests/fixtures/public.bmp").as_slice(),
        ] {
            let image = native_media::decode(bytes).unwrap();
            assert_eq!((image.w, image.h), (20, 10));
            assert_eq!(image.rgb.len(), 20 * 10 * 3);
            for n in [0, 1, 8, 15, 25] {
                assert!(
                    std::panic::catch_unwind(|| native_media::decode(&bytes[..n.min(bytes.len())]))
                        .is_ok()
                );
            }
        }
        let jpeg = include_bytes!("../../../tests/fixtures/public.jpg");
        let mut video = b"NGMJPG01".to_vec();
        video.extend_from_slice(&5u32.to_le_bytes());
        video.extend_from_slice(&1u32.to_le_bytes());
        video.extend_from_slice(&(jpeg.len() as u32).to_le_bytes());
        video.extend_from_slice(jpeg);
        let movie = native_media::movie(&video).unwrap();
        assert_eq!(movie.fps, 5);
        assert_eq!(movie.frames.len(), 1);
        assert!(!movie.document);
        video[..8].copy_from_slice(b"NGDOC001");
        assert!(native_media::movie(&video).unwrap().document);
        video.push(0);
        assert!(native_media::movie(&video).is_err());
        video.truncate(19);
        assert!(native_media::movie(&video).is_err());
    }
    #[test]
    fn streaming_import_and_verify() {
        let p = std::env::temp_dir().join(format!("nullgrav-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        let key = init(&p, b"this test passphrase is public").unwrap();
        let src = p.join("example.txt");
        fs::write(&src, vec![0x51; CHUNK_LEN + 19]).unwrap();
        let id = add(&p, &key, &src).unwrap();
        let enc = p.join(format!("{id}.NGV"));
        assert_eq!(verify(&enc, &key).unwrap().1, CHUNK_LEN as u64 + 19);
        let out = p.join("result.txt");
        extract(&enc, &key, &out).unwrap();
        assert_eq!(fs::read(&src).unwrap(), fs::read(out).unwrap());
        assert!(init(&p, b"this test passphrase is public").is_err());
        let mut bytes = fs::read(&enc).unwrap();
        let n = bytes.len();
        bytes[n - 1] ^= 1;
        fs::write(&enc, bytes).unwrap();
        assert!(verify(&enc, &key).is_err());
        assert!(extract(&enc, &key, &p.join("rejected.txt")).is_err());
        assert!(!p.join("rejected.txt").exists());
        fs::remove_dir_all(p).unwrap();
    }
}

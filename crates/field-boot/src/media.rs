//! Bounded, native image decoding and a deliberately limited MJPEG field format.
use alloc::{format, string::String, vec, vec::Vec};
use zeroize::Zeroizing;
use zune_core::{colorspace::ColorSpace, options::DecoderOptions};
pub struct Image {
    pub rgb: Zeroizing<Vec<u8>>,
    pub w: usize,
    pub h: usize,
}
fn options() -> DecoderOptions {
    DecoderOptions::default()
        .set_max_width(4096)
        .set_max_height(4096)
        .jpeg_set_out_colorspace(ColorSpace::RGB)
        .png_set_strip_to_8bit(true)
}
pub fn decode(data: &[u8]) -> Result<Image, String> {
    if data.starts_with(&[255, 216]) {
        let mut d = zune_jpeg::JpegDecoder::new_with_options(data, options());
        let rgb = d.decode().map_err(|e| format!("JPEG: {e:?}"))?;
        let i = d.info().ok_or("JPEG dimensions missing")?;
        let w = i.width as usize;
        let h = i.height as usize;
        if rgb.len() != w * h * 3 {
            return Err(String::from("JPEG colorspace unsupported"));
        }
        return Ok(Image {
            rgb: Zeroizing::new(rgb),
            w,
            h,
        });
    }
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut d = zune_png::PngDecoder::new_with_options(data, options());
        let raw = Zeroizing::new(d.decode_raw().map_err(|e| format!("PNG: {e:?}"))?);
        let (w, h) = d.get_dimensions().ok_or("PNG dimensions missing")?;
        let cs = d.get_colorspace().ok_or("PNG colorspace missing")?;
        let components = cs.num_components();
        if raw.len() != w * h * components {
            return Err(String::from("PNG dimensions mismatch"));
        }
        let mut rgb = Zeroizing::new(Vec::with_capacity(w * h * 3));
        for p in raw.chunks_exact(components) {
            match cs {
                ColorSpace::RGB => rgb.extend_from_slice(p),
                ColorSpace::RGBA => {
                    let a = p[3] as u32;
                    for &v in &p[..3] {
                        rgb.push(((v as u32 * a + 11 * (255 - a)) / 255) as u8);
                    }
                }
                ColorSpace::Luma | ColorSpace::LumaA => rgb.extend_from_slice(&[p[0]; 3]),
                _ => return Err(String::from("PNG colorspace unsupported")),
            }
        }
        return Ok(Image { rgb, w, h });
    }
    if data.starts_with(b"BM") {
        fn u32at(d: &[u8], p: usize) -> Result<u32, String> {
            Ok(u32::from_le_bytes(
                d.get(p..p + 4)
                    .ok_or("Truncated BMP")?
                    .try_into()
                    .map_err(|_| "BMP")?,
            ))
        }
        if data.len() < 54 {
            return Err("Truncated BMP".into());
        }
        let off = u32at(data, 10)? as usize;
        let w = u32at(data, 18)? as i32;
        let sh = u32at(data, 22)? as i32;
        let h = sh.unsigned_abs() as usize;
        let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;
        if w <= 0
            || w > 4096
            || h == 0
            || h > 4096
            || !matches!(bpp, 24 | 32)
            || u32at(data, 30)? != 0
        {
            return Err("Only uncompressed 24/32-bit BMP up to 4096px is supported".into());
        }
        let w = w as usize;
        let stride = (w * bpp).div_ceil(32) * 4;
        if off.checked_add(h * stride).is_none_or(|n| n > data.len()) {
            return Err("Truncated BMP".into());
        }
        let mut rgb = Zeroizing::new(vec![0; w * h * 3]);
        for y in 0..h {
            let sy = if sh > 0 { h - 1 - y } else { y };
            for x in 0..w {
                let p = off + sy * stride + x * bpp / 8;
                let q = (y * w + x) * 3;
                rgb[q..q + 3].copy_from_slice(&[data[p + 2], data[p + 1], data[p]]);
            }
        }
        return Ok(Image { rgb, w, h });
    }
    Err("Image format unsupported: this build decodes PNG, JPEG and BMP".into())
}
pub struct Movie {
    pub frames: Vec<(usize, usize)>,
    pub fps: u32,
    pub document: bool,
}
pub fn movie(data: &[u8]) -> Result<Movie, String> {
    if data.len() < 20 || (&data[..8] != b"NGMJPG01" && &data[..8] != b"NGDOC001") {
        return Err("Not a NULLGRAV field video".into());
    }
    let fps = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let document = &data[..8] == b"NGDOC001";
    let n = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    if !(1..=30).contains(&fps) || n == 0 || n > 10000 {
        return Err("Video exceeds playback bounds".into());
    }
    let mut offset = 16;
    let mut frames = Vec::with_capacity(n);
    for _ in 0..n {
        let size = u32::from_le_bytes(
            data.get(offset..offset + 4)
                .ok_or("Truncated video")?
                .try_into()
                .unwrap(),
        ) as usize;
        offset += 4;
        if size == 0
            || size > 2 * 1024 * 1024
            || offset.checked_add(size).is_none_or(|n| n > data.len())
        {
            return Err("Invalid video frame".into());
        }
        frames.push((offset, size));
        offset += size;
    }
    if offset != data.len() {
        return Err("Unexpected video trailer".into());
    }
    Ok(Movie {
        frames,
        fps,
        document,
    })
}

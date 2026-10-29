//! Small CPU framebuffer compositor, shared with the headless preview renderer.
use alloc::{format, string::String, vec, vec::Vec};
pub const W: usize = 1280;
pub const H: usize = 720;
pub const BG: u32 = 0x0b0e0d;
pub const PANEL: u32 = 0x111614;
pub const BORDER: u32 = 0x29312a;
pub const WHITE: u32 = 0xe4e8de;
pub const MUTED: u32 = 0x819080;
pub const LIME: u32 = 0xc4f36b;
pub const AMBER: u32 = 0xdda45a;
pub const RED: u32 = 0xe37766;
pub struct Font {
    size: usize,
    cw: usize,
    ch: usize,
    data: Vec<u8>,
}
impl Font {
    fn load(b: &[u8]) -> Self {
        let data = miniz_oxide::inflate::decompress_to_vec_zlib(b).expect("embedded font");
        let size = u16::from_le_bytes([data[0], data[1]]) as usize;
        let cw = u16::from_le_bytes([data[2], data[3]]) as usize;
        let ch = u16::from_le_bytes([data[4], data[5]]) as usize;
        Self { size, cw, ch, data }
    }
}
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
pub struct Ui {
    pub pixels: Vec<u32>,
    wallpaper: Vec<u8>,
    fonts: Vec<Font>,
}
impl Ui {
    pub fn new() -> Self {
        Self {
            pixels: vec![BG; W * H],
            wallpaper: miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
                "../../../assets/wallpaper.rgbz"
            ))
            .expect("embedded art"),
            fonts: vec![
                Font::load(include_bytes!("../../../assets/font-12.bin")),
                Font::load(include_bytes!("../../../assets/font-14.bin")),
                Font::load(include_bytes!("../../../assets/font-16.bin")),
                Font::load(include_bytes!("../../../assets/font-20.bin")),
                Font::load(include_bytes!("../../../assets/font-28.bin")),
                Font::load(include_bytes!("../../../assets/font-40.bin")),
                Font::load(include_bytes!("../../../assets/font-64.bin")),
            ],
        }
    }
    pub fn clear(&mut self, c: u32) {
        self.pixels.fill(c);
    }
    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: u32) {
        for yy in y.min(H)..(y + h).min(H) {
            self.pixels[yy * W + x.min(W)..yy * W + (x + w).min(W)].fill(c);
        }
    }
    pub fn border(&mut self, r: Rect, c: u32) {
        self.rect(r.x, r.y, r.w, 1, c);
        self.rect(r.x, r.y + r.h - 1, r.w, 1, c);
        self.rect(r.x, r.y, 1, r.h, c);
        self.rect(r.x + r.w - 1, r.y, 1, r.h, c);
    }
    pub fn line(&mut self, mut x: i32, mut y: i32, x1: i32, y1: i32, c: u32) {
        let dx = (x1 - x).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let dy = -(y1 - y).abs();
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
                self.pixels[y as usize * W + x as usize] = c;
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    pub fn text(&mut self, x: usize, y: usize, size: usize, text: &str, color: u32) {
        let f = &self.fonts[self.fonts.iter().position(|f| f.size == size).unwrap_or(1)];
        let mut xx = x;
        for c in text.chars() {
            let c = if c.is_ascii() && (' '..='~').contains(&c) {
                c as usize
            } else {
                b'?' as usize
            };
            let index = c - 32;
            let start = 101 + index * f.cw * f.ch;
            for yy in 0..f.ch {
                if y + yy >= H {
                    break;
                }
                for gx in 0..f.cw {
                    if xx + gx >= W {
                        break;
                    }
                    let a = f.data[start + yy * f.cw + gx] as u32;
                    if a != 0 {
                        let p = &mut self.pixels[(y + yy) * W + xx + gx];
                        let blend = |shift: u32| {
                            (((*p >> shift) & 255u32) * (255 - a) + ((color >> shift) & 255u32) * a)
                                / 255
                        };
                        *p = (blend(16) << 16) | (blend(8) << 8) | blend(0);
                    }
                }
            }
            xx += f.data[6 + index] as usize;
        }
    }
    pub fn text_width(&self, size: usize, s: &str) -> usize {
        let f = &self.fonts[self.fonts.iter().position(|f| f.size == size).unwrap_or(1)];
        s.chars()
            .map(|c| {
                f.data[6 + if c.is_ascii() && (' '..='~').contains(&c) {
                    c as usize - 32
                } else {
                    31
                }] as usize
            })
            .sum()
    }
    pub fn wrap(
        &mut self,
        x: usize,
        mut y: usize,
        width: usize,
        text: &str,
        color: u32,
        max_lines: usize,
    ) {
        let mut line = String::new();
        let mut count = 0;
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                String::from(word)
            } else {
                format!("{line} {word}")
            };
            if self.text_width(14, &candidate) > width && !line.is_empty() {
                self.text(x, y, 14, &line, color);
                line.clear();
                y += 24;
                count += 1;
                if count >= max_lines {
                    return;
                }
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        self.text(x, y, 14, &line, color);
    }
    pub fn art(&mut self, dim: u32) {
        for (i, rgb) in self.wallpaper.chunks_exact(3).enumerate() {
            self.pixels[i] = ((rgb[0] as u32 * dim / 255) << 16)
                | ((rgb[1] as u32 * dim / 255) << 8)
                | (rgb[2] as u32 * dim / 255);
        }
    }
    pub fn emblem(&mut self, x: usize, y: usize, scale: usize, c: u32) {
        let x = x as i32;
        let y = y as i32;
        let s = scale as i32;
        for o in 0..s {
            self.line(x + o, y + 9 * s, x + 9 * s + o, y, c);
            self.line(x + 9 * s + o, y, x + 18 * s + o, y + 9 * s, c);
            self.line(x + 4 * s, y + 13 * s + o, x + 14 * s, y + 13 * s + o, c);
            self.line(x + 8 * s, y + 17 * s + o, x + 10 * s, y + 17 * s + o, c);
        }
    }
    pub fn button(&mut self, r: Rect, label: &str, active: bool) {
        self.rect(r.x, r.y, r.w, r.h, if active { LIME } else { PANEL });
        self.border(r, if active { LIME } else { BORDER });
        self.text(
            r.x + 18,
            r.y + 9,
            14,
            label,
            if active { BG } else { WHITE },
        );
    }
    pub fn cursor(&mut self, x: usize, y: usize) {
        for yy in 0..16 {
            for xx in 0..=yy / 2 {
                if x + xx < W && y + yy < H {
                    self.pixels[(y + yy) * W + x + xx] = WHITE;
                }
            }
        }
    }
    pub fn splash(&mut self, stage: usize) {
        self.art(190);
        self.rect(0, 0, W, 7, LIME);
        self.emblem(74, 64, 3, LIME);
        self.text(148, 65, 40, "NULLGRAV", WHITE);
        self.text(150, 111, 12, "INDEPENDENT FIELD SYSTEMS", MUTED);
        self.text(74, 259, 64, "BEYOND THE", WHITE);
        self.text(74, 329, 64, "KNOWN FIELD.", WHITE);
        self.text(78, 433, 14, "PRIVATE RESEARCH / PORTABLE BY DESIGN", MUTED);
        self.rect(78, 525, 430, 2, BORDER);
        self.rect(78, 525, 430 * stage / 4, 2, LIME);
        let stages = [
            "ESTABLISHING FRAMEBUFFER",
            "IDENTIFYING BOOT MEDIUM",
            "APPLYING DISK ACCESS POLICY",
            "INITIALIZING FIELD CONSOLE",
            "FIELD CONSOLE READY",
        ];
        self.text(78, 545, 12, stages[stage.min(4)], LIME);
        self.text(
            78,
            652,
            12,
            "NATIVE X86-64 UEFI  /  ENGINEERING PREVIEW 0.2",
            MUTED,
        );
        self.text(970, 652, 12, "NO NETWORK STACK", MUTED);
    }
    pub fn shell(&mut self, page: usize, unlocked: bool) {
        self.clear(BG);
        self.rect(0, 0, 244, H, 0x0e120f);
        self.rect(243, 0, 1, H, BORDER);
        self.emblem(28, 33, 2, LIME);
        self.text(78, 28, 28, "NULLGRAV", WHITE);
        self.text(79, 64, 12, "FIELD SYSTEMS / 01", MUTED);
        self.rect(24, 110, 195, 1, BORDER);
        self.text(28, 135, 12, "OPERATIONS", MUTED);
        for (i, (n, label)) in [
            ("01", "OVERVIEW"),
            ("02", "VAULT"),
            ("03", "DISK GATE"),
            ("04", "REPLICATE"),
            ("05", "SYSTEM BRIEF"),
        ]
        .iter()
        .enumerate()
        {
            let y = 174 + i * 62;
            if i == page {
                self.rect(16, y - 5, 212, 47, 0x222c1c);
                self.rect(16, y - 5, 3, 47, LIME);
            }
            self.text(30, y + 7, 12, n, if i == page { LIME } else { MUTED });
            self.text(66, y + 4, 16, label, if i == page { WHITE } else { MUTED });
        }
        self.rect(24, 554, 195, 1, BORDER);
        self.text(28, 578, 12, "NETWORK", MUTED);
        self.text(126, 578, 12, "ABSENT", LIME);
        self.text(28, 606, 12, "VAULT", MUTED);
        self.text(
            126,
            606,
            12,
            if unlocked { "UNLOCKED" } else { "SEALED" },
            if unlocked { LIME } else { AMBER },
        );
        self.text(28, 656, 12, "[L] LOCK  [Q] POWER", MUTED);
        self.text(277, 28, 12, "NULLGRAV / RESEARCH TRANSPORT", MUTED);
        self.text(1010, 28, 12, "BUILD 0.2.0 / ALPHA", AMBER);
        self.rect(277, 66, 970, 1, BORDER);
        self.rect(277, 676, 970, 1, BORDER);
        self.text(
            278,
            689,
            12,
            "FIRMWARE-BACKED PREVIEW / NO ISOLATED USERSPACE YET",
            AMBER,
        );
    }
    pub fn overview(&mut self, unlocked: bool, files: usize, disks: usize) {
        self.shell(0, unlocked);
        // Preserve the live layout while reusing the generated hardware photograph.
        for y in 88..465 {
            for x in 715..1246 {
                let i = y * W + x;
                let j = (y * W + x) * 3;
                self.pixels[i] = ((self.wallpaper[j] as u32 * 165 / 255) << 16)
                    | ((self.wallpaper[j + 1] as u32 * 165 / 255) << 8)
                    | (self.wallpaper[j + 2] as u32 * 165 / 255);
            }
        }
        self.text(279, 97, 12, "OFFLINE / INDEPENDENT / OPEN SOURCE", LIME);
        self.text(277, 139, 64, "CONTROL", WHITE);
        self.text(277, 207, 64, "THE FIELD.", WHITE);
        self.wrap(282,304,410,"A portable workspace for extraordinary research. Choose what enters. Decide what leaves.",MUTED,3);
        self.button(
            Rect {
                x: 282,
                y: 393,
                w: 224,
                h: 46,
            },
            if unlocked {
                "[2] OPEN VAULT"
            } else {
                "[U] UNLOCK VAULT"
            },
            true,
        );
        self.text(526, 408, 12, "[N] CREATE VAULT", MUTED);
        for (i, (title, value, sub)) in [
            (
                "ENCRYPTED OBJECTS",
                format!("{files:02}"),
                "NAMES SEALED AT REST",
            ),
            (
                "DEVICE POLICY",
                format!("{disks:02}"),
                "EXTERNAL VOLUMES BLOCKED",
            ),
            (
                "NETWORK INTERFACES",
                String::from("00"),
                "NO NETWORK CODE INCLUDED",
            ),
        ]
        .iter()
        .enumerate()
        {
            let x = 282 + i * 324;
            self.rect(x, 487, 305, 145, PANEL);
            self.border(
                Rect {
                    x,
                    y: 487,
                    w: 305,
                    h: 145,
                },
                BORDER,
            );
            self.text(x + 18, 502, 12, title, MUTED);
            self.text(x + 18, 525, 40, value, WHITE);
            self.text(x + 18, 602, 12, sub, if i == 1 { AMBER } else { MUTED });
        }
    }
    pub fn heading(&mut self, kicker: &str, title: &str, subtitle: &str) {
        self.text(282, 95, 12, kicker, LIME);
        self.text(280, 123, 40, title, WHITE);
        self.text(283, 187, 14, subtitle, MUTED);
    }
    pub fn modal(&mut self, title: &str, description: &str, input: &str, secret: bool) {
        for p in &mut self.pixels {
            *p = (((*p >> 16 & 255u32) / 3) << 16)
                | (((*p >> 8 & 255u32) / 3) << 8)
                | ((*p & 255u32) / 3);
        }
        self.rect(326, 197, 680, 307, 0x151b15);
        self.border(
            Rect {
                x: 326,
                y: 197,
                w: 680,
                h: 307,
            },
            BORDER,
        );
        self.rect(326, 197, 680, 3, LIME);
        self.text(352, 219, 28, title, WHITE);
        self.wrap(352, 270, 620, description, MUTED, 3);
        self.rect(352, 350, 628, 49, BG);
        self.border(
            Rect {
                x: 352,
                y: 350,
                w: 628,
                h: 49,
            },
            BORDER,
        );
        let shown = if secret {
            "*".repeat(input.chars().count().min(58))
        } else {
            input.chars().take(58).collect()
        };
        self.text(366, 362, 16, &shown, WHITE);
        self.text(
            352,
            438,
            12,
            "[ENTER] CONTINUE                         [ESC] CANCEL",
            LIME,
        );
    }
    pub fn notice(&mut self, text: &str, bad: bool) {
        self.rect(282, 632, 960, 32, if bad { 0x352019 } else { 0x202a19 });
        self.text(
            294,
            638,
            12,
            &text.chars().take(126).collect::<String>(),
            if bad { RED } else { LIME },
        );
    }
    pub fn progress(&mut self, title: &str, n: u64, total: u64) {
        self.shell(3, false);
        self.heading(
            "REPLICATION / VERIFIED COPY",
            title,
            "Destination writes are explicit. Boot source and internal disks are protected.",
        );
        self.rect(282, 285, 960, 5, BORDER);
        self.rect(282, 285, (960 * n / total.max(1)) as usize, 5, LIME);
        self.text(282, 318, 28, &format!("{}%", n * 100 / total.max(1)), WHITE);
        self.text(
            282,
            375,
            14,
            "KEEP BOTH DEVICES CONNECTED UNTIL VERIFICATION FINISHES.",
            AMBER,
        );
    }
    pub fn erasure(&mut self, stage: &str, n: u64, total: u64, failed: bool) {
        self.pixels.fill(BG);
        self.shell(1, false);
        self.heading(
            "THREE-FAILURE RESPONSE",
            "BOOT CARD / DATA DESTRUCTION",
            "Only the current removable boot medium is selected. This operation cannot be undone.",
        );
        self.wrap(282, 230, 950, stage, if failed { RED } else { AMBER }, 3);
        self.rect(282, 345, 960, 6, BORDER);
        self.rect(282, 345, (960 * n / total.max(1)) as usize, 6, RED);
        self.text(282, 375, 32, &format!("{}%", n * 100 / total.max(1)), WHITE);
        self.wrap(282, 455, 930, if failed {
            "The complete overwrite was NOT verified. Access stays locked. Press Q to shut down. Do not assume all data was erased."
        } else {
            "Keep the card connected until completion. Normal logical writes preserve the card for reformatting; flash remnants and earlier copies may survive."
        }, MUTED, 4);
    }
    pub fn image(&mut self, rgb: &[u8], w: usize, h: usize, bounds: Rect) {
        let Rect { x, y, w: bw, h: bh } = bounds;
        if w == 0 || h == 0 || rgb.len() < w.saturating_mul(h).saturating_mul(3) {
            return;
        }
        let (dwidth, dheight) = if w * bh > h * bw {
            (bw, h * bw / w)
        } else {
            (w * bh / h, bh)
        };
        let ox = x + (bw - dwidth) / 2;
        let oy = y + (bh - dheight) / 2;
        for yy in 0..dheight {
            for xx in 0..dwidth {
                let i = ((yy * h / dheight) * w + xx * w / dwidth) * 3;
                if ox + xx < W && oy + yy < H {
                    self.pixels[(oy + yy) * W + ox + xx] =
                        ((rgb[i] as u32) << 16) | ((rgb[i + 1] as u32) << 8) | rgb[i + 2] as u32;
                }
            }
        }
    }
}

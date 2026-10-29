//! Procedural effects are clipped to artwork and decoration, never prompts/media.
use super::*;
const CYAN: u32 = 0x68dccc;
const DIM: u32 = 0x243e32;
const ORBIT: [(i32, i32); 32] = [
    (1000, 0),
    (981, 195),
    (924, 383),
    (831, 556),
    (707, 707),
    (556, 831),
    (383, 924),
    (195, 981),
    (0, 1000),
    (-195, 981),
    (-383, 924),
    (-556, 831),
    (-707, 707),
    (-831, 556),
    (-924, 383),
    (-981, 195),
    (-1000, 0),
    (-981, -195),
    (-924, -383),
    (-831, -556),
    (-707, -707),
    (-556, -831),
    (-383, -924),
    (-195, -981),
    (0, -1000),
    (195, -981),
    (383, -924),
    (556, -831),
    (707, -707),
    (831, -556),
    (924, -383),
    (981, -195),
];
fn orbit(phase: usize) -> (i32, i32) {
    let i = (phase / 8) % ORBIT.len();
    let t = (phase % 8) as i32;
    let a = ORBIT[i];
    let b = ORBIT[(i + 1) % ORBIT.len()];
    (a.0 + (b.0 - a.0) * t / 8, a.1 + (b.1 - a.1) * t / 8)
}
impl Ui {
    fn tint(&mut self, r: Rect, color: u32, alpha: u32) {
        for y in r.y..(r.y + r.h).min(H) {
            for x in r.x..(r.x + r.w).min(W) {
                let p = &mut self.pixels[y * W + x];
                let channel = |n: u32| {
                    (((*p >> n) & 255u32) * (255 - alpha) + ((color >> n) & 255u32) * alpha) / 255
                };
                *p = (channel(16) << 16) | (channel(8) << 8) | channel(0);
            }
        }
    }
    fn circle(&mut self, cx: usize, cy: usize, radius: usize, color: u32) {
        for i in 0..64 {
            let a = orbit(i * 4);
            let b = orbit((i + 1) * 4);
            self.line(
                cx as i32 + a.0 * radius as i32 / 1000,
                cy as i32 + a.1 * radius as i32 / 1000,
                cx as i32 + b.0 * radius as i32 / 1000,
                cy as i32 + b.1 * radius as i32 / 1000,
                color,
            );
        }
    }
    fn rain(&mut self, r: Rect, columns: usize, dense: bool) {
        let f = self.signal.frame as usize;
        for col in 0..columns {
            let x = r.x + col * r.w / columns;
            let head = (f * (4 + col % 5) + col * 71) % r.h;
            for j in 0..if dense { 12 } else { 6 } {
                let y = (head + r.h - (j * 19) % r.h) % r.h;
                if y + 16 >= r.h {
                    continue;
                }
                let n = self.signal.word(col * 17 + j);
                let s = if dense {
                    format!("{:04X}", n & 65535)
                } else {
                    format!("{:02X}", n & 255)
                };
                self.text(
                    x,
                    r.y + y,
                    12,
                    &s,
                    if j == 0 {
                        CYAN
                    } else if j < 3 {
                        0x4b7155
                    } else {
                        DIM
                    },
                );
            }
        }
    }
    fn radar(&mut self, cx: usize, cy: usize, radius: usize) {
        let f = self.signal.frame as usize;
        for div in 1..=4 {
            self.circle(cx, cy, radius * div / 4, DIM);
        }
        self.line(
            (cx - radius) as i32,
            cy as i32,
            (cx + radius) as i32,
            cy as i32,
            DIM,
        );
        self.line(
            cx as i32,
            (cy - radius) as i32,
            cx as i32,
            (cy + radius) as i32,
            DIM,
        );
        for i in 0..32 {
            let a = orbit(i * 8);
            let r = if i % 4 == 0 { radius + 7 } else { radius + 3 };
            self.line(
                cx as i32 + a.0 * radius as i32 / 1000,
                cy as i32 + a.1 * radius as i32 / 1000,
                cx as i32 + a.0 * r as i32 / 1000,
                cy as i32 + a.1 * r as i32 / 1000,
                0x4b7155,
            );
        }
        for trail in (0..24).rev() {
            let a = orbit(f * 3 + 256 - trail);
            self.line(
                cx as i32,
                cy as i32,
                cx as i32 + a.0 * radius as i32 / 1000,
                cy as i32 + a.1 * radius as i32 / 1000,
                if trail < 2 { LIME } else { 0x1d3525 },
            );
        }
        for i in 0..21 {
            let a = orbit(i * 47 + f * (1 + i % 3));
            let r = (i * 37 + f / 3) % (radius - 16) + 12;
            let x = (cx as i32 + a.0 * r as i32 / 1000) as usize;
            let y = (cy as i32 + a.1 * r as i32 / 1000) as usize;
            let c = if i % 4 == 0 { CYAN } else { LIME };
            self.rect(x.saturating_sub(3), y, 7, 1, c);
            self.rect(x, y.saturating_sub(3), 1, 7, c);
            if i % 5 == 0 {
                self.text(
                    x + 7,
                    y + 3,
                    12,
                    &format!("{:02X}", self.signal.word(i) & 255),
                    MUTED,
                );
            }
        }
        self.circle(cx, cy, 9, LIME);
    }
    fn tear(&mut self, r: Rect, count: usize) {
        let f = self.signal.frame as usize;
        if !self.signal.motion || f % 89 > 10 {
            return;
        }
        for n in 0..count {
            let word = self.signal.word(n + 189) as usize;
            let y = r.y + word % (r.h - 12);
            let height = 2 + word % 9;
            let shift = 3 + (word / 11) % 27;
            for row in y..y + height {
                let line = &mut self.pixels[row * W + r.x..row * W + r.x + r.w];
                if n % 2 == 0 {
                    line.rotate_left(shift);
                } else {
                    line.rotate_right(shift);
                }
                for p in line.iter_mut().step_by(3) {
                    *p = (*p & 0x00ff00) | ((*p >> 8) & 0xff) | 0x220000;
                }
            }
            self.rect(r.x, y, r.w, 1, if n % 2 == 0 { CYAN } else { 0x465a39 });
        }
    }
    pub fn boot_signal(&mut self) {
        if !self.signal.motion {
            return;
        }
        self.rain(
            Rect {
                x: 695,
                y: 138,
                w: 525,
                h: 437,
            },
            12,
            false,
        );
        self.radar(955, 340, 152);
        self.text(
            734,
            590,
            12,
            "SIGNAL INITIALIZATION / DISPLAY SEQUENCE",
            CYAN,
        );
        self.text(
            734,
            613,
            12,
            &format!(
                "{:08X}  {:08X}  {:08X}",
                self.signal.word(0),
                self.signal.word(1),
                self.signal.word(2)
            ),
            MUTED,
        );
        self.tear(
            Rect {
                x: 640,
                y: 160,
                w: 607,
                h: 385,
            },
            6,
        );
    }
    pub fn ambient(&mut self, page: usize) {
        if !self.signal.motion {
            return;
        }
        let f = self.signal.frame as usize;
        self.rect(251, 90 + (f * 5) % 511, 2, 18, CYAN);
        self.rect(277 + (f * 13) % 866, 72, 78, 1, LIME);
        if page == 0 {
            let art = Rect {
                x: 715,
                y: 88,
                w: 531,
                h: 377,
            };
            self.tint(art, 0x071610, 70);
            self.rain(
                Rect {
                    x: 731,
                    y: 117,
                    w: 495,
                    h: 325,
                },
                12,
                false,
            );
            self.radar(982, 269, 131);
            self.rect(732, 101, 207, 23, BG);
            self.text(742, 106, 12, "SIGNAL / DISPLAY CHANNEL", CYAN);
            self.rect(730, 407, 500, 39, 0x0e1913);
            self.text(
                742,
                415,
                12,
                &format!(
                    "{:08X}  {:08X}  {:08X}",
                    self.signal.word(41),
                    self.signal.word(42),
                    self.signal.word(43)
                ),
                LIME,
            );
            self.tear(art, 5);
            self.text(
                282,
                462,
                12,
                "[6] SIGNAL LAB / LIVE NUMBER STREAM / [G] MOTION",
                CYAN,
            );
        }
    }
    pub fn motion_status(&mut self) {
        self.text(
            1020,
            689,
            12,
            if !self.signal.motion {
                "[G] FX OFF"
            } else if self.signal.paused {
                "[G] FX HELD"
            } else {
                "[G] FX LIVE"
            },
            CYAN,
        );
    }
    pub fn signal_lab(&mut self) {
        self.heading(
            "LOCAL ENTROPY / PUBLIC DISPLAY",
            "SIGNAL LAB / LIVE FEED",
            "[R] DRAW 256 BITS    [SPACE] HOLD STREAM    [G] MOTION ON / OFF",
        );
        let left = Rect {
            x: 282,
            y: 225,
            w: 337,
            h: 394,
        };
        self.rect(left.x, left.y, left.w, left.h, 0x0b1611);
        if self.signal.motion {
            self.rain(
                Rect {
                    x: 294,
                    y: 239,
                    w: 305,
                    h: 357,
                },
                6,
                true,
            );
        }
        self.tint(left, BG, 95);
        self.radar(450, 426, 140);
        self.tear(
            Rect {
                x: 286,
                y: 257,
                w: 329,
                h: 350,
            },
            4,
        );
        self.border(left, BORDER);
        self.rect(294, 236, 309, 30, 0x0c1811);
        self.text(305, 243, 12, "FIELD SCOPE / SIMULATED", CYAN);
        self.text(
            302,
            590,
            12,
            &format!("FRAME {:08} / DISPLAY ONLY", self.signal.frame % 100000000),
            LIME,
        );

        let sample = Rect {
            x: 638,
            y: 225,
            w: 604,
            h: 184,
        };
        self.rect(sample.x, sample.y, sample.w, sample.h, PANEL);
        self.border(sample, BORDER);
        self.rect(638, 225, 4, 184, CYAN);
        self.text(660, 242, 12, "RANDOM SAMPLE / 256 BITS", CYAN);
        if let Some(bytes) = &self.signal.sample {
            let mut rows = [String::new(), String::new()];
            for (i, b) in bytes.iter().enumerate() {
                if i % 4 == 0 && i % 16 != 0 {
                    rows[i / 16].push(' ');
                }
                rows[i / 16].push_str(&format!("{b:02X}"));
            }
            self.text(660, 275, 20, &rows[0], WHITE);
            self.text(660, 308, 20, &rows[1], WHITE);
        } else {
            self.text(660, 282, 20, "RNG UNAVAILABLE / SAMPLE CLEARED", AMBER);
        }
        self.text(
            660,
            352,
            12,
            &format!(
                "SOURCE: {}     DRAW: {:06}",
                self.signal.source,
                self.signal.generation % 1000000
            ),
            LIME,
        );
        self.text(
            660,
            380,
            12,
            "ON-SCREEN SAMPLE / SEPARATE FROM VAULT KEYS",
            MUTED,
        );
        let scope = Rect {
            x: 638,
            y: 426,
            w: 604,
            h: 193,
        };
        self.rect(scope.x, scope.y, scope.w, scope.h, 0x0b1611);
        self.border(scope, BORDER);
        self.text(
            660,
            440,
            12,
            if self.signal.paused {
                "STREAM / HELD"
            } else {
                "STREAM / DISPLAY ACTIVITY"
            },
            CYAN,
        );
        for y in (474..594).step_by(24) {
            self.rect(654, y, 572, 1, DIM);
        }
        for x in (654..1226).step_by(44) {
            self.rect(x, 474, 1, 120, DIM);
        }
        let mut last = (654, 536);
        for n in 0..143 {
            let word = self.signal.word(n + 33);
            let x = 654 + n * 4;
            let y = 491 + (word % 88) as usize;
            self.line(
                last.0 as i32,
                last.1 as i32,
                x as i32,
                y as i32,
                if n % 3 == 0 { CYAN } else { LIME },
            );
            last = (x, y);
        }
        for i in 0..36 {
            let height = (self.signal.word(i + 177) % 26) as usize;
            self.rect(
                660 + i * 15,
                606 - height,
                9,
                height,
                if i % 4 == 0 { CYAN } else { 0x41633a },
            );
        }
        let x = 654 + (self.signal.frame as usize * 11) % 570;
        self.rect(x, 474, 1, 121, CYAN);
    }
}

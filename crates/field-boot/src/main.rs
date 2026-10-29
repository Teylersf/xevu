#![no_std]
#![no_main]
extern crate alloc;
mod media;
mod storage;
mod ui;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::time::Duration;
use nullgrav_vault::MasterKey;
use storage::{Entry, Physical, Storage};
use uefi::{
    boot,
    prelude::*,
    proto::console::{
        gop::{BltOp, BltPixel, BltRegion, GraphicsOutput},
        pointer::{AbsolutePointer, Pointer},
        text::{Key, ScanCode},
    },
};
use ui::{AMBER, BG, H, LIME, MUTED, PANEL, Rect, Ui, W, WHITE};
use zeroize::{Zeroize, Zeroizing};

struct Screen {
    gop: boot::ScopedProtocol<GraphicsOutput>,
    buffer: Vec<BltPixel>,
    w: usize,
    h: usize,
}
impl Screen {
    fn new() -> uefi::Result<Self> {
        let handle = boot::find_handles::<GraphicsOutput>()?
            .first()
            .copied()
            .ok_or(uefi::Error::from(Status::NOT_FOUND))?;
        let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle)?;
        let best = gop
            .modes()
            .filter(|m| {
                let (w, h) = m.info().resolution();
                w >= 800 && h >= 600 && w <= 1920 && h <= 1200
            })
            .min_by_key(|m| {
                let (w, h) = m.info().resolution();
                w.abs_diff(W) + h.abs_diff(H)
            });
        if let Some(mode) = best {
            gop.set_mode(&mode)?;
        }
        let (w, h) = gop.current_mode_info().resolution();
        Ok(Self {
            gop,
            buffer: vec![BltPixel::new(0, 0, 0); w * h],
            w,
            h,
        })
    }
    fn show(&mut self, u: &Ui) {
        let (dw, dh) = if self.w * H > self.h * W {
            (self.h * W / H, self.h)
        } else {
            (self.w, self.w * H / W)
        };
        let ox = (self.w - dw) / 2;
        let oy = (self.h - dh) / 2;
        self.buffer.fill(BltPixel::new(0, 0, 0));
        for y in 0..dh {
            for x in 0..dw {
                let c = u.pixels[(y * H / dh) * W + x * W / dw];
                self.buffer[(oy + y) * self.w + ox + x] =
                    BltPixel::new((c >> 16) as u8, (c >> 8) as u8, c as u8);
            }
        }
        let _ = self.gop.blt(BltOp::BufferToVideo {
            buffer: &self.buffer,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (self.w, self.h),
        });
    }
    fn clear(&mut self) {
        self.buffer.fill(BltPixel::new(0, 0, 0));
        let _ = self.gop.blt(BltOp::VideoFill {
            color: BltPixel::new(0, 0, 0),
            dest: (0, 0),
            dims: (self.w, self.h),
        });
    }
}
#[derive(Clone, Copy)]
enum Prompt {
    Unlock,
    Create,
    Repeat,
    Clone(usize),
}
struct Modal {
    kind: Prompt,
    input: Zeroizing<String>,
}
struct Viewer {
    name: Zeroizing<String>,
    data: Zeroizing<Vec<u8>>,
    image: Option<media::Image>,
    movie: Option<media::Movie>,
    frame: usize,
    scroll: usize,
    paused: bool,
}
struct App {
    storage: Storage,
    key: Option<MasterKey>,
    page: usize,
    selected: usize,
    files: Vec<Entry>,
    names: Vec<(String, u64)>,
    disks: Vec<Physical>,
    browse: Option<(usize, String)>,
    entries: Vec<Entry>,
    modal: Option<Modal>,
    pending: Option<Zeroizing<String>>,
    viewer: Option<Viewer>,
    notice: String,
    bad: bool,
    dirty: bool,
    ticks: u64,
    last_input: u64,
    mouse: (usize, usize),
    mouse_down: bool,
}
impl App {
    fn new(storage: Storage) -> Self {
        let mut s = Self {
            storage,
            key: None,
            page: 0,
            selected: 0,
            files: Vec::new(),
            names: Vec::new(),
            disks: Vec::new(),
            browse: None,
            entries: Vec::new(),
            modal: None,
            pending: None,
            viewer: None,
            notice: String::new(),
            bad: false,
            dirty: true,
            ticks: 0,
            last_input: 0,
            mouse: (0, 0),
            mouse_down: false,
        };
        s.refresh();
        s
    }
    fn notify(&mut self, text: impl Into<String>, bad: bool) {
        self.notice = text.into();
        self.bad = bad;
        self.dirty = true;
    }
    fn refresh(&mut self) {
        self.files = self.storage.encrypted_files();
        self.wipe_names();
        if let Some(key) = &self.key {
            for e in &self.files {
                self.names.push(
                    self.storage
                        .metadata(&e.name, key)
                        .unwrap_or_else(|_| ("[CORRUPT / AUTHENTICATION FAILED]".to_string(), 0)),
                );
            }
        }
        self.disks = self.storage.physical_disks().unwrap_or_default();
        self.dirty = true;
    }
    fn wipe_names(&mut self) {
        for (n, _) in &mut self.names {
            n.zeroize();
        }
        self.names.clear();
    }
    fn lock(&mut self) {
        self.viewer = None;
        self.modal = None;
        self.pending = None;
        self.key = None;
        self.wipe_names();
        self.storage.revoke_all();
        self.browse = None;
        self.entries.clear();
        self.notify("VAULT SEALED / ACCESS GRANTS REVOKED", false);
    }
    fn goto(&mut self, p: usize) {
        self.storage.revoke_all();
        self.browse = None;
        self.entries.clear();
        self.page = p;
        self.selected = 0;
        self.notice.clear();
        self.dirty = true;
        if p == 2 {
            let _ = self.storage.refresh_volumes();
        }
        if p == 3 {
            self.disks = self.storage.physical_disks().unwrap_or_default();
        }
    }
    fn prompt(&mut self, kind: Prompt) {
        self.modal = Some(Modal {
            kind,
            input: Zeroizing::new(String::new()),
        });
        self.dirty = true;
    }
    fn count(&self) -> usize {
        if self.browse.is_some() {
            self.entries.len()
        } else {
            match self.page {
                1 => self.files.len(),
                2 => self.storage.volumes.len(),
                3 => self.disks.len(),
                _ => 0,
            }
        }
    }
    fn draw(&mut self, u: &mut Ui) {
        if self.page == 0 {
            u.overview(
                self.key.is_some(),
                self.files.len(),
                self.storage.volumes.iter().filter(|v| !v.boot).count(),
            );
        } else {
            u.shell(self.page, self.key.is_some());
            match self.page {
                1 => {
                    u.heading(
                        "COMPARTMENT / ENCRYPTED OBJECTS",
                        "RESEARCH VAULT",
                        if self.key.is_some() {
                            "[ENTER] VIEW   [I] IMPORT   [L] LOCK"
                        } else {
                            "[U] UNLOCK   [N] CREATE A NEW VAULT"
                        },
                    );
                    if self.key.is_none() {
                        u.text(282, 281, 28, "CONTENTS SEALED", WHITE);
                        u.wrap(282,343,780,"Filenames and file contents are encrypted. Enter the vault passphrase to authenticate the object directory.",MUTED,3);
                        u.button(
                            Rect {
                                x: 282,
                                y: 430,
                                w: 240,
                                h: 46,
                            },
                            "[U] UNLOCK VAULT",
                            true,
                        );
                    } else if self.files.is_empty() {
                        u.text(282, 282, 28, "NO OBJECTS YET", WHITE);
                        u.wrap(282,344,780,"Open Disk Gate to authorize a FAT volume, choose a file, and import an encrypted copy. The source is opened read-only.",MUTED,3);
                    } else {
                        let start = self.selected.saturating_sub(7);
                        for (i, e) in self.files.iter().enumerate().skip(start).take(9) {
                            let y = 241 + (i - start) * 40;
                            if i == self.selected {
                                u.rect(282, y, 960, 37, 0x222c1c);
                            }
                            let (n, size) = self
                                .names
                                .get(i)
                                .cloned()
                                .unwrap_or((e.name.clone(), e.size));
                            u.text(
                                296,
                                y + 7,
                                14,
                                &n.chars().take(82).collect::<String>(),
                                WHITE,
                            );
                            u.text(
                                1120,
                                y + 7,
                                12,
                                &format!("{} KiB", size.div_ceil(1024)),
                                MUTED,
                            );
                        }
                    }
                }
                2 => {
                    if let Some((_, p)) = &self.browse {
                        u.heading("DISK GATE / TEMPORARY READ GRANT","SELECT AN IMPORT","[ENTER] OPEN DIRECTORY / IMPORT FILE    [BACKSPACE] PARENT    [ESC] REVOKE");
                        u.text(284, 218, 12, &p.chars().take(112).collect::<String>(), LIME);
                        let start = self.selected.saturating_sub(7);
                        for (i, e) in self.entries.iter().enumerate().skip(start).take(9) {
                            let y = 249 + (i - start) * 39;
                            if i == self.selected {
                                u.rect(282, y, 960, 36, 0x222c1c);
                            }
                            u.text(
                                296,
                                y + 6,
                                14,
                                &format!(
                                    "{} {}",
                                    if e.directory { "[DIR]" } else { "[   ]" },
                                    e.name.chars().take(75).collect::<String>()
                                ),
                                WHITE,
                            );
                        }
                    } else {
                        u.heading(
                            "DEVICE CONTROL / EXPLICIT ACCESS",
                            "DISK GATE",
                            "[ENTER] GRANT TEMPORARY READ ACCESS    [R] RESCAN",
                        );
                        let start = self.selected.saturating_sub(5);
                        for (i, v) in self.storage.volumes.iter().enumerate().skip(start).take(6) {
                            let y = 245 + (i - start) * 56;
                            u.rect(
                                282,
                                y,
                                960,
                                48,
                                if i == self.selected { 0x222c1c } else { PANEL },
                            );
                            u.text(297, y + 12, 14, &v.name, WHITE);
                            u.text(
                                615,
                                y + 12,
                                12,
                                if v.boot {
                                    "BOOT MEDIUM / VAULT ONLY"
                                } else {
                                    "FAT VOLUME / NO FILE ACCESS"
                                },
                                MUTED,
                            );
                            u.text(
                                1090,
                                y + 12,
                                12,
                                if v.boot { "PROTECTED" } else { "BLOCKED" },
                                if v.boot { LIME } else { AMBER },
                            );
                        }
                        u.text(
                            282,
                            599,
                            12,
                            "NTFS / EXFAT / EXT FILESYSTEM DRIVERS ARE NOT INCLUDED IN THIS BUILD.",
                            AMBER,
                        );
                    }
                }
                3 => {
                    u.heading(
                        "TRANSPORT / EXACT REPLICA",
                        "REPLICATE DRIVE",
                        "[ENTER] SELECT DESTINATION    [R] RESCAN    NO AUTOMATIC WRITES",
                    );
                    let start = self.selected.saturating_sub(5);
                    for (i, d) in self.disks.iter().enumerate().skip(start).take(6) {
                        let y = 245 + (i - start) * 56;
                        u.rect(
                            282,
                            y,
                            960,
                            48,
                            if i == self.selected { 0x222c1c } else { PANEL },
                        );
                        u.text(297, y + 12, 14, &d.name, WHITE);
                        let label = if d.policy.boot {
                            "BOOT SOURCE"
                        } else if !d.policy.removable {
                            "INTERNAL / BLOCKED"
                        } else if d.policy.read_only {
                            "READ ONLY"
                        } else {
                            "REMOVABLE TARGET"
                        };
                        u.text(
                            915,
                            y + 12,
                            12,
                            label,
                            if d.policy.boot { LIME } else { AMBER },
                        );
                    }
                    u.text(
                        282,
                        599,
                        12,
                        "EXACT CLONES SHARE VAULT KEYS. DESTINATION CONTENTS WILL BE OVERWRITTEN.",
                        AMBER,
                    );
                }
                4 => {
                    u.heading(
                        "BUILD MANIFEST / HONEST CAPABILITIES",
                        "SYSTEM BRIEF",
                        "NULLGRAV 0.1 / NATIVE RUST / X86-64 UEFI",
                    );
                    let lines = [
                        "PURPOSE       OFFLINE TRANSPORT FOR ANTIGRAVITY RESEARCH",
                        "PLATFORM      UEFI FIRMWARE DRIVERS / APOLLO LAKE TARGET",
                        "CRYPTO        ARGON2ID / XCHACHA20-POLY1305 / HKDF-SHA256",
                        "MEDIA         PNG / JPEG / BMP / UTF-8 TEXT / SILENT FIELD MJPEG",
                        "DRIVES        FAT FIRMWARE VOLUMES / READ-ONLY IMPORT / VERIFIED CLONE",
                        "ISOLATION     APPLICATION POLICY ONLY / NO USERSPACE SANDBOX",
                        "STATUS        EXPERIMENTAL / NOT AUDITED / NOT CLASSIFIED APPROVED",
                        "TRUST         FIRMWARE, HARDWARE AND ALL NATIVE CODE MUST BE TRUSTED",
                        "[L] LOCK      [Q] SHUT DOWN      [R] REFRESH",
                    ];
                    for (i, l) in lines.iter().enumerate() {
                        u.text(
                            282,
                            245 + i * 35,
                            12,
                            l,
                            if i == 5 || i == 6 { AMBER } else { MUTED },
                        );
                    }
                }
                _ => {}
            }
        }
        if let Some(v) = &self.viewer {
            u.rect(264, 80, 1000, 586, BG);
            u.text(
                282,
                95,
                12,
                "AUTHENTICATED VIEW / MEMORY ONLY / ESC TO CLOSE",
                LIME,
            );
            u.text(
                282,
                123,
                28,
                &v.name.chars().take(83).collect::<String>(),
                WHITE,
            );
            if let Some(image) = &v.image {
                u.image(
                    &image.rgb,
                    image.w,
                    image.h,
                    Rect {
                        x: 282,
                        y: 184,
                        w: 960,
                        h: 420,
                    },
                );
            } else if let Some(movie) = &v.movie {
                let (p, n) = movie.frames[v.frame];
                if let Ok(img) = media::decode(&v.data[p..p + n]) {
                    u.image(
                        &img.rgb,
                        img.w,
                        img.h,
                        Rect {
                            x: 282,
                            y: 179,
                            w: 960,
                            h: 421,
                        },
                    );
                }
                u.text(
                    282,
                    618,
                    12,
                    &if movie.document {
                        format!(
                            "DOCUMENT / PAGE {} OF {} / LEFT & RIGHT TO TURN",
                            v.frame + 1,
                            movie.frames.len()
                        )
                    } else {
                        format!(
                            "SILENT FIELD VIDEO / FRAME {} OF {} / SPACE PAUSE / ARROWS SEEK",
                            v.frame + 1,
                            movie.frames.len()
                        )
                    },
                    MUTED,
                );
            } else {
                let text=core::str::from_utf8(&v.data).unwrap_or("This binary format has no native viewer. PNG, JPEG, BMP, text and converted field video are supported. PDF / Office / VLC are not ported.");
                for (i, line) in text.lines().skip(v.scroll).take(18).enumerate() {
                    let shown = line
                        .chars()
                        .take(112)
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect::<String>();
                    u.text(284, 184 + i * 23, 14, &shown, WHITE);
                }
                u.text(
                    282,
                    618,
                    12,
                    "UP / DOWN SCROLL  |  ESC CLOSE  |  L LOCK",
                    MUTED,
                );
            }
        }
        if !self.notice.is_empty() {
            u.notice(&self.notice, self.bad);
        }
        if let Some(m) = &self.modal {
            let (t, d, secret) = match m.kind {
                Prompt::Unlock => (
                    "UNLOCK VAULT",
                    "Passphrase stays in memory. Authentication must succeed before any object can open.",
                    true,
                ),
                Prompt::Create => (
                    "CREATE A VAULT",
                    "Choose a long passphrase of at least 16 bytes. There is no password recovery or universal key.",
                    true,
                ),
                Prompt::Repeat => (
                    "REPEAT PASSPHRASE",
                    "Confirm the passphrase before creating the encrypted vault on the boot medium.",
                    true,
                ),
                Prompt::Clone(_) => (
                    "OVERWRITE DESTINATION",
                    "This erases the selected removable destination. Type CLONE followed by its disk number, for example CLONE 2.",
                    false,
                ),
            };
            u.modal(t, d, &m.input, secret);
        }
        if self.mouse != (0, 0) {
            u.cursor(self.mouse.0, self.mouse.1);
        }
        self.dirty = false;
    }
    fn open_selected(&mut self) {
        if self.page != 1 {
            return;
        }
        let Some(key) = &self.key else {
            self.prompt(Prompt::Unlock);
            return;
        };
        if let Some(e) = self.files.get(self.selected) {
            match self.storage.view(&e.name, key) {
                Ok((name, data)) => {
                    let image = media::decode(&data).ok();
                    let movie = media::movie(&data).ok();
                    let paused = movie.as_ref().is_some_and(|m| m.document);
                    self.viewer = Some(Viewer {
                        name: Zeroizing::new(name),
                        data,
                        image,
                        movie,
                        frame: 0,
                        scroll: 0,
                        paused,
                    });
                    self.notice.clear();
                    self.dirty = true;
                }
                Err(e) => self.notify(e, true),
            }
        }
    }
    fn submit(&mut self, u: &mut Ui, screen: &mut Screen) {
        let Some(m) = self.modal.take() else {
            return;
        };
        self.dirty = true;
        match m.kind {
            Prompt::Create => {
                if m.input.len() < 16 {
                    self.notify("PASSPHRASE MUST CONTAIN AT LEAST 16 BYTES", true);
                } else if self.storage.has_key() {
                    self.notify("A VAULT ALREADY EXISTS / USE UNLOCK", true);
                } else {
                    self.pending = Some(m.input);
                    self.prompt(Prompt::Repeat);
                }
            }
            Prompt::Repeat => {
                let Some(p) = self.pending.take() else {
                    return;
                };
                if *p != *m.input {
                    self.notify("PASSPHRASES DIFFER / VAULT WAS NOT CREATED", true);
                    return;
                }
                u.notice("DERIVING KEY / PLEASE WAIT", false);
                screen.show(u);
                match self.storage.initialize(p.as_bytes()) {
                    Ok(k) => {
                        self.key = Some(k);
                        self.refresh();
                        self.notify("VAULT CREATED AND HEADER READBACK AUTHENTICATED", false);
                    }
                    Err(e) => self.notify(e, true),
                }
            }
            Prompt::Unlock => {
                u.notice("AUTHENTICATING VAULT / PLEASE WAIT", false);
                screen.show(u);
                match self.storage.unlock(m.input.as_bytes()) {
                    Ok(k) => {
                        self.key = Some(k);
                        self.refresh();
                        self.notify("VAULT UNLOCKED / OBJECTS AUTHENTICATE WHEN OPENED", false);
                    }
                    Err(_) => self.notify(
                        "UNLOCK FAILED / WRONG PASSPHRASE, CORRUPT HEADER OR MISSING VAULT",
                        true,
                    ),
                }
            }
            Prompt::Clone(i) => {
                if *m.input != format!("CLONE {}", i + 1) {
                    self.notify("DESTINATION PHRASE DID NOT MATCH / NO WRITES", true);
                    return;
                }
                let Some(target) = self.disks.get(i).cloned() else {
                    return;
                };
                self.viewer = None;
                self.key = None;
                self.wipe_names();
                let mut last_percent = 101;
                let result = self.storage.clone_drive(&target, |n, total| {
                    let percent = n * 100 / total.max(1);
                    if percent != last_percent {
                        u.progress("COPY / READBACK AUTHENTICATION", n, total);
                        screen.show(u);
                        last_percent = percent;
                    }
                });
                match result {
                    Ok(hash) => self.notify(format!("CLONE VERIFIED / SHA256 {hash}"), false),
                    Err(e) => self.notify(
                        format!("CLONE FAILED / DESTINATION MAY BE INCOMPLETE / {e}"),
                        true,
                    ),
                }
            }
        }
    }
    fn action(&mut self, key: Key, u: &mut Ui, screen: &mut Screen) -> bool {
        self.last_input = self.ticks;
        self.dirty = true;
        if self.modal.is_some() {
            match key {
                Key::Special(ScanCode::ESCAPE) => {
                    self.modal = None;
                    self.pending = None;
                }
                Key::Printable(c) => {
                    let c = char::from(c);
                    if c == '\r' {
                        self.submit(u, screen);
                    } else if c == '\u{8}' {
                        if let Some(m) = &mut self.modal {
                            m.input.pop();
                        }
                    } else if !c.is_control()
                        && let Some(m) = &mut self.modal
                        && m.input.len() < 512
                    {
                        m.input.push(c);
                    }
                }
                _ => {}
            }
            return true;
        }
        if let Key::Printable(c) = key {
            match char::from(c).to_ascii_lowercase() {
                'l' => {
                    self.lock();
                    return true;
                }
                'q' => {
                    self.lock();
                    u.clear(BG);
                    screen.clear();
                    return false;
                }
                _ => {}
            }
        }
        if let Some(v) = &mut self.viewer {
            match key {
                Key::Special(ScanCode::ESCAPE) => {
                    self.viewer = None;
                }
                Key::Special(ScanCode::UP) => v.scroll = v.scroll.saturating_sub(1),
                Key::Special(ScanCode::DOWN) => v.scroll = v.scroll.saturating_add(1),
                Key::Special(ScanCode::PAGE_DOWN) => v.scroll = v.scroll.saturating_add(16),
                Key::Special(ScanCode::PAGE_UP) => v.scroll = v.scroll.saturating_sub(16),
                Key::Special(ScanCode::RIGHT) => {
                    if let Some(m) = &v.movie {
                        v.frame =
                            (v.frame + if m.document { 1 } else { 10 }).min(m.frames.len() - 1);
                    }
                }
                Key::Special(ScanCode::LEFT) => {
                    v.frame =
                        v.frame
                            .saturating_sub(if v.movie.as_ref().is_some_and(|m| m.document) {
                                1
                            } else {
                                10
                            })
                }
                Key::Printable(c) if char::from(c) == ' ' => v.paused = !v.paused,
                _ => {}
            }
            return true;
        }
        match key {
            Key::Special(ScanCode::UP) => self.selected = self.selected.saturating_sub(1),
            Key::Special(ScanCode::DOWN) => {
                self.selected = (self.selected + 1).min(self.count().saturating_sub(1))
            }
            Key::Special(ScanCode::ESCAPE) => {
                self.storage.revoke_all();
                self.browse = None;
                self.entries.clear();
                self.selected = 0;
            }
            Key::Printable(c) => {
                let c = char::from(c);
                match c.to_ascii_lowercase() {
                    '1'..='5' => self.goto(c as usize - '1' as usize),
                    'u' => self.prompt(Prompt::Unlock),
                    'n' => self.prompt(Prompt::Create),
                    'i' => self.goto(2),
                    'r' => {
                        self.refresh();
                        let _ = self.storage.refresh_volumes();
                        self.notify("DEVICE LIST REFRESHED / ALL IMPORT GRANTS REVOKED", false);
                    }
                    '\u{8}' => {
                        if let Some((idx, mut p)) = self.browse.take() {
                            if let Some(pos) = p.rfind('\\') {
                                p.truncate(pos);
                            }
                            if p.is_empty() {
                                p.push('\\');
                            }
                            match self.storage.browse(idx, &p) {
                                Ok(e) => {
                                    self.entries = e;
                                    self.browse = Some((idx, p));
                                    self.selected = 0;
                                }
                                Err(e) => self.notify(e, true),
                            }
                        }
                    }
                    '\r' => {
                        if self.browse.is_some() {
                            self.import_selected(u, screen);
                        } else {
                            match self.page {
                                1 => self.open_selected(),
                                2 => {
                                    let idx = self.selected;
                                    match self.storage.grant_volume(idx) {
                                        Ok(()) => match self.storage.browse(idx, "\\") {
                                            Ok(e) => {
                                                self.entries = e;
                                                self.browse = Some((idx, "\\".to_string()));
                                                self.selected = 0;
                                            }
                                            Err(e) => {
                                                self.storage.revoke_all();
                                                self.notify(e, true);
                                            }
                                        },
                                        Err(e) => self.notify(e, true),
                                    }
                                }
                                3 => {
                                    if let Some(d) = self.disks.get(self.selected) {
                                        if d.policy.boot
                                            || !d.policy.removable
                                            || d.policy.read_only
                                        {
                                            self.notify("SELECT A WRITABLE REMOVABLE DESTINATION / SOURCE IS PROTECTED",true);
                                        } else {
                                            self.prompt(Prompt::Clone(self.selected));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        true
    }
    fn import_selected(&mut self, u: &mut Ui, screen: &mut Screen) {
        let Some((idx, p)) = self.browse.clone() else {
            return;
        };
        let Some(e) = self.entries.get(self.selected).cloned() else {
            return;
        };
        let full = if p == "\\" {
            format!("\\{}", e.name)
        } else {
            format!("{p}\\{}", e.name)
        };
        if e.directory {
            match self.storage.browse(idx, &full) {
                Ok(es) => {
                    self.entries = es;
                    self.browse = Some((idx, full));
                    self.selected = 0;
                }
                Err(e) => self.notify(e, true),
            }
            return;
        }
        let Some(key) = &self.key else {
            self.storage.revoke_all();
            self.browse = None;
            self.prompt(Prompt::Unlock);
            return;
        };
        u.notice("ENCRYPTING COPY / VERIFYING READBACK / PLEASE WAIT", false);
        screen.show(u);
        let r = self.storage.import(idx, &full, &e.name, key);
        self.browse = None;
        self.entries.clear();
        self.selected = 0;
        self.refresh();
        match r {
            Ok(()) => self.notify(
                "IMPORT VERIFIED / SOURCE UNMODIFIED / READ GRANT REVOKED",
                false,
            ),
            Err(e) => self.notify(format!("IMPORT FAILED / {e}"), true),
        }
    }
    fn click(&mut self, x: usize, y: usize, u: &mut Ui, screen: &mut Screen) -> bool {
        if self.modal.is_some() || self.viewer.is_some() {
            return true;
        }
        if x < 244 {
            if (169..478).contains(&y) {
                self.goto(((y - 169) / 62).min(4));
            }
            return true;
        }
        if self.page == 0
            && (Rect {
                x: 282,
                y: 393,
                w: 224,
                h: 46,
            })
            .contains(x, y)
        {
            if self.key.is_some() {
                self.goto(1);
            } else {
                self.prompt(Prompt::Unlock);
            }
            return true;
        }
        let (step, start) = if self.browse.is_some() {
            (39, 249)
        } else {
            match self.page {
                1 => (40, 241),
                2 | 3 => (56, 245),
                _ => (1, 999),
            }
        };
        if y >= start && y < start + step * 9 {
            let offset = self
                .selected
                .saturating_sub(if self.page == 1 || self.browse.is_some() {
                    7
                } else {
                    5
                });
            let i = (y - start) / step + offset;
            if i < self.count() {
                if i == self.selected {
                    return self.action(
                        Key::Printable(uefi::Char16::try_from('\r').unwrap()),
                        u,
                        screen,
                    );
                }
                self.selected = i;
                self.dirty = true;
            }
        }
        true
    }
}

#[entry]
fn main() -> Status {
    if uefi::helpers::init().is_err() {
        return Status::ABORTED;
    }
    uefi::system::with_stdout(|o| {
        let _ = o.enable_cursor(false);
    });
    let mut screen = match Screen::new() {
        Ok(s) => s,
        Err(_) => return Status::UNSUPPORTED,
    };
    let mut u = Ui::new();
    for i in 0..=4 {
        u.splash(i);
        screen.show(&u);
        boot::stall(Duration::from_millis(160));
    }
    let storage = match Storage::new() {
        Ok(s) => s,
        Err(e) => {
            u.notice(&format!("BOOT MEDIUM ERROR / {e}"), true);
            screen.show(&u);
            loop {
                boot::stall(Duration::from_secs(1));
            }
        }
    };
    let mut pointer = boot::find_handles::<Pointer>()
        .ok()
        .and_then(|h| h.first().copied())
        .and_then(|h| boot::open_protocol_exclusive::<Pointer>(h).ok());
    if let Some(p) = &mut pointer {
        let _ = p.reset(false);
    }
    let mut absolute = boot::find_handles::<AbsolutePointer>()
        .ok()
        .and_then(|h| h.first().copied())
        .and_then(|h| boot::open_protocol_exclusive::<AbsolutePointer>(h).ok());
    let mut app = App::new(storage);
    let mut running = true;
    while running {
        if app.dirty {
            app.draw(&mut u);
            screen.show(&u);
        }
        if let Ok(Some(k)) = uefi::system::with_stdin(|i| i.read_key()) {
            running = app.action(k, &mut u, &mut screen);
        }
        let mut clicked = false;
        if let Some(p) = &mut absolute
            && let Ok(Some(s)) = p.read_state()
        {
            let mode = p.mode();
            let x = ((s.current_x.saturating_sub(mode.absolute_min_x)) * W as u64
                / (mode
                    .absolute_max_x
                    .saturating_sub(mode.absolute_min_x)
                    .max(1))) as usize;
            let y = ((s.current_y.saturating_sub(mode.absolute_min_y)) * H as u64
                / (mode
                    .absolute_max_y
                    .saturating_sub(mode.absolute_min_y)
                    .max(1))) as usize;
            app.mouse = (x.min(W - 1), y.min(H - 1));
            let down = s.active_buttons & 1 != 0;
            clicked = down && !app.mouse_down;
            app.mouse_down = down;
            app.dirty = true;
            app.last_input = app.ticks;
        }
        if let Some(p) = &mut pointer
            && let Ok(Some(s)) = p.read_state()
        {
            app.mouse = (
                (app.mouse.0 as i32 + s.relative_movement_x * 2).clamp(0, W as i32 - 1) as usize,
                (app.mouse.1 as i32 + s.relative_movement_y * 2).clamp(0, H as i32 - 1) as usize,
            );
            let down = bool::from(s.left_button);
            clicked |= down && !app.mouse_down;
            app.mouse_down = down;
            app.dirty = true;
            app.last_input = app.ticks;
        }
        if clicked {
            running = app.click(app.mouse.0, app.mouse.1, &mut u, &mut screen);
        }
        app.ticks += 1;
        if app.key.is_some() && app.ticks - app.last_input > 30000 {
            app.lock();
        }
        if let Some(v) = &mut app.viewer
            && let Some(m) = &v.movie
            && !m.document
            && !v.paused
            && app.ticks.is_multiple_of((100 / m.fps as u64).max(1))
        {
            v.frame = (v.frame + 1) % m.frames.len();
            app.dirty = true;
        }
        boot::stall(Duration::from_millis(10));
    }
    screen.clear();
    uefi::runtime::reset(uefi::runtime::ResetType::SHUTDOWN, Status::SUCCESS, None)
}

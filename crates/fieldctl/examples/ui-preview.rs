//! Render the same compositor/effects as the guest, with PUBLIC fixture entropy.
extern crate alloc;
#[allow(dead_code)]
#[path = "../../field-boot/src/signals.rs"]
mod signals;
#[allow(dead_code)]
#[path = "../../field-boot/src/ui.rs"]
mod ui;
use std::{fs, io::Write, path::PathBuf};
fn save(ui: &ui::Ui, path: PathBuf) {
    let mut f = fs::File::create(path).unwrap();
    write!(f, "P6\n{} {}\n255\n", ui::W, ui::H).unwrap();
    let rgb: Vec<u8> = ui
        .pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    f.write_all(&rgb).unwrap();
}
fn main() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-output/ui-preview");
    fs::create_dir_all(&out).unwrap();
    let mut signal = signals::Signal::new(Some(([41; 56], "PUBLIC TEST FIXTURE")));
    signal.set_sample(Some(([0xa3; 32], "PUBLIC TEST FIXTURE")));
    let mut u = ui::Ui::new(signal);
    for _ in 0..19 {
        u.signal.advance();
    }
    u.overview(false, 0, 1);
    u.ambient(0);
    u.motion_status();
    save(&u, out.join("overview.ppm"));
    u.shell(5, false);
    u.signal_lab();
    u.ambient(5);
    u.motion_status();
    save(&u, out.join("signals.ppm"));
    u.splash(2);
    u.boot_signal();
    save(&u, out.join("boot.ppm"));
    println!("Public-fixture native UI previews: {}", out.display());
}

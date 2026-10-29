NULLGRAV 0.3.0 adds an animated Signal Lab and a glitch-driven boot/dashboard UI.

- Moving hexadecimal cascades, a sweeping field scope, waveform activity,
  particles, scan tracers, and short horizontal glitch bursts in the artwork.
- Page 6 shows a fresh 256-bit public sample from firmware RNG or checked CPU
  RDRAND. Press R to draw again. Display animation has its own independently
  seeded XChaCha20 state; it never receives vault keys or passphrases.
- Space holds the Signal Lab stream and G disables motion. Effects pause during
  passphrase entry and media viewing. Scope/waveform graphics are simulated.
- A versioned interactive VM launcher keeps older preview test disks separate.

The three-failure erase policy remains enabled: the third consecutive failed
native unlock starts destruction of the entire boot card, including unused
capacity and boot records, then shuts down for safe removal. A correct unlock
resets the persistent count. Keep separate backups and read docs/ERASURE.md.

Validation includes 16 automated Rust tests, strict host/native Clippy and
formatting checks, and native QEMU verification of animation, pause, sample
regeneration, motion-off, stable masked prompts, disk invariance and shutdown.
The full native erase regression is also checked on disposable VM images.

This is an unsigned, firmware-backed engineering preview, with no independent
security audit, standalone kernel isolation or classified-data approval. Visual
effects add no cryptographic assurance. Earlier copies/offline tools bypass the
software retry limit, and logical overwrites cannot prove SD-flash sanitization.
Physical J3455/SD hardware remains untested; no real SD was written.

Extract the archive, read README.md and SECURITY.md, and verify SHA256SUMS.txt.
TEST NULLGRAV VM.cmd launches a visible VM through Ubuntu WSL/WSLg with QEMU/OVMF
installed. FLASH NULLGRAV.cmd lists devices and requires a specific empty target;
it never automatically writes a newly inserted USB/SD device.

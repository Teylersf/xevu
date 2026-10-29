NULLGRAV 0.2.0 adds a destructive three-failure policy to native vault unlock.
**The third consecutive failed authentication erases the entire boot card,
including unused capacity and boot files, then shuts down for safe removal.**
A correct unlock resets the persistent counter. Keep separate backups.

Includes the x86-64 UEFI boot image, Windows fieldctl, an explicit USB/SD installer,
encrypted vault, FAT source import, image/text/converted-document viewers, silent
converted MJPEG playback, and installed-image cloning with readback verification.

The erase path overwrites the wrapped key first, makes one random logical pass
with readback checks, invalidates boot records, and requests driver detach.
It uses normal writes and leaves the card available for reformatting. Readers
without mechanical eject support require physical removal after shutdown.
See docs/ERASURE.md for the exact scope and failure behavior.

Validation includes 15 automated Rust tests, strict host/native Clippy, formatting,
dependency advisories, and a disposable QEMU test of persistent failures, successful
reset, whole-card erasure, unchanged other disks, shutdown, and reuse after reimage.
The earlier 0.1.0 preview also exercised import, media viewing and verified cloning.

This is a software policy, not a hardware retry barrier. An earlier clone or
offline tool can bypass it. SD wear leveling can retain physical key/data pages;
logical overwriting does not prove forensic sanitization. No real SD was erased.

This is firmware-backed software, not a completed standalone kernel OS. No
isolated userspace, VLC/general MP4/audio support, native PDF/Office parsers,
Secure Boot signing, independent security audit, or government approval is
included. The physical J3455 and SD reader are untested. Use public test content.

Extract the archive and read README.md and SECURITY.md. The image is under dist/.
FLASH NULLGRAV.cmd lists disks and asks for a specific empty USB/SD target; it
never automatically selects or writes a newly plugged-in device. No physical
media was written during this build. Checksums are included in SHA256SUMS.txt.

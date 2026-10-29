NULLGRAV 0.1.0 is a bootable engineering preview for offline antigravity research
transport, with a native private aerospace contractor visual style.

Includes the x86-64 UEFI boot image, Windows fieldctl, an explicit USB/SD installer,
encrypted vault, FAT source import, image/text/converted-document viewers, silent
converted MJPEG playback, and installed-image cloning with readback verification.

Validation: 12 automated tests, strict host/native Clippy checks, source-format
checks, an advisory audit with no known vulnerabilities, and native QEMU flows.
The 256 MiB installed image was cloned from a larger synthetic physical source
to a smaller target; source content remained unchanged.

This is firmware-backed software, not a completed standalone kernel OS. No
isolated userspace, VLC/general MP4/audio support, native PDF/Office parsers,
Secure Boot signing, independent security audit, or government approval is
included. The physical J3455 and SD reader are untested. Use public test content.

Extract the archive and read README.md and SECURITY.md. The image is under dist/.
FLASH NULLGRAV.cmd lists disks and asks for a specific empty USB/SD target; it
never automatically selects or writes a newly plugged-in device. No physical
media was written during this build. Checksums are included in SHA256SUMS.txt.

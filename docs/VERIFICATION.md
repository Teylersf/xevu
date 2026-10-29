# Verification record

Verified on 2026-10-28 using Rust 1.97.0 on Windows and QEMU/OVMF in Ubuntu WSL.
Screenshots are captured from the running UEFI executable, not a browser mockup.
All media in tests are explicitly public, synthetic fixtures.

The development host can build the native executable and host CLI. The Apollo
Lake J3455 physical machine and a removable USB/SD target have not been tested.

## Automated checks

- 15 Rust tests pass: encrypted round trips, empty/boundary-sized objects,
  metadata confidentiality, wrong keys/passwords, corruption, truncation,
  appended bytes, cross-file/chunk substitution, bounded KDF parameters,
  password/name/size restrictions, nonce-reuse refusal, device grants/revocation,
  clone target protections and extent bounds, streaming host import/extraction,
  image decoder fixtures, converted media bounds, persistent three-failure state,
  successful reset, torn/corrupt retry records, and boot-only erasure authority.
- Host and UEFI `cargo clippy` checks pass with `-D warnings`.
- `cargo fmt --all -- --check` and `git diff --check` pass.
- PowerShell scripts parse; the volume-lock interop compiles without opening any
  device. The writer's default listing mode was exercised without writing media.
- `cargo audit` 0.22.2 checked 59 locked dependencies against RustSec database
  commit `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`: no known vulnerabilities or
  informational warnings. See [dependency-audit.json](dependency-audit.json).
  This is an advisory check, not a cryptographic or systems audit.

## Original 0.1.0 native flows

- Booted the MBR/FAT32 image via OVMF and reached the native dashboard.
- Created a vault by typing/repeating a public fixture passphrase through the
  guest keyboard; the saved header read back and authenticated.
- Granted temporary access to a FAT source, imported text and BMP objects,
  authenticated their encrypted readback, and displayed the plaintext in RAM.
- Imported converted two-page PDF content and navigated pages in the JPEG-page
  viewer. Imported, played, and paused a 20-frame silent MJPEG fixture.
- Locked the vault and observed wrong-passphrase rejection in the native UI.
- The boot source was refused as a clone destination.
- A 256 MiB installed image on a 384 MiB physical source was replicated to a
  smaller 256 MiB destination. External SHA-256 comparison matched the installed
  extent, and the 64 MiB import-source image's hash remained unchanged. The
  source's synthetic unallocated tail was outside the copied extent. See
  [clone-verification.json](clone-verification.json).

Initial 0.1.0 emulator sessions exposed the firmware's default five-minute
watchdog. The boot entry explicitly disables it and refuses to continue on a real
configuration error; that behavior is retained in 0.2.0. The 0.1.0 release's
separate long-session receipt records survival beyond that deadline and the
tested image digest. That earlier image also completed native PNG import/view.
No physical media or host system disk was modified by those tests.

## 0.2.0 native erase verification

The final 0.2.0 image was booted in a separate disposable headless VM. A vault
was created through the guest keyboard and a public text fixture was imported.
Two incorrect unlocks survived a complete QEMU power cycle, and a correct
passphrase reset the durable count. Three further incorrect authentications
triggered key destruction, a verified 384 MiB logical overwrite (including the
unallocated tail), invalid MBR/FAT boot records, and automatic shutdown. SHA-256
of both other attached disk images stayed unchanged. The erased virtual card
retained its capacity and booted a newly written clean image successfully.
See [purge-verification.json](purge-verification.json) and the native
[warning](images/erase-warning.png) / [progress](images/erase-progress.png) captures.
The first exploratory run exposed premature driver disconnect removing raw disk
access; the final path closes owned files and keeps BlockIO available until done.
A failed pass leaves a terminal failure screen and does not report success.

A second VM booted a synthetic interrupted-erasure snapshot with a committed
three-failure trigger and an already overwritten key capsule. Startup resumed
erasure without any input, invalidated boot records and shut down, leaving both
other virtual disks unchanged. This is controlled fault injection, not physical
power-loss validation; see [purge-resume-verification.json](purge-resume-verification.json).

These are regular-file virtual disks, not physical SD-flash sanitization tests.
No real SD or host disk was written. Earlier interactive 0.1.0 VM state is kept
separate from this release's versioned 0.2.0 VM directory.

## Build artifacts

The final clean, secret-free release image is 268,435,456 bytes.

- Image SHA-256: `ccfa1af57d71d1dc4ba724888ca5c4406f21150853a80f20b48f3e87f6d19b1f`
- EFI executable SHA-256: `1b78edd5d89851abcb524006ba9d2dbdb2e556f496811c706e3574e042942a21`

The package is read back from the ZIP and every payload is SHA-256 compared with
the corresponding build artifact. Test vaults, fixture passphrases, OVMF state,
developer caches, and emulator disk images are excluded from the public archive.

## Limits of this record

No physical J3455/SD boot result, firmware/hardware attack resistance, isolated
userspace, native VLC/general codecs/audio, native PDF/Office parsing, new cipher
security proof, independent penetration test, or classified-data approval exists.

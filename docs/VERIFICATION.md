# Verification record

Verified on 2026-10-28 using Rust 1.97.0 on Windows and QEMU/OVMF in Ubuntu WSL.
Screenshots are captured from the running UEFI executable, not a browser mockup.
All media in tests are explicitly public, synthetic fixtures.

The development host can build the native executable and host CLI. The Apollo
Lake J3455 physical machine and a removable USB/SD target have not been tested.

## Automated checks

- 16 Rust tests pass: encrypted round trips, empty/boundary-sized objects,
  metadata confidentiality, wrong keys/passwords, corruption, truncation,
  appended bytes, cross-file/chunk substitution, bounded KDF parameters,
  password/name/size restrictions, nonce-reuse refusal, device grants/revocation,
  clone target protections and extent bounds, streaming host import/extraction,
  image decoder fixtures, converted media bounds, persistent three-failure state,
  successful reset, torn/corrupt retry records, boot-only erasure authority, and
  independent cosmetic stream state, pause behavior and failed-sample clearing.
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
The screenshots show the current 0.3.0 build; the linked 0.2.0 receipts retain
their original tested image digests.
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

## 0.3.0 animated UI verification

The new image reached the native dashboard after seven distinct captured boot
animation frames. Signal Lab produced 18 distinct native screen captures. The
VM confirmed Space freezes the complete stream, R changes the displayed sample
while the scope remains held, G disables/resumes motion, and passphrase entry
pauses effects with masked input. Canceling the prompt left retry records blank.
SHA-256 of all three attached virtual disks remained unchanged during the UI
flow, and Q shut down the guest. See [signals-verification.json](signals-verification.json).
The sample source observed in that guest was UEFI RNG; scope/waveform graphics
are simulated. Visual effects are not a cryptographic health test.

The final 0.3.0 image also passed the complete native erase regression: vault
initialization/import, two failed attempts across reboot, a correct reset, three
new failures, all 384 MiB overwritten, unchanged other disks, invalid boot records,
automatic shutdown, and successful reuse after reimaging. See
[purge-0.3.0-verification.json](purge-0.3.0-verification.json).

## 0.3.1 Windows file-import tooling

Three Python helper tests cover unchanged source content, fresh image creation,
file/size/name/output bounds and Unicode long-filename encoding. A native QEMU
session received a newly attached read-only virtual USB image while running,
refreshed Disk Gate, imported a file with spaces and an ampersand in its name,
and opened its encrypted copy in the native text viewer. Original source,
transfer image and other virtual disk hashes stayed unchanged. Restart retained
the vault and reattached the latest transfer read-only. The actual Windows
PowerShell wrapper was exercised through WSL against a separate test session.
See [vm-import-verification.json](vm-import-verification.json) and the native
[imported-file view](images/windows-file-import.png).

The guest image is byte-for-byte identical to 0.3.0. All test data were public
synthetic fixtures; no physical disk was attached or written. Host file-picker
selections are stored only in ignored local state, never release artifacts.

## Build artifacts

The final clean, secret-free release image is 268,435,456 bytes.

- Image SHA-256: `493f2b7524b399b4d0732ca259245c492280288f17bb303cee75e38e16b81e25`
- EFI executable SHA-256: `647aba288385abf766c8f1ccd7abc87acedc6679357cab3711e623c4685777be`

The package is read back from the ZIP and every payload is SHA-256 compared with
the corresponding build artifact. Test vaults, real secrets, OVMF state,
developer caches, and emulator disk images are excluded from the public archive.
Verification source contains explicitly public synthetic fixture credentials.

## Limits of this record

No physical J3455/SD boot result, firmware/hardware attack resistance, isolated
userspace, native VLC/general codecs/audio, native PDF/Office parsing, new cipher
security proof, independent penetration test, or classified-data approval exists.

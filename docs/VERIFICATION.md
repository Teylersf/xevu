# Verification record

Verified on 2026-10-28 using Rust 1.97.0 on Windows and QEMU/OVMF in Ubuntu WSL.
Screenshots are captured from the running UEFI executable, not a browser mockup.
All media in tests are explicitly public, synthetic fixtures.

The development host can build the native executable and host CLI. The Apollo
Lake J3455 physical machine and a removable USB/SD target have not been tested.

## Automated checks

- 12 Rust tests pass: encrypted round trips, empty/boundary-sized objects,
  metadata confidentiality, wrong keys/passwords, corruption, truncation,
  appended bytes, cross-file/chunk substitution, bounded KDF parameters,
  password/name/size restrictions, nonce-reuse refusal, device grants/revocation,
  clone target protections and extent bounds, streaming host import/extraction,
  image decoder fixtures, and converted media bounds.
- Host and UEFI `cargo clippy` checks pass with `-D warnings`.
- `cargo fmt --all -- --check` and `git diff --check` pass.
- PowerShell scripts parse; the volume-lock interop compiles without opening any
  device. The writer's default listing mode was exercised without writing media.
- `cargo audit` 0.22.2 checked 59 locked dependencies against RustSec database
  commit `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`: no known vulnerabilities or
  informational warnings. See [dependency-audit.json](dependency-audit.json).
  This is an advisory check, not a cryptographic or systems audit.

## Observed native flows

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

Two long emulator sessions were interrupted by the surrounding WSL runtime
restarting; the saved images were retained, the VM resumed, and the checks were
continued. No physical media or host system disk was modified. No claim of a
completed long-duration or power-failure test is made.

## Build artifacts

The final clean, secret-free release image is 268,435,456 bytes.

- Image SHA-256: `c2ff8218f58f69e279072e08d7049431d69a6be3c3a579456cfcfd9e2e47950f`
- EFI executable SHA-256: `aa2d45f0ba0e7b2eb14fb9c3a3dc4799e9af3c9064932f1c702fa226233916e0`

The package is read back from the ZIP and every payload is SHA-256 compared with
the corresponding build artifact. Test vaults, fixture passphrases, OVMF state,
developer caches, and emulator disk images are excluded from the public archive.

## Limits of this record

No physical J3455/SD boot result, firmware/hardware attack resistance, isolated
userspace, native VLC/general codecs/audio, native PDF/Office parsing, new cipher
security proof, independent penetration test, or classified-data approval exists.

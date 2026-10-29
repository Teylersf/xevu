# Security status and threat model

NULLGRAV 0.1 is an engineering preview. Do not entrust it with sensitive leaks,
government-classified material, or irreplaceable originals. Security properties
below describe code behavior, not an independent assurance or accreditation.

## Intended threat

An attacker obtains a **locked** removable medium and attempts to read its file
contents or filenames, change ciphertext, truncate objects, or substitute chunks.
The host computer, firmware, random generator, source files, dependencies, and
authorized operator are assumed trustworthy. A strong passphrase is required.

The original broader goal is not yet met: trusted firmware is part of this build,
and all native code runs in one address space. It does not enforce a boundary
against a compromised viewer or malicious kernel/firmware-level code.

## Implemented controls

- Argon2id v1.3: 64 MiB, three iterations, one lane, independent 16-byte salt.
  Imported KDF settings are bounded to 8–64 MiB, 1–3 iterations, one lane.
- A random 256-bit master key is wrapped by XChaCha20-Poly1305. Per-file keys are
  derived with HKDF-SHA256 using random 128-bit file IDs and a fixed domain label.
- Each object has a separate random 128-bit nonce prefix. Its 64-bit chunk index
  completes a 192-bit nonce. Encryption permits each index only once per writer.
- The complete object header and chunk index are authenticated as AAD. Metadata
  is encrypted. Padded data chunks authenticate before being returned to viewers.
  Expected total size rejects both truncation and appended bytes.
- Firmware RNG is preferred. A checked RDRAND fallback exists for x86-64.
  Failures produce an error; clocks, filenames, or predictable seeds are never
  used as cryptographic entropy.
- No default key, public passphrase, hardcoded recovery key, analytics, or network
  transport is present in the release image.
- Device discovery reads firmware metadata. Application file browsing of other
  FAT volumes is gated by an explicit grant. Source files open in read-only mode.
  Leaving the browser, locking, refreshing, and completing an import revoke grants.
- Clone targets must be removable, writable, distinct from the boot source, and
  large enough for the installed image extent. Identity is rechecked before write.
  SHA-256 readback is compared for every copied block batch.

## Important limits

- Native unlock has an automatic three-failure erase policy. It is not a
  hardware-enforced guessing limit or guaranteed flash sanitization. See
  [ERASURE.md](docs/ERASURE.md) for scope, crash behavior, offline bypasses,
  data-loss risk, and the exact overwrite sequence.

- UEFI and its filesystem drivers can probe disks during device discovery.
  Application policy is not hardware-level disk blocking. GetProtocol borrows
  deliberately avoid disconnecting the firmware's active filesystem drivers.
- The firmware boot executable and generated image are **unsigned**. No verified
  or measured chain of trust exists. The artwork is not a security signal.
- Passwords, content, decoder temporaries, and pixels exist in RAM while open.
  Zeroizing owned buffers cannot guarantee erasure of firmware/decoder copies,
  CPU caches, allocator state, GPU/framebuffers, or cold-boot remnants.
- Parsers are bounded but run with the same privileges as the storage broker.
  Fuzzing and independent decoder/security review are still required.
- No anti-rollback state exists. An attacker can restore an older valid object,
  omit complete files, or restore an older entire vault snapshot undetected.
- Counts, padded sizes, directory locations, filesystem timestamps, boot files,
  and the existence of a NULLGRAV vault remain observable. No plausible-deniability
  or hidden-volume guarantee is made.
- Lost passphrases mean lost access. Clones duplicate keys and credentials; they
  are not independently rekeyed recipients. Changing a passphrase/rekeying is not
  implemented. Send the medium and passphrase by separate trusted means.
- The native path currently loads each object into memory, capped at 128 MiB.
  Video playback is silent converted MJPEG, not a VLC port or general codec stack.
  Document viewing uses converted JPEG pages; original PDF/Office parsers are not
  present. Text display currently covers ASCII glyphs only.
- Imported plaintext stays on its original source. Importing is a copy, not secure
  deletion. Flash wear leveling prevents ordinary deletion from proving erasure.
- The installer writes/verifies only the image range, leaving any previous bytes
  beyond it. Use new or independently sanitized media for real investigations.
  Clone code bounds its range to the installed ESP and excludes that unused tail.
  Unused bytes inside the installed partition can still hold earlier ciphertext.
- Physical J3455 compatibility, SD boot, disconnect/reconnect behavior, malicious
  USB devices, hardware keyloggers, DMA attacks, and hostile firmware are unverified.
- No promises prevent a recipient from photographing, copying, or redistributing
  authorized content. An offline machine cannot enforce reliable remote revocation.

## Before handling real sensitive material

The roadmap requires a standalone kernel or an appropriately isolated microkernel
architecture, least-privilege userspace/decoder boundaries, native device policy,
a verified boot and update design, robust crash recovery, fuzzing, independent
cryptographic and systems audits, and supported-hardware validation. Novel storage
composition requires analysis; it is not a substitute for reviewed primitives.

Actual U.S. classified systems require their organization's approved architecture
and accreditation. This repository does not supply those approvals.

Report an issue privately through GitHub's private vulnerability reporting if
enabled; otherwise do not put secrets or real leaks into a public issue. There is
no established response-time or security-support SLA for this preview.

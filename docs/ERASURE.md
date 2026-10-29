# Three failed native unlocks

Version 0.2.0 enables this policy for every vault opened in the native boot UI.
**The third consecutive failed authentication destroys the entire current boot
card, including the vault, boot files, and unused space beyond the partition.**
A correct unlock resets the count to three remaining attempts. Canceling a prompt,
creating/repeating a passphrase, a missing file, and a rejected format/KDF do not
count as failed authentication. There is no cancel or recovery after the trigger.

## Sequence

1. Identify exactly one physical USB/SD boot device using the loaded image's
   device path. Require removable, writable, 512-byte sectors, 64 MiB–128 GiB,
   and the supported single ESP layout starting at LBA 2048. Internal disks and
   source/clone destinations receive no erasure authority.
2. Keep two checksummed retry records at LBAs 16 and 17 in the image's reserved
   MBR gap. Bind them to the SHA-256 of the key capsule. Flush/read back each
   update before reporting an incorrect password. On the third failure, commit
   the triggered state to both slots. A later boot resumes a valid triggered
   state before allowing normal interaction, if enough boot files still survive.
3. Drop owned plaintext, decoded media, passphrases, and master keys. Revoke import
   grants. Overwrite the 128-byte wrapped key capsule with independent random
   bytes, flush, and read it back when its file is still accessible.
4. Flush and close all owned boot filesystem handles, and make no further file
   calls on it. Keep the raw BlockIO provider connected; early disconnect can
   remove it on USB firmware. Recheck the boot disk's
   device path, media ID, size, block size, presence, and write protection.
5. Make one reverse pass over every exposed logical sector after LBA 17, including
   unused card capacity. Each batch uses XChaCha20 output from a fresh hardware/
   firmware random seed and a unique LBA-derived nonce. Flush and compare SHA-256
   of readback with the intended bytes for every batch.
6. Zero and verify LBAs 0–17 last. This invalidates the MBR, clears the retry
   journal, and leaves the overwritten EFI binary and FAT boot sectors unusable.
   Request a logical driver detach and shut down for safe physical removal.
   SD readers generally cannot mechanically eject a card. No firmware erase,
   sanitize, card password, permanent write protection, or controller lock is used.

The card's capacity and controller are preserved. It needs reformatting or a new
image before reuse. Ordinary writes consume a small amount of normal flash life;
this is not a zero-wear operation or a guarantee against faulty hardware.

## Limits that matter

- This is an application policy, **not a tamper-resistant retry limit**. Its
  checksum detects accidental corruption, not an attacker. An earlier image,
  edited counter, modified public source, or independent password cracking tool
  bypasses it. The host `fieldctl` preparation tool intentionally writes no raw
  disks and does not enforce this native boot policy.
- New vaults are armed during creation. A 0.1.0 or host-prepared vault with both
  raw slots blank is armed on its first native unlock. Nonblank invalid journals
  and mismatched capsules fail closed; they are not silently reset. A partially
  failed first initialization may require reflashing an otherwise empty test card.
- SD flash wear leveling can retain old physical pages, including key material.
  Logical overwrite/readback cannot establish forensic sanitization or NIST/NSA
  compliance. Earlier clones and external plaintext copies remain accessible.
  See [NIST SP 800-88 Rev. 2](https://csrc.nist.gov/pubs/sp/800/88/r2/final).
- Power loss can interrupt erasure. The key is attempted first, but no atomic
  whole-card destruction exists. Once EFI files are overwritten, the card may
  no longer boot to resume. A write/readback/disconnect error leaves a terminal
  failure screen; it never reports complete or reopens the vault. Press Q to
  shut down. An incomplete pass must not be treated as completed erasure.
- Only completed authentication failures are charged. A cut before the durable
  update can avoid a count. The redundant journal can recover the previous
  committed slot after a torn write; it cannot supply hardware anti-rollback.
- Someone who knows this policy can deliberately destroy a card with three bad
  submissions. Keep any necessary backups separate from the card.

Test the feature with `tools/verify_purge.py` in a disposable QEMU session. Never
test it on the only copy of research, or attach a physical host disk to that VM.

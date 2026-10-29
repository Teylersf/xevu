NULLGRAV 0.4.0 adds explicit export of ordinary PC files.

The vault keeps encrypted objects with hidden original names/content. Select a
file in the unlocked vault and press E to export its original filename and exact
unencrypted contents. Every source chunk authenticates before writing; the
staged and final file are SHA-256 verified. Existing files are not overwritten.

SAVE FILES TO WINDOWS.cmd asks for a destination folder (including C:), attaches
a writable WINDOWS_OUT virtual FAT drive, and waits while you export in NULLGRAV.
After Q shuts the guest down, it restores verified normal files into that folder.
The helper receives no vault key and never exposes the raw C: drive to the guest.
Temporary bridge cleanup follows successful output; failures can leave plaintext.

On real hardware, native export supports writable FAT USB/SD data drives. Files
are under NULLGRAV_EXPORTS and work in normal Windows. Direct native NTFS C:
writes remain unsupported; use the FAT drive after Windows boots. Original
PDF/MP4/etc. payloads export without conversion even when native viewing is not
supported. Converted NGD/NGM previews cannot recreate an original never imported.
File contents/names are preserved; timestamps, ACLs and alternate streams are not.

UPGRADE NULLGRAV VM.cmd copies an existing 0.3 encrypted vault and both retry
records into 0.4 after the old VM stops. It verifies readback, keeps the original
VM disks, preserves the password and never resets the failure count. A committed
erasure trigger, inconsistent state or an existing destination disk is refused.

Validation: 17 Rust tests, 10 Python tests, strict Clippy/format checks, original
text/PNG/PDF/MP4 export with matching hashes, ordinary C: writes and independent
readers, cancellation/tamper rejection, and native legacy-vault migration plus
the actual Windows export wrapper. Only regular-file images were attached to QEMU; C: testing used ordinary files.

This remains an unsigned, firmware-backed engineering preview, with no standalone
kernel isolation, independent security audit or classified-data approval. Three
failed native unlocks still erase the boot medium. Exported plaintext, old VM
backups and prior clones remain independent copies. Read README.md, SECURITY.md,
HOW TO EXPORT FILES.txt and the verification records before use.

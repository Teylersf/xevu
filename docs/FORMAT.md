# XEVU experimental envelope version 1

XEVU retains the original version-1 NULLGRAV identifiers and key-derivation
namespace to read existing vaults. These are compatibility names.

All integers are little-endian. This format is publicly specified to enable
review and recovery tooling. It is an unaudited composition of existing
primitives. It is not claimed to be novel, approved, or quantum resistant.

## VAULT.NGK — exactly 128 bytes

| Offset | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `NGKEY001` |
| 8 | 4 | Version 1 |
| 12 | 4 | Argon2id memory cost in KiB (writer: 65536) |
| 16 | 4 | Argon2id iteration count (writer: 3) |
| 20 | 4 | Argon2id lanes (1) |
| 24 | 16 | Random salt |
| 40 | 24 | Random XChaCha20 nonce |
| 64 | 48 | Encrypted 32-byte master key and 16-byte authentication tag |
| 112 | 16 | Reserved; must be zero |

Argon2id v1.3 produces a 32-byte wrapping key. The first 64 bytes are AAD when
wrapping/unwrapping the master key. Nonzero reserved bytes are rejected. There
is no user identity, default key, server key, or recovery key in this record.

## Native retry journal — two 512-byte raw sectors

The envelope remains version 1. Native boot release 0.2 adds a separate journal
at LBAs 16 and 17 in the MBR gap. It is not encrypted or tamper resistant.

| Offset | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `NGTRY001` |
| 8 | 8 | Monotonically increasing committed generation, initially 1 |
| 16 | 1 | Consecutive failed authentications, 0–3; 3 triggers erasure |
| 17 | 15 | Reserved zero bytes |
| 32 | 32 | SHA-256 of the original 128-byte VAULT.NGK |
| 64 | 32 | SHA-256 of bytes 0–63 |
| 96 | 416 | Reserved zero bytes |

The highest valid generation wins. Two valid records must have the same key
hash, and equal generations must agree. Both entirely zero means unarmed; two
invalid nonblank slots fail closed. Normal updates alternate slots; arming and
the irreversible trigger write both. Flush and readback precede acceptance.
Successful authentication commits a zero count. Full sequence and limitations:
[ERASURE.md](ERASURE.md).

## Object .NGV header — 64 bytes

| Offset | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `NGFILE01` |
| 8 | 16 | Random file ID |
| 24 | 16 | Independent random nonce prefix |
| 40 | 8 | Data chunk count |
| 48 | 4 | Chunk plaintext length, fixed at 65536 |
| 52 | 12 | Reserved; must be zero |

HKDF-SHA256 uses the master key as IKM, the 16-byte file ID as salt, and ASCII
`NULLGRAV/FILE/v1` as info, producing a 32-byte object key.

For index `i`, the nonce is the 16-byte nonce prefix followed by uint64 `i`.
AAD is the complete 64-byte object header followed by uint64 `i`. A writer must
never reuse an index; modifying an object requires a fresh file ID and nonce
prefix. There is no in-place update feature.

## Metadata record — 4112 encrypted bytes

Index 0 authenticates/encrypts 4096 plaintext bytes plus a 16-byte tag:

| Offset | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | Original plaintext size |
| 8 | 2 | UTF-8 basename byte length, 1–240 |
| 10 | variable | Original UTF-8 basename |
| remaining | variable | Zero padding |

Separators, colon, controls, dot, and dot-dot names are rejected. Metadata and
data chunk counts must agree. Filename authenticity does not establish the
real-world identity of the sender; no sender signature exists in this version.

## Data records

Indices 1 through chunk count each encrypt 65536 plaintext bytes plus a 16-byte
tag. The last plaintext chunk is zero padded; the authenticated metadata gives
the real length. Zero-length files contain metadata but no data records.

The exact encrypted object size is:

`64 + 4112 + chunk_count * 65552`

Reject any mismatch before allocating a plaintext file buffer. The native viewer
limits plaintext to 128 MiB; the host CLI streams chunks. There is no encrypted
directory manifest, anti-rollback counter, or authenticated cross-file membership.

## NGMJPG01 — plaintext converted-video payload

Eight-byte magic `NGMJPG01`, uint32 frame rate (1–30), uint32 frame count (1–10000),
then for each frame uint32 JPEG size and that many JPEG bytes. Frames are limited
to 2 MiB and the full native object to 128 MiB. This format is silent. It becomes
encrypted only after import as an NGV object. This is a convenience format, not
a new video codec or cryptographic mechanism.

## NGDOC001 — plaintext converted-document payload

Uses the same bounded frame records with eight-byte magic `NGDOC001`, frame-rate
field 1, and one JPEG per rendered PDF page. The native viewer disables automatic
advance and uses left/right navigation. This is a page-image container, not a PDF
renderer. It is plaintext until wrapped inside an NGV object.

## Plaintext export and handoff receipts (0.4)

Export authenticates the complete NGV object and writes its original payload,
without decoding or conversion, under `NULLGRAV_EXPORTS/<original basename>` on
the selected writable removable FAT volume. The boot medium and internal or
read-only media are excluded. Windows device names and incompatible basenames
are rejected; an existing destination file is never replaced.

After staging, renaming and SHA-256 readback of the final file, the native writer
creates `NG_EXPORT_LOG/<random identifier>.JSON` containing a UTF-8 JSON object:

```json
{"version":1,"name":"Research.pdf","bytes":1234,"sha256":"<64 lowercase hex digits>"}
```

The Windows companion waits for guest shutdown, reads only this dedicated FAT
bridge image and validates the bounded receipts and complete file hashes before
writing ordinary files into the selected Windows folder. Receipts are limited
to 4096 bytes, payloads to 128 MiB each and the batch to 256 MiB. These unkeyed
hashes check transfer consistency; they do not authenticate a sender. The helper
does not receive a vault key. Original timestamps, permissions and alternate
streams are not part of the encrypted envelope or restored payload.

# NULLGRAV experimental envelope version 1

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

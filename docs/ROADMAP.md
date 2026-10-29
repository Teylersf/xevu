# Remaining work toward the original full-OS security goal

The current UEFI environment makes the interaction and encryption format
reviewable. It does not fulfill the full standalone-OS or classified-data goal.

| Stage | Required result | Current state |
| --- | --- | --- |
| Native preview | Boot image, UI, encrypted vault, controlled import, native limited viewers, clone verification | Implemented; emulator evidence in VERIFICATION.md |
| J3455 validation | Boot the exact release on the real machine; identify firmware, reader, keyboard, pointer and display behavior | Hardware/media access needed |
| Standalone kernel | ExitBootServices, owned memory map/page tables, scheduling, interrupt/exception handling, trusted entropy and allocator | Not implemented |
| Device enforcement | Native xHCI/HID/storage, AHCI and relevant SDHCI support; default-deny capabilities; DMA protections on supported hardware | Not implemented |
| Isolated viewers | Process separation and per-file handles; no viewer access to keys, raw disks, other objects, or networking | Not implemented |
| General media | Port a mature codec stack/libVLC and its runtime/graphics/audio dependencies; supported-format and malformed-input testing | Not implemented |
| General documents | Isolated PDF/Office rendering with macros/networking disabled; Unicode shaping and fonts | Converted page images only |
| Boot/update trust | Signed releases, verifiable boot chain, offline update trust and rollback policy, reproducible-build verification | Unsigned preview |
| Cryptographic assurance | Independent review of the envelope, authenticated vault membership, rollback/freshness design, recovery and rekey procedures | Existing primitives; unaudited composition |
| Research track | Prior-art review and formal models for any proposed novel storage/access scheme; independent cryptanalysis before protecting sensitive content | No novelty/security claim |
| Operational assurance | Fuzzing, physical-device tests, fault injection, power-loss/corruption recovery, external security review | Limited automated/emulator coverage |

The core cryptographic algorithm is deliberately not invented for this release.
Lattice-based recipient key establishment, threshold recovery and signed manifests
are possible future studies; none is silently claimed or enabled here. A new
mathematical construction must have a precise threat model and public analysis.

Government-classified deployment additionally needs the responsible organization's
approved architecture and accreditation; a public repository cannot confer them.

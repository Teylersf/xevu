# XEVU

**Carry what matters.**

Pronounced **zeh-voo**. Written **XEVU**, without a forced acronym. The category
descriptor is **offline file workspace**. The name is intentionally abstract:
the files and the people using it define its purpose.

## Position

An open-source, portable workspace for carrying encrypted files and choosing
exactly when to import, view, export or clone them. The same identity works for
creative projects, business documents, research, education and personal archives.
These are intended contexts, not claims of regulatory approval or readiness to
hold sensitive production information. The current build is an unaudited preview.

Primary line: **Carry what matters.**

Supporting line: **Encrypted files. Explicit transfers. A workspace you can carry.**

Product sentence: **XEVU is an open-source, offline file workspace with encrypted
storage, explicit drive access and original-file export.**

## Identity

The **Crosspoint** mark consists of four paths around an open center. It suggests
movement with a deliberate boundary. The geometric wordmark is original vector
work; it uses no font dependency. Use the supplied paths without redrawing them.

| Color | Hex | Use |
| --- | --- | --- |
| Graphite | `#0B1014` | Main background, dark logo on light surfaces |
| Panel | `#121A20` | Raised surfaces |
| Line | `#2B383E` | Quiet dividers |
| Ivory | `#EDF1E8` | Main typography and reversed logo |
| Muted | `#A0AEA8` | Supporting typography |
| Lime | `#C8FF76` | Mark, selected actions, small accents |

Headlines: Barlow Semi Condensed SemiBold. Labels and technical details: IBM Plex
Mono. The existing fonts retain their OFL licenses in `assets/fonts/`.

Use the mark at 24 px or larger, with clear space of at least one quarter of its
height. Keep it monochrome: lime or ivory on dark backgrounds, graphite on light.
Do not stretch, outline, rotate, apply a glow or fill the center. Preserve strong
contrast. The SVGs are editable; the transparent PNGs are ready for direct use.

## Art direction

Tactile graphite and titanium, abstract folded forms, controlled light and one
lime edge. Quietly futuristic, precise and useful. Technical motion stays in the
native Signal Lab and can be disabled. No military affiliation, aerospace focus,
weapons, rockets, padlock stock art or invented compliance seals.

## Voice and claims

Speak plainly. Say what a person can do and what happens to their files. Prefer
"encrypted copy" and "original-file export" to "unbreakable" or "military grade."

Supported: public source; no native network stack or telemetry; authenticated
encrypted contents and filenames; explicit source selection; verified export of
original filenames and payload bytes. Exported copies are ordinary readable files.

Always retain the preview qualifier in standalone marketing. This remains a
firmware-backed UEFI environment, with limited viewers and filesystems, no
standalone kernel isolation, no independent audit, and no classified-data or
regulatory approval. **Three failed native unlocks erase the boot medium.**
Use test data. Native destinations are removable FAT; C: output uses the Windows
companion. Never imply that novel branding changes those security boundaries.

## Deliverables

- `assets/brand/`: original SVG mark and wordmark, transparent PNGs, identity JSON.
- `marketing/xevu-launch-1600x900.png`: master launch banner.
- `marketing/xevu-social-1200x630.png`: link/share card.
- `marketing/xevu-social-1080x1080.png`: square social artwork.
- `marketing/xevu-social-1080x1350.png`: portrait social artwork.
- `marketing/xevu-brand-board.png`: identity overview.
- `output/pdf/xevu-product-sheet.pdf`: one-page product handout.
- `output/pdf/xevu-brand-guide.pdf`: three-page identity and messaging guide.
- `marketing/COPY.md`: reusable launch, social and product copy.
- `marketing/NAME_RESEARCH.md`: limited name-screening notes.

The generated abstract artwork and its exact prompt are documented in
`assets/ARTWORK.md`. The vector identity and layouts are reproducible with
`python tools/brand_assets.py --pdf` (Pillow, ReportLab and PyMuPDF required).
Runtime firmware assets are prepared separately by `tools/prepare_assets.py`.

## Compatibility

XEVU continues the earlier NULLGRAV preview. Envelope magics, `.NGV`/`.NGK`
extensions, `NULLGRAV/FILE/v1`, `NULLGRAV/VAULT` and `NULLGRAV_EXPORTS` remain
version-1 compatibility identifiers. They are not current product messaging.
Changing them during a visual rebrand would make existing encrypted files
unreadable. The upgrade tool retains passwords, ciphertext and retry records.

XEVU 0.5.0 is the general-purpose rebrand of the earlier NULLGRAV preview.

XEVU (zeh-voo) / Carry what matters.

A new original Crosspoint logo and geometric wordmark, graphite/ivory/lime
identity, abstract boot artwork and native UI replace the antigravity-specific
positioning. The workspace now speaks to creative work, business documents,
research, education and personal files. The public repository is Teylersf/xevu.

The marketing kit includes six editable SVG variants, six transparent PNG logos,
a 1600x900 launch banner, 1200x630 / 1080x1080 / 1080x1350 social artwork, an
identity board, launch copy, a one-page product PDF and a three-page brand guide.
Source geometry, the built-in imagegen prompt, font licenses and naming notes
are included. Public name screening is preliminary; trademark/domain clearance
is not claimed.

The native boot image, Windows launchers and public host CLI use XEVU branding;
the CLI executable is xevuctl.exe. UPGRADE XEVU VM.cmd preserves an existing
0.3/0.4 vault, password, ciphertext and both retry slots after the earlier VM
stops. Old source-checkout launcher names remain aliases. On-disk format-v1
identifiers and key derivation remain unchanged so existing files stay readable.

Validation: 17 Rust tests, 10 Python helper tests, strict host/UEFI Clippy and
formatting checks, PowerShell parsing, and a real boot of the exact 0.5.0 image.
A native 0.4.0 fixture vault was migrated, unlocked with its original passphrase,
and exported with its exact original filename and SHA-256. The original source
image was unchanged. The new Windows helper also saved and verified that file
in a normal C: folder. Marketing PNGs, true-alpha logo PNGs and all PDF pages
were rendered and reviewed. Verification evidence is in docs/VERIFICATION.md.

This remains an unsigned, unaudited firmware-backed UEFI engineering preview,
not a completed standalone OS. Use test data. Three failed native unlocks erase
the boot medium. Native viewing/filesystem support is limited; native export
uses removable FAT, while C: output uses the Windows companion. Branding adds
no audit, regulatory/classified approval, isolation or hardware validation.

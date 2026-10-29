# XEVU artwork and identity provenance

The master `xevu-artwork.png` was generated with the built-in imagegen tool on
2026-10-29. `wallpaper.rgbz` is its 1280x720 RGB build resource. Marketing layouts
embed the same artwork. It is an abstract brand sculpture, not manufactured
hardware or evidence of any physical research.

The Crosspoint mark and XEVU geometric wordmark are original code-native vector
designs. `assets/brand/identity.json` is the geometry/color source;
`tools/brand_assets.py` renders SVG/transparent PNG variants and the matching
native polygon resource. Typography is rendered separately for crisp text.

## Exact imagegen prompt

Use case: ads-marketing. Asset type: master landscape artwork for the XEVU general-purpose offline file workspace, to appear behind a native boot screen and in launch materials. Generate a premium abstract studio artwork, wide 16:9 composition, 1536 by 864 pixels if supported. A single sculptural open aperture made of two precisely folded, interlocking graphite and satin-titanium ribbons, subtly suggesting crossing paths and a protected inner space, suspended on the RIGHT HALF against a near-black graphite void. Quiet chartreuse light (#c8ff76) travels along only one inner edge; cool ivory highlights, deep soft shadows, meticulous manufactured surfaces, subtle fine grain, restrained photographic realism, sophisticated industrial design. Left 48 percent must be very dark and largely empty for crisp separately rendered text. Generous breathing room, single memorable silhouette, no clutter. Broad creative and business appeal with a futuristic technical feel. This is abstract brand art, not a real manufactured product. No text, no letters, no logos, no watermarks, no padlocks, no shields, no rockets, no aircraft, no weapons, no military insignia, no antigravity or science-fiction laboratory scene.

## Reproduction

`python tools/brand_assets.py --pdf` renders vector identity and marketing
materials. `python tools/prepare_assets.py` prepares native art/font resources.
`assets/manifest.json` and `marketing/manifest.json` record resource hashes.
The original prompt and built-in tool mode are retained here for review.

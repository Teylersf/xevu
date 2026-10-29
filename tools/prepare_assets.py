"""Prepare compact native UI resources; generated art is never fetched at boot."""
from pathlib import Path
import hashlib, json, struct, urllib.request, zlib
from PIL import Image, ImageDraw, ImageFont
ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
FONTS = ASSETS / "fonts"
FONTS.mkdir(exist_ok=True)
SOURCES = {
    "BarlowSemiCondensed-SemiBold.ttf": "https://raw.githubusercontent.com/google/fonts/main/ofl/barlowsemicondensed/BarlowSemiCondensed-SemiBold.ttf",
    "OFL-Barlow.txt": "https://raw.githubusercontent.com/google/fonts/main/ofl/barlowsemicondensed/OFL.txt",
    "IBMPlexMono-Regular.ttf": "https://raw.githubusercontent.com/google/fonts/main/ofl/ibmplexmono/IBMPlexMono-Regular.ttf",
    "OFL-IBMPlexMono.txt": "https://raw.githubusercontent.com/google/fonts/main/ofl/ibmplexmono/OFL.txt",
}
for name, url in SOURCES.items():
    path = FONTS / name
    if not path.exists():
        urllib.request.urlretrieve(url, path)
for size in [12, 14, 16, 20, 28, 40, 64]:
    name = "IBMPlexMono-Regular.ttf" if size <= 16 else "BarlowSemiCondensed-SemiBold.ttf"
    font = ImageFont.truetype(str(FONTS / name), size)
    cw, ch = size + 6, size + 10
    advances = bytes(min(255, round(font.getlength(chr(c)))) for c in range(32, 127))
    pixels = bytearray()
    for c in range(32, 127):
        im = Image.new("L", (cw, ch))
        ImageDraw.Draw(im).text((0, 0), chr(c), font=font, fill=255, anchor="la")
        pixels.extend(im.tobytes())
    packed = struct.pack("<HHH", size, cw, ch) + advances + pixels
    (ASSETS / f"font-{size}.bin").write_bytes(zlib.compress(packed, 9))
im = Image.open(ASSETS / "nullgrav-wallpaper.png").convert("RGB").resize((1280, 720), Image.Resampling.LANCZOS)
(ASSETS / "wallpaper.rgbz").write_bytes(zlib.compress(im.tobytes(), 9))
manifest = {"fonts": SOURCES, "art": "Built-in imagegen; prompt in assets/ARTWORK.md", "sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in ASSETS.iterdir() if p.suffix in [".bin", ".rgbz", ".png"]}}
(ASSETS / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print("Prepared native artwork and seven antialiased font atlases.")

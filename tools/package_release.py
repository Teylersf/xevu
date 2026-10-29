"""Package only public, clean release artifacts. No test vaults or local state."""
from pathlib import Path
import hashlib,json,zipfile
ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'dist'
files=['dist/nullgrav-0.1.0-x86_64.img','dist/nullgrav-0.1.0-x86_64.json','dist/nullgrav-0.1.0-x86_64.sha256','dist/BOOTX64.EFI','dist/fieldctl.exe','README.md','SECURITY.md','LICENSE','FLASH NULLGRAV.cmd','tools/write-media.ps1','tools/flash-menu.ps1','tools/prepare_video.py','tools/prepare_document.py','docs/FORMAT.md','docs/VERIFICATION.md','docs/images/dashboard.png']
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
checksums=''.join(digest(ROOT/name)+'  '+name+'\n' for name in files)
archive=DIST/'nullgrav-0.1.0-preview.zip'
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for name in files:z.write(ROOT/name,name)
    z.writestr('SHA256SUMS.txt',checksums)
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    for name in files:
        h=hashlib.sha256()
        with z.open(name) as f:
            for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
        assert h.hexdigest()==digest(ROOT/name),name
(DIST/'SHA256SUMS.txt').write_text(checksums+digest(archive)+'  '+archive.name+'\n')
receipt={'archive':archive.name,'bytes':archive.stat().st_size,'sha256':digest(archive),'file_count':len(files)+1,'zip_crc_and_payload_sha256_verified':True,'contains_test_vault':False}
(DIST/'release-verification.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))

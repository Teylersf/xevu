"""Package only public, clean release artifacts. No test vaults or local state."""
from pathlib import Path
import hashlib,json,zipfile
ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'dist'
files=['dist/nullgrav-0.3.0-x86_64.img','dist/nullgrav-0.3.0-x86_64.json','dist/nullgrav-0.3.0-x86_64.sha256','dist/BOOTX64.EFI','dist/fieldctl.exe','README.md','SECURITY.md','LICENSE','THIRD_PARTY_NOTICES.md','assets/fonts/OFL-Barlow.txt','assets/fonts/OFL-IBMPlexMono.txt','assets/nullgrav-mark.svg','FLASH NULLGRAV.cmd','tools/write-media.ps1','tools/flash-menu.ps1','tools/prepare_video.py','tools/prepare_document.py','docs/FORMAT.md','docs/VERIFICATION.md','docs/ROADMAP.md','docs/images/dashboard.png','docs/images/boot.png']
files += ['TEST NULLGRAV VM.cmd','tools/start-vm.ps1','tools/qemu.py','tools/build_image.py',
          'tools/verify_purge.py','docs/ERASURE.md','docs/purge-verification.json',
          'docs/images/erase-warning.png','docs/images/erase-progress.png',
          'docs/purge-resume-verification.json','docs/dependency-audit.json',
          'docs/clone-verification.json','docs/RELEASE_NOTES.md','assets/ARTWORK.md']
files += ['tools/verify_signals.py','docs/signals-verification.json','docs/images/signals.png',
          'docs/purge-0.3.0-verification.json']
files += ['ADD FILES TO NULLGRAV VM.cmd','HOW TO ADD FILES.txt','tools/add-vm-files.ps1',
          'tools/vm_import.py','tools/test_vm_import.py','tools/verify_vm_import.py',
          'docs/vm-import-verification.json','docs/images/windows-file-import.png']
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
checksums=''.join(digest(ROOT/name)+'  '+name+'\n' for name in files)
archive=DIST/'nullgrav-0.3.1-preview.zip'
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

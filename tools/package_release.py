"""Package only public, clean release artifacts. No test vaults or local state."""
from pathlib import Path
import hashlib,json,zipfile
ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'dist'
files=['dist/xevu-0.5.0-x86_64.img','dist/xevu-0.5.0-x86_64.json','dist/xevu-0.5.0-x86_64.sha256','dist/xevu-BOOTX64.EFI','dist/xevuctl.exe','README.md','SECURITY.md','LICENSE','THIRD_PARTY_NOTICES.md','assets/fonts/OFL-Barlow.txt','assets/fonts/OFL-IBMPlexMono.txt','assets/brand/xevu-mark-lime.svg','FLASH XEVU.cmd','tools/write-media.ps1','tools/flash-menu.ps1','tools/prepare_video.py','tools/prepare_document.py','docs/FORMAT.md','docs/VERIFICATION.md','docs/ROADMAP.md','docs/images/dashboard.png','docs/images/boot.png']
files += ['TEST XEVU VM.cmd','tools/start-vm.ps1','tools/qemu.py','tools/build_image.py',
          'tools/verify_purge.py','docs/ERASURE.md','docs/purge-verification.json',
          'docs/images/erase-warning.png','docs/images/erase-progress.png',
          'docs/purge-resume-verification.json','docs/dependency-audit.json',
          'docs/clone-verification.json','docs/RELEASE_NOTES.md','assets/ARTWORK.md']
files += ['tools/verify_signals.py','docs/signals-verification.json','docs/images/signals.png',
          'docs/purge-0.3.0-verification.json']
files += ['ADD FILES TO XEVU VM.cmd','HOW TO ADD FILES.txt','tools/add-vm-files.ps1',
          'tools/vm_import.py','tools/test_vm_import.py','tools/verify_vm_import.py',
          'docs/vm-import-verification.json','docs/images/windows-file-import.png']
files += ['SAVE FILES TO WINDOWS.cmd','HOW TO EXPORT FILES.txt','UPGRADE XEVU VM.cmd',
          'tools/save-vm-files.ps1','tools/vm_export.py','tools/fat_reader.py','tools/vm_session.py',
          'tools/upgrade_vm.py','tools/upgrade-vm.ps1','tools/test_vm_export.py',
          'tools/verify_export.py','docs/export-verification.json','docs/windows-copy-verification.json',
          'docs/upgrade-export-verification.json','docs/images/export-confirmation.png','docs/images/export-complete.png']
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
checksums=''.join(digest(ROOT/name)+'  '+name+'\n' for name in files)
files += ['docs/rebrand-verification.json','marketing/BRAND.md','marketing/COPY.md',
          'marketing/NAME_RESEARCH.md','marketing/xevu-brand-board.png','marketing/xevu-launch-1600x900.png',
          'output/pdf/xevu-product-sheet.pdf','output/pdf/xevu-brand-guide.pdf']
files += ['docs/images/xevu-boot.png','docs/images/xevu-dashboard.png','docs/images/xevu-signals.png']
for folder in ['assets/brand','marketing','output/pdf']:
    for path in sorted((ROOT/folder).rglob('*')):
        name=path.relative_to(ROOT).as_posix()
        if path.is_file() and name not in files:files.append(name)
files += ['assets/xevu-artwork.png','assets/fonts/BarlowSemiCondensed-SemiBold.ttf',
          'assets/fonts/IBMPlexMono-Regular.ttf']
archive=DIST/'xevu-0.5.0-preview.zip'
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

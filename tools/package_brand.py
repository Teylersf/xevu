"""Package the XEVU identity/marketing kit without any OS image or vault data."""
from pathlib import Path
import hashlib,json,zipfile

ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'dist'

def digest(path):
    with path.open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
    files=[ROOT/p for p in ['LICENSE','assets/ARTWORK.md','assets/xevu-artwork.png',
        'assets/fonts/BarlowSemiCondensed-SemiBold.ttf','assets/fonts/IBMPlexMono-Regular.ttf',
        'assets/fonts/OFL-Barlow.txt','assets/fonts/OFL-IBMPlexMono.txt',
        'tools/brand_assets.py','tools/prepare_assets.py']]
    for folder in ['assets/brand','marketing','output/pdf']:
        files.extend(p for p in sorted((ROOT/folder).rglob('*'))if p.is_file())
    names=[p.relative_to(ROOT).as_posix()for p in files]
    checksums=''.join(digest(p)+'  '+name+'\n'for p,name in zip(files,names))
    DIST.mkdir(exist_ok=True);archive=DIST/'xevu-brand-kit.zip'
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9)as z:
        for p,name in zip(files,names):z.write(p,name)
        z.writestr('SHA256SUMS.txt',checksums)
    with zipfile.ZipFile(archive)as z:
        assert z.testzip()is None
        for p,name in zip(files,names):
            assert hashlib.sha256(z.read(name)).hexdigest()==digest(p),name
    receipt={'brand':'XEVU','archive':archive.name,'bytes':archive.stat().st_size,
        'sha256':digest(archive),'file_count':len(names)+1,'zip_crc_and_payload_sha256_verified':True,
        'contains_os_image_or_vault':False}
    (DIST/'brand-kit-verification.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(receipt,indent=2))

if __name__=='__main__':main()

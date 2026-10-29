"""Read selected host files into a new, read-only virtual transfer drive.

Never opens a physical disk or writes a boot/vault image. Transfer images contain
plaintext copies and stay in ignored test-output/windows-imports until removed.
"""
from pathlib import Path
import argparse, hashlib, json, os, stat, subprocess, sys, uuid
from build_image import FatImage
import qemu,vm_session

ROOT=Path(__file__).resolve().parents[1]
MAX_FILE=128*1024*1024
MAX_BATCH=200*1024*1024
MAX_FILES=200

def selected_files(files):
    if not files or len(files)>MAX_FILES:
        raise ValueError(f'Select 1..{MAX_FILES} files per transfer.')
    selected=[];names=set();total=0
    for value in files:
        p=Path(value).resolve(strict=True)
        if not p.is_file():raise ValueError('Choose ordinary files, not folders or devices.')
        name=p.name
        if len(name.encode('utf-8'))>240 or name in ('.','..') or name.endswith((' ','.')) or any(ord(c)<32 or ord(c)==127 or c in '/\\:' for c in name):
            raise ValueError(f'Filename is not supported by the native vault: {name}')
        if name.casefold() in names:raise ValueError('Two selected files have the same name. Rename one or transfer them separately.')
        names.add(name.casefold())
        size=p.stat().st_size
        if size>MAX_FILE:raise ValueError(f'{name} exceeds the native 128 MiB per-file limit.')
        total+=size
        if total>MAX_BATCH:raise ValueError('Select at most 200 MiB per transfer. The preview vault holds about 250 MiB total.')
        selected.append((p,size))
    return selected,total

def build_transfer(files, output_dir=None):
    selected,total=selected_files(files)
    directory=(output_dir or ROOT/'test-output/windows-imports').resolve()
    allowed=(ROOT/'test-output').resolve()
    if directory!=allowed and allowed not in directory.parents:
        raise ValueError('Transfer output must stay inside this project\'s test-output folder.')
    directory.mkdir(parents=True,exist_ok=True)
    image=directory/('windows-'+uuid.uuid4().hex+'.img')
    # New UUID-named regular file only. Never reuse a caller-supplied image/device.
    with image.open('xb'):pass
    size_mib=max(64,((total+32*1024*1024+64*1024*1024-1)//(64*1024*1024))*64)
    fat=FatImage(size_mib);inventory=[]
    try:
        for p,size in selected:
            data=p.read_bytes()
            if len(data)!=size:raise ValueError('A source file changed size while copying; retry after it finishes saving.')
            fat.add(p.name,data)
            inventory.append({'name':p.name,'bytes':size,'sha256':hashlib.sha256(data).hexdigest()})
        image_hash=fat.write(image)
        receipt={'image':str(image),'image_sha256':image_hash,'file_count':len(inventory),
            'payload_bytes':total,'files':inventory,'plaintext_transfer_copy':True,'read_only_attachment':True}
        image.with_suffix('.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
        return receipt
    except BaseException:
        image.unlink(missing_ok=True)
        raise

def safe_image(image):
    p=Path(image).resolve(strict=True)
    allowed=(ROOT/'test-output').resolve()
    if allowed not in p.parents or p.suffix.lower()!='.img' or not stat.S_ISREG(p.stat().st_mode):
        raise ValueError('Only a regular transfer .img file inside test-output can be attached.')
    return p

def attach_transfer(image, session_dir=None):
    image=safe_image(image)
    directory=Path(session_dir).resolve() if session_dir else vm_session.running_session()
    if (ROOT/'test-output').resolve() not in directory.parents:
        raise ValueError('Use an XEVU test-output VM session.')
    qemu.OUT=directory;qemu.STATE=directory/'qemu-session.json'
    try:qemu.command('query-status')
    except (OSError,ValueError,FileNotFoundError) as e:
        raise ValueError('Start TEST XEVU VM.cmd first, then add files. Your existing vault will be kept.') from e
    # New devices only: never detach a source that the user may be reading.
    devices=qemu.command('query-block')
    if len(devices)>=10:raise ValueError('Several transfer drives are attached. Shut down with Q and restart the VM, then retry. The vault is retained.')
    token=uuid.uuid4().hex[:12]
    file_node='ngfile_'+token;raw_node='ngraw_'+token;device='ngimport_'+token
    file_added=False;raw_added=False
    try:
        qemu.command('blockdev-add',{'driver':'file','node-name':file_node,'filename':str(image),'read-only':True});file_added=True
        qemu.command('blockdev-add',{'driver':'raw','node-name':raw_node,'file':file_node,'read-only':True});raw_added=True
        qemu.command('device_add',{'driver':'usb-storage','id':device,'bus':'xhci.0','drive':raw_node,'removable':True,'serial':'NGIMPORT-'+token})
    except BaseException:
        if raw_added:
            try:qemu.command('blockdev-del',{'node-name':raw_node})
            except Exception:pass
        if file_added:
            try:qemu.command('blockdev-del',{'node-name':file_node})
            except Exception:pass
        raise
    # Retain only the latest transfer on the next boot; prior image copies stay
    # on disk, and existing drives in this running VM are never removed.
    saved={'image':str(image),'node':raw_node,'device':device,'read_only':True}
    temp=directory/('last-windows-import-'+token+'.tmp')
    temp.write_text(json.dumps(saved,indent=2)+'\n',encoding='utf-8')
    temp.replace(directory/'last-windows-import.json')
    return saved

def windows_to_wsl(path):
    result=subprocess.run(['wsl.exe','-d','Ubuntu','--','wslpath','-a',str(path).replace('\\','/')],capture_output=True,text=True,check=True)
    return result.stdout.strip()

def main():
    p=argparse.ArgumentParser()
    sub=p.add_subparsers(dest='action',required=True)
    load=sub.add_parser('load');load.add_argument('--manifest',type=Path,required=True);load.add_argument('--session-dir',type=Path)
    attach=sub.add_parser('attach');attach.add_argument('--image',type=Path,required=True);attach.add_argument('--session-dir',type=Path)
    a=p.parse_args()
    try:
        if a.action=='attach':
            print(json.dumps(attach_transfer(a.image,a.session_dir)));return
        values=json.loads(a.manifest.read_text(encoding='utf-8-sig'))
        files=values.get('files')
        if not isinstance(files,list) or any(not isinstance(v,str) for v in files):raise ValueError('Invalid file selection.')
        receipt=build_transfer(files)
        print(f'Prepared {receipt["file_count"]} file(s), {receipt["payload_bytes"]:,} bytes. Attaching read-only...',flush=True)
        if os.name=='nt':
            args=['wsl.exe','-d','Ubuntu','--','python3',windows_to_wsl(ROOT/'tools/vm_import.py'),
                'attach','--image',windows_to_wsl(Path(receipt['image']))]
            if a.session_dir:args+=['--session-dir',windows_to_wsl(a.session_dir)]
            subprocess.run(args,check=True)
        else:attach_transfer(receipt['image'],a.session_dir)
        print('Ready. In the VM: N/U, then 3, R, select the LAST non-boot volume, Enter; select your file, Enter. Press 2 to view the vault.')
    except Exception as e:
        print(f'Import setup failed: {e}',file=sys.stderr);raise SystemExit(1)

if __name__=='__main__':main()

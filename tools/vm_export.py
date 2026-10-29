"""Export bridge: guest decrypts; Windows receives verified ordinary files.

No vault key is accepted here. The bridge is read only after the VM shuts down.
"""
from pathlib import Path
import argparse,hashlib,json,os,re,stat,subprocess,sys,time,uuid
from build_image import FatImage
from fat_reader import FatReader
import qemu,vm_import,vm_session

ROOT=Path(__file__).resolve().parents[1]
MAX_FILE=128*1024*1024

def windows_name(name):
    if not isinstance(name,str) or not name or len(name.encode('utf-8'))>240 or name in ('.','..') or name.endswith((' ','.')):return False
    if any(ord(c)<32 or ord(c)==127 or c in '<>:"/\\|?*'for c in name):return False
    stem=name.split('.')[0].rstrip(' ').upper()
    if stem in ('CON','PRN','AUX','NUL','CONIN$','CONOUT$'):return False
    if any(stem.startswith(p)and stem[len(p):]in ('1','2','3','4','5','6','7','8','9','¹','²','³')for p in ('COM','LPT')):return False
    return True

def create_bridge(output_dir=None):
    directory=(output_dir or ROOT/'test-output/windows-exports').resolve()
    if (ROOT/'test-output').resolve()not in directory.parents:raise ValueError('Bridge must stay in test-output')
    directory.mkdir(parents=True,exist_ok=True)
    image=directory/('export-'+uuid.uuid4().hex+'.img')
    with image.open('xb'):pass
    fat=FatImage(256,label='WINDOWS_OUT');fat.mkdir('NULLGRAV_EXPORTS');fat.mkdir('NG_EXPORT_LOG')
    fat.write(image);return image

def attach_bridge(image,session_dir=None):
    image=vm_import.safe_image(image)
    directory=Path(session_dir).resolve()if session_dir else vm_session.running_session(require_current=True)
    if (ROOT/'test-output').resolve()not in directory.parents:raise ValueError('Invalid VM session')
    qemu.OUT=directory;qemu.STATE=directory/'qemu-session.json'
    qemu.command('query-status')
    token=uuid.uuid4().hex[:12];file_node='ngexportfile_'+token;node='ngexport_'+token;device='ngexportusb_'+token
    file_added=False;raw_added=False
    try:
        qemu.command('blockdev-add',{'driver':'file','node-name':file_node,'filename':str(image),'read-only':False});file_added=True
        qemu.command('blockdev-add',{'driver':'raw','node-name':node,'file':file_node,'read-only':False});raw_added=True
        qemu.command('device_add',{'driver':'usb-storage','id':device,'bus':'xhci.0','drive':node,'removable':True,'serial':'NGEXPORT-'+token})
    except BaseException:
        if raw_added:
            try:qemu.command('blockdev-del',{'node-name':node})
            except Exception:pass
        if file_added:
            try:qemu.command('blockdev-del',{'node-name':file_node})
            except Exception:pass
        raise
    return {'image':str(image),'node':node,'session_dir':str(directory),'read_only':False}

def committed_files(image):
    files={};total=0
    with FatReader(image)as fat:
        directory=fat.find(['NG_EXPORT_LOG'])
        if not directory.directory:raise ValueError('Export log is not a directory')
        for entry in fat.directory(directory.cluster):
            if entry.directory or not entry.name.upper().endswith('.JSON'):continue
            record=json.loads(fat.read_entry(entry,4096).decode('utf-8'))
            name=record.get('name');size=record.get('bytes');digest=record.get('sha256')
            if record.get('version')!=1 or not windows_name(name) or type(size)is not int or not 0<=size<=MAX_FILE or not isinstance(digest,str)or not re.fullmatch('[0-9a-f]{64}',digest):
                raise ValueError('Invalid export receipt; nothing will be saved')
            data=fat.read(['NULLGRAV_EXPORTS',name],MAX_FILE)
            if len(data)!=size or hashlib.sha256(data).hexdigest()!=digest:raise ValueError('Export file does not match its receipt; nothing will be saved')
            key=name.casefold()
            if key in files and files[key][0]!=name:raise ValueError('Ambiguous output filenames')
            files[key]=(name,data,digest);total+=len(data)
            if total>256*1024*1024:raise ValueError('Export batch exceeds bounds')
    return list(files.values())

def save_exports(image,destination):
    # Authenticate the complete batch against its native readback receipts before
    # any destination writes. Never interpret a receipt name as a path/device.
    files=committed_files(image)
    if not files:raise ValueError('No completed exports. In the VM select a vault file, press E, choose WINDOWS_OUT and type EXPORT.')
    directory=Path(destination).resolve(strict=True)
    if not directory.is_dir():raise ValueError('Choose an existing destination folder')
    outputs=[]
    for name,data,digest in files:
        target=directory/name
        if target.resolve().parent!=directory:raise ValueError('Output escapes the selected folder')
        if target.exists():
            if not target.is_file()or target.is_symlink():
                raise FileExistsError('A different file already exists. Choose an empty folder; nothing was overwritten.')
            with target.open('rb')as check:
                if hashlib.file_digest(check,'sha256').hexdigest()!=digest:raise FileExistsError('A different file already exists. Choose an empty folder; nothing was overwritten.')
        outputs.append((target,data,digest))
    saved=[]
    for target,data,digest in outputs:
        if target.exists():
            if target.is_symlink():raise ValueError('Output is a link, not a normal file')
            with target.open('rb')as check:
                if hashlib.file_digest(check,'sha256').hexdigest()!=digest:raise FileExistsError('Output changed; nothing was overwritten')
        else:
            created=False
            try:
                with target.open('xb')as f:
                    created=True;f.write(data);f.flush();os.fsync(f.fileno())
                with target.open('rb')as check:
                    if hashlib.file_digest(check,'sha256').hexdigest()!=digest:raise ValueError('Windows output readback mismatch')
            except BaseException:
                if created:target.unlink(missing_ok=True)
                raise
        saved.append({'name':target.name,'bytes':len(data),'sha256':digest})
    return saved

def wait_for_shutdown(session):
    qemu.STATE=Path(session)/'qemu-session.json'
    while True:
        try:qemu.command('query-status')
        except(OSError,ValueError,FileNotFoundError):return
        time.sleep(.8)

def cleanup_bridge(image,receipt):
    image=Path(image);receipt=Path(receipt)
    if not re.fullmatch('export-[0-9a-f]{32}\\.img',image.name)or receipt!=image.with_suffix('.json'):
        raise ValueError('Refusing cleanup of anything except an owned export bridge')
    if image.exists():
        vm_import.safe_image(image)
        image.unlink()  # Delete the literal bridge/link, never its resolved target.
    receipt.unlink(missing_ok=True)

def main():
    p=argparse.ArgumentParser();sub=p.add_subparsers(dest='action',required=True)
    attach=sub.add_parser('attach');attach.add_argument('--image',type=Path,required=True);attach.add_argument('--receipt',type=Path,required=True);attach.add_argument('--session-dir',type=Path)
    wait=sub.add_parser('wait');wait.add_argument('--session-dir',type=Path,required=True)
    run=sub.add_parser('run');run.add_argument('--destination',type=Path,required=True);run.add_argument('--session-dir',type=Path)
    a=p.parse_args()
    try:
        if a.action=='wait':wait_for_shutdown(a.session_dir);return
        if a.action=='attach':
            result=attach_bridge(a.image,a.session_dir);a.receipt.write_text(json.dumps(result)+'\n');return
        destination=a.destination.resolve(strict=True)
        if not destination.is_dir():raise ValueError('Choose an existing Windows folder')
        image=create_bridge();receipt=image.with_suffix('.json')
        if os.name=='nt':
            args=['wsl.exe','-d','Ubuntu','--','python3',vm_import.windows_to_wsl(ROOT/'tools/vm_export.py'),
                'attach','--image',vm_import.windows_to_wsl(image),'--receipt',vm_import.windows_to_wsl(receipt)]
            if a.session_dir:args+=['--session-dir',vm_import.windows_to_wsl(a.session_dir)]
            subprocess.run(args,check=True)
            attached=json.loads(receipt.read_text())
        else:
            attached=attach_bridge(image,a.session_dir);receipt.write_text(json.dumps(attached)+'\n')
        print('WINDOWS_OUT is attached. In NULLGRAV:',flush=True)
        print('2 -> select a vault file -> E -> WINDOWS_OUT -> Enter -> type EXPORT -> Enter.',flush=True)
        print('Repeat for other files, then press Q to shut down NULLGRAV. Keep this window open.',flush=True)
        if os.name=='nt':
            subprocess.run(['wsl.exe','-d','Ubuntu','--','python3',vm_import.windows_to_wsl(ROOT/'tools/vm_export.py'),
                'wait','--session-dir',attached['session_dir']],check=True)
        else:wait_for_shutdown(attached['session_dir'])
        saved=save_exports(image,destination)
        # This is our own temporary bridge, not the user's source or C: copy.
        # The VM has released it and every delivered file was read back first.
        try:
            cleanup_bridge(image,receipt)
        except (OSError,ValueError):
            print(f'The plaintext transfer image may remain at {image}; Windows files were saved.',flush=True)
        print(f'Saved and SHA-256 verified {len(saved)} normal file(s) in {destination}. Originals remain encrypted in the vault.',flush=True)
    except Exception as e:
        print(f'Export failed: {e}',file=sys.stderr);raise SystemExit(1)

if __name__=='__main__':main()

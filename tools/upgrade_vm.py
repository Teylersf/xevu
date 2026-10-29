"""Copy an existing VM's encrypted vault and retry state into the new boot image.

The source VM must be stopped. Old disk files are retained; no physical devices.
"""
from pathlib import Path
import argparse,hashlib,json,struct,time
from build_image import FatImage
from fat_reader import FatReader
import qemu,vm_session

ROOT=Path(__file__).resolve().parents[1]

def current_running(directory):
    state=directory/'qemu-session.json'
    if not state.exists():return False
    info=json.loads(state.read_text(encoding='utf-8'))
    # QMP can close slightly before the process releases its disk handles. On
    # the WSL upgrade path, also keep waiting while this exact QEMU PID is alive.
    process=Path('/proc')/str(info.get('pid','invalid'))/'cmdline'
    if process.exists():
        try:
            command=process.read_bytes()
            if b'qemu-system-'in command and str(directory/'boot-test.img').encode()in command:return True
        except OSError:pass
    original=qemu.STATE
    try:
        qemu.STATE=state
        try:qemu.command('query-status');return True
        except(OSError,ValueError,FileNotFoundError):return False
    finally:qemu.STATE=original

def journal_record(raw):
    if len(raw)!=512 or raw[:8]!=b'NGTRY001' or not 0<=raw[16]<=3 or not struct.unpack_from('<Q',raw,8)[0]:return None
    if any(raw[17:32])or any(raw[96:])or hashlib.sha256(raw[:64]).digest()!=raw[64:96]:return None
    return struct.unpack_from('<Q',raw,8)[0],raw[16],raw[32:64]

def migrate(source_dir,destination_dir,efi):
    source_dir=Path(source_dir).resolve();destination_dir=Path(destination_dir).resolve()
    allowed=(ROOT/'test-output').resolve()
    if allowed not in source_dir.parents or allowed not in destination_dir.parents or source_dir==destination_dir:raise ValueError('Use distinct test-output VM folders')
    if current_running(source_dir):raise ValueError('Press Q in the old VM before upgrading; no files were changed')
    source=source_dir/'boot-test.img';dest=destination_dir/'boot-test.img'
    if dest.exists():raise FileExistsError('The updated VM already has a disk. Use TEST NULLGRAV VM.cmd; it was not replaced.')
    records=[];copied=[];total=0;header=None
    with FatReader(source)as old:
        journal=old.read_at(16*512,1024)
        for raw in [journal[:512],journal[512:]]:
            record=journal_record(raw)
            if record:records.append(record)
        if records:
            latest=max(records)
            if len(records)==2 and (records[0][2]!=records[1][2]or(records[0][0]==records[1][0]and records[0]!=records[1])):raise ValueError('Retry records conflict; original vault retained')
            header=old.read(['NULLGRAV','VAULT','VAULT.NGK'],128)
            if len(header)!=128 or hashlib.sha256(header).digest()!=latest[2]:raise ValueError('Vault key and retry records disagree; original retained')
            if latest[1]==3:raise ValueError('The old vault has a committed erasure trigger; migration refused')
        elif any(journal):raise ValueError('Retry records are damaged; original retained')
        image=FatImage(256)
        for path,entry in old.walk():
            relative='/'.join(path)
            if relative.casefold()=='efi/boot/bootx64.efi':continue
            data=old.read_entry(entry,256*1024*1024);total+=len(data)
            if total>254*1024*1024:raise ValueError('Too much source data to fit the new image')
            image.add(relative,data)
            copied.append((path,hashlib.sha256(data).hexdigest()))
        image.add('EFI/BOOT/BOOTX64.EFI',Path(efi).read_bytes())
    destination_dir.mkdir(parents=True,exist_ok=True)
    with dest.open('xb'):pass
    try:
        image.write(dest)
        with dest.open('r+b')as f:f.seek(16*512);f.write(journal);f.truncate(max(source.stat().st_size,image.size));f.flush()
        with FatReader(dest)as check:
            for path,digest in copied:
                if hashlib.sha256(check.read(list(path),256*1024*1024)).hexdigest()!=digest:raise ValueError('Migration readback mismatch')
            if check.read_at(16*512,1024)!=journal:raise ValueError('Retry state readback mismatch')
        last=source_dir/'last-windows-import.json'
        if last.exists():(destination_dir/last.name).write_bytes(last.read_bytes())
        result={'source_vm_retained':True,'copied_files':len(copied),'copied_bytes':total,
            'encrypted_files_readback_verified':True,'retry_state_preserved':True,'counter_reset':False,
            'native_version':vm_session.VERSION,'physical_devices_accessed':False}
        (destination_dir/'upgrade-verification.json').write_text(json.dumps(result,indent=2)+'\n')
        return result
    except BaseException:
        dest.unlink(missing_ok=True);raise

def main():
    p=argparse.ArgumentParser();p.add_argument('--wait',action='store_true');p.add_argument('--launch',action='store_true');a=p.parse_args()
    try:
        old=vm_session.interactive(vm_session.LEGACY_VERSION);new=vm_session.interactive()
        if a.wait and current_running(old):
            print('Press Q in the old NULLGRAV VM when ready. Its encrypted vault and retry count will be copied; the old disks stay as a backup.',flush=True)
            while current_running(old):time.sleep(1)
        result=migrate(old,new,ROOT/'dist/BOOTX64.EFI');print(json.dumps(result,indent=2),flush=True)
        if a.launch:
            qemu.OUT=new;qemu.STATE=new/'qemu-session.json';qemu.start(resume=True,display='gtk',memory_mib=4096)
    except Exception as e:
        print(f'Upgrade stopped: {e}',flush=True);raise SystemExit(1)

if __name__=='__main__':main()

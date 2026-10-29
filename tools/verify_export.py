"""End-to-end plaintext release using public fixtures and regular VM images."""
from pathlib import Path
import hashlib,json,shutil,struct,time
import qemu,vm_export
from fat_reader import FatReader

OUT=qemu.ROOT/'test-output/export-flow-vm'
PASS='public export test passphrase'

def digest(path):
    with path.open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def capture(label):
    p=OUT/(label+'.ppm');qemu.command('screendump',{'filename':str(p)})
    parts=p.read_bytes().split(b'\n',3);return parts[3]if len(parts)==4 and parts[1]==b'1280 720'else b''

def wait_for(check,label,timeout=100):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        if check():return
        time.sleep(.3)
    capture('failed-'+label);raise RuntimeError('Timeout: '+label)

def ready():
    rgb=capture('boot-candidate');return rgb and rgb[(170*1280+16)*3:(170*1280+16)*3+3]==bytes([200,255,118])

def stop():
    try:qemu.command('quit')
    except(OSError,ValueError):pass
    qemu.STATE.unlink(missing_ok=True);time.sleep(.5)

def objects():
    with FatReader(OUT/'boot-test.img')as fat:
        return {e.name:hashlib.sha256(fat.read_entry(e)).hexdigest()for e in fat.directory(fat.find(['NULLGRAV','VAULT']).cluster)if e.name.upper().endswith('.NGV')}

def main():
    qemu.OUT=OUT;qemu.STATE=OUT/'qemu-session.json'
    fixtures=OUT/'fixtures'
    if not fixtures.is_dir() or len(list(fixtures.iterdir()))!=4:raise RuntimeError('Prepare four public fixtures before this test')
    expected={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in fixtures.iterdir()}
    qemu.start(memory_mib=1024)
    try:
        wait_for(ready,'dashboard');time.sleep(.6)
        qemu.key('n');qemu.text(PASS);qemu.key('ret');qemu.text(PASS);qemu.key('ret')
        def armed():
            with (OUT/'boot-test.img').open('rb')as f:f.seek(8192);return f.read(8)==b'NGTRY001'
        wait_for(armed,'created');time.sleep(8)
        for i in range(4):
            qemu.key('3');qemu.key('r');time.sleep(.7)
            for _ in range(8):qemu.key('down')
            qemu.key('ret');time.sleep(.5)
            for _ in range(i):qemu.key('down')
            qemu.key('ret');wait_for(lambda:len(objects())==i+1,'import-'+str(i));time.sleep(.5)
        source_objects=objects()
        with FatReader(OUT/'boot-test.img')as fat:
            for e in fat.directory(fat.find(['NULLGRAV','VAULT']).cluster):
                if e.name.upper().endswith('.NGV'):
                    blob=fat.read_entry(e)
                    for name in expected:assert name.encode()not in blob
        others={name:digest(OUT/name)for name in ['import-source.img','clone-target.img']}
        bridge=vm_export.create_bridge(OUT/'bridges');vm_export.attach_bridge(bridge,OUT);time.sleep(3)
        before=digest(bridge)
        qemu.key('2');qemu.key('e');time.sleep(.5);capture('export-destination')
        qemu.key('ret');time.sleep(.5);capture('export-confirmation');qemu.key('esc');qemu.key('esc');time.sleep(.5)
        assert digest(bridge)==before,'Cancel wrote plaintext'
        for i in range(4):
            qemu.key('2')
            for _ in range(i):qemu.key('down')
            qemu.key('e');time.sleep(.3);qemu.key('ret');qemu.text('EXPORT');qemu.key('ret')
            def exported():
                try:return len(vm_export.committed_files(bridge))==i+1
                except(Exception):return False
            wait_for(exported,'export-'+str(i));time.sleep(.6)
        capture('export-complete')
        assert objects()==source_objects,'Vault ciphertext changed during export'
        assert {n:digest(OUT/n)for n in others}==others
        qemu.key('q')
        def gone():
            try:qemu.command('query-status');return False
            except(OSError,ValueError):return True
        wait_for(gone,'shutdown',20)
        saved=vm_export.save_exports(bridge,OUT)
        assert {v['name']:v['sha256']for v in saved}==expected
        print('Text, image, original PDF and original MP4 exported byte-for-byte.',flush=True)
        # Corrupt only data authentication tags on the disposable boot image.
        # A fresh target must remain completely unchanged after failed releases.
        with FatReader(OUT/'boot-test.img')as fat:
            changes=[]
            for e in fat.directory(fat.find(['NULLGRAV','VAULT']).cluster):
                if e.name.upper().endswith('.NGV'):
                    chain=fat.chain(e.cluster,(e.size+fat.cluster_size-1)//fat.cluster_size+1)
                    changes.append(fat.cluster_offset(chain[-1])+(e.size-1)%fat.cluster_size)
        with (OUT/'boot-test.img').open('r+b')as f:
            for offset in changes:f.seek(offset);b=f.read(1);f.seek(offset);f.write(bytes([b[0]^1]))
        qemu.STATE.unlink(missing_ok=True);qemu.start(resume=True,memory_mib=1024)
        wait_for(ready,'tamper-dashboard');time.sleep(.5)
        qemu.key('u');qemu.text(PASS);qemu.key('ret');time.sleep(8)
        empty=vm_export.create_bridge(OUT/'tampered-target');vm_export.attach_bridge(empty,OUT);time.sleep(3)
        empty_hash=digest(empty)
        for i in range(4):
            qemu.key('2')
            for _ in range(i):qemu.key('down')
            qemu.key('e');time.sleep(.3)
            for _ in range(6):qemu.key('down')
            qemu.key('ret');qemu.text('EXPORT');qemu.key('ret');time.sleep(1.5)
        capture('tampered-export-refused');assert digest(empty)==empty_hash,'Unauthenticated data wrote to destination'
        result={'image_sha256':digest(qemu.ROOT/'dist/xevu-0.5.0-x86_64.img'),
            'original_file_hashes':expected,'restored_file_hashes':{v['name']:v['sha256']for v in saved},
            'source_names_hidden_in_encrypted_objects':True,'vault_ciphertext_retained':True,
            'cancel_does_not_write':True,'tampered_objects_do_not_write_plaintext':True,
            'other_virtual_disks_unchanged':True,'source_formats_not_converted':True,
            'physical_devices_attached':False}
        (OUT/'export-verification.json').write_text(json.dumps(result,indent=2)+'\n')
        (OUT/'bridge-path.txt').write_text(str(bridge)+'\n')
        print(json.dumps(result,indent=2),flush=True)
    finally:stop()

if __name__=='__main__':main()

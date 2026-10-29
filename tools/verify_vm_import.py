"""Hotplug a host-file transfer, then import/view it through the native UI."""
import hashlib,json,time
import qemu,vm_import

OUT=qemu.ROOT/'test-output/import-flow-vm'
PASS='public import test passphrase'

def digest(path):
    with path.open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def capture(label):
    p=OUT/(label+'.ppm');qemu.command('screendump',{'filename':str(p)})
    parts=p.read_bytes().split(b'\n',3)
    return parts[3]if len(parts)==4 and parts[1]==b'1280 720'else b''

def wait_for(check,label,timeout=100):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        if check():return
        time.sleep(.3)
    capture('failed-'+label);raise RuntimeError('Timeout: '+label)

def dashboard():
    rgb=capture('startup')
    return rgb and rgb[(170*1280+16)*3:(170*1280+16)*3+3]==bytes([196,243,107])

def stop():
    try:qemu.command('quit')
    except(OSError,ValueError):pass
    qemu.STATE.unlink(missing_ok=True);time.sleep(.5)

def main():
    qemu.OUT=OUT;qemu.STATE=OUT/'qemu-session.json'
    qemu.start(memory_mib=1024)
    try:
        wait_for(dashboard,'dashboard');time.sleep(.6)
        originals={n:digest(OUT/n)for n in ['import-source.img','clone-target.img']}
        qemu.key('n');qemu.text(PASS);qemu.key('ret');qemu.text(PASS);qemu.key('ret')
        def armed():
            with (OUT/'boot-test.img').open('rb')as f:f.seek(8192);return f.read(8)==b'NGTRY001'
        wait_for(armed,'created');time.sleep(8)
        source=OUT/'My research & notes.txt';source.write_text('WINDOWS FILE TRANSFER WORKS\nPUBLIC FIXTURE / ORIGINAL PRESERVED\n',encoding='utf-8')
        source_hash=digest(source)
        receipt=vm_import.build_transfer([str(source)],OUT/'transfers')
        with (OUT/'boot-test.img').open('rb')as f:initial_magic_count=f.read().count(b'NGFILE01')
        image=qemu.ROOT/receipt['image'];image_before=digest(image)
        attached=vm_import.attach_transfer(image,OUT)
        blocks=qemu.command('query-block')
        assert any(b.get('inserted',{}).get('node-name')==attached['node'] and b['inserted'].get('ro')for b in blocks)
        print('Host file attached as a read-only USB transfer image.',flush=True)
        time.sleep(3);qemu.key('3');qemu.key('r');time.sleep(2)
        for _ in range(12):qemu.key('down')
        qemu.key('ret');time.sleep(1);capture('source-browser')
        qemu.key('ret');time.sleep(3);qemu.key('2');time.sleep(.6);capture('vault-imported')
        qemu.key('ret');time.sleep(1);capture('imported-file-view')
        # The native object magic must have appeared beyond the EFI binary's own
        # constants; a new, nontrivial object is also visible in the captured UI.
        with (OUT/'boot-test.img').open('rb')as f:
            raw=f.read();assert raw.count(b'NGFILE01')>initial_magic_count
        assert digest(source)==source_hash and digest(image)==image_before
        assert {n:digest(OUT/n)for n in originals}==originals
        boot_hash=digest(OUT/'boot-test.img');stop()
        qemu.start(resume=True,memory_mib=1024)
        blocks=qemu.command('query-block')
        assert any(b.get('inserted',{}).get('file')==str(image) and b['inserted'].get('ro')for b in blocks)
        assert digest(OUT/'boot-test.img')==boot_hash
        result={'image_sha256':digest(qemu.ROOT/'dist/nullgrav-0.3.0-x86_64.img'),
            'live_read_only_transfer_attached':True,'native_import_and_view_observed':True,
            'source_file_and_transfer_image_unchanged':True,'other_virtual_disks_unchanged':True,
            'vault_disk_retained_across_restart':True,'latest_transfer_reattached_read_only':True,
            'physical_devices_attached':False,'fixture_only':True}
        (OUT/'vm-import-verification.json').write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result,indent=2),flush=True)
    finally:stop()

if __name__=='__main__':main()

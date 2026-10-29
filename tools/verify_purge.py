"""Destructive native-UI verification on disposable regular QEMU images only.

Run with Linux/WSL Python 3 and installed QEMU/OVMF. No physical disk access.
The separate purge-vm directory never touches the user's interactive VM.
"""
import argparse, datetime, hashlib, json, shutil, struct, time
from pathlib import Path
import qemu

OUT = qemu.ROOT / 'test-output/purge-vm'
PASS = 'public test passphrase 2026'  # Public synthetic fixture, never production.

def digest(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()

def capture(label):
    p = OUT / (label + '.ppm')
    qemu.command('screendump', {'filename': str(p)})
    return p.read_bytes()

def wait_for(check, label, timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if check(): return
        time.sleep(.3)
    capture('failure-' + label)
    raise RuntimeError('Timeout: ' + label)

def ui_ready():
    parts = capture('current').split(b'\n', 3)
    if len(parts) != 4 or parts[1] != b'1280 720': return False
    rgb = parts[3]; i = (170 * 1280 + 16) * 3
    return rgb[i:i+3] == bytes([196, 243, 107])

def record():
    with (OUT/'boot-test.img').open('rb') as f:
        f.seek(16 * 512); raw = f.read(1024)
    records = []
    for b in [raw[:512], raw[512:]]:
        if b[:8] == b'NGTRY001' and hashlib.sha256(b[:64]).digest() == b[64:96]:
            records.append((struct.unpack_from('<Q', b, 8)[0], b[16]))
    return max(records) if records else None

def unlock(password, expected):
    qemu.key('l'); qemu.key('u'); time.sleep(.5)
    qemu.text(password); qemu.key('ret')
    wait_for(lambda: record() is not None and record()[1] == expected,
             'unlock-count-' + str(expected))
    time.sleep(1.2)

def stop():
    try: qemu.command('quit')
    except (OSError, ValueError): pass
    qemu.STATE.unlink(missing_ok=True)
    time.sleep(.5)

def resume_trigger_test():
    """Simulate an interrupted, committed erase with its key already destroyed."""
    before = OUT/'before-erasure.img'
    raw = before.read_bytes()
    slots = [raw[16*512:17*512], raw[17*512:18*512]]
    latest = max(slots, key=lambda b: struct.unpack_from('<Q', b, 8)[0])
    assert latest[:8] == b'NGTRY001'
    key_hash = latest[32:64]
    offsets = []; offset = 0
    while True:
        offset = raw.find(b'NGKEY001', offset)
        if offset < 0: break
        if hashlib.sha256(raw[offset:offset+128]).digest() == key_hash: offsets.append(offset)
        offset += 8
    assert len(offsets) == 1, 'The public test key capsule must have a unique hash match'
    out = qemu.ROOT/'test-output/purge-resume-vm'; out.mkdir(parents=True, exist_ok=True)
    staged = out/'committed-trigger.img'; shutil.copyfile(before, staged)
    next_record = bytearray(latest)
    struct.pack_into('<Q', next_record, 8, struct.unpack_from('<Q', latest, 8)[0]+1)
    next_record[16] = 3; next_record[64:96] = hashlib.sha256(next_record[:64]).digest()
    with staged.open('r+b') as f:
        f.seek(offsets[0]); f.write(bytes([0xa5])*128)
        for lba in (16,17): f.seek(lba*512); f.write(next_record)
    del raw
    qemu.OUT = out; qemu.STATE = out/'qemu-session.json'
    try:
        qemu.start(staged)
        others = {n:digest(out/n) for n in ['import-source.img','clone-target.img']}
        deadline = time.monotonic()+180; shutdown=False
        while time.monotonic()<deadline:
            try:
                qemu.command('query-status')
                qemu.command('screendump',{'filename':str(out/'startup-erasure.ppm')})
            except (OSError,ValueError): shutdown=True; break
            time.sleep(.8)
        assert shutdown, 'Committed erasure did not resume and finish without input'
        boot=out/'boot-test.img'
        with boot.open('rb') as f: assert f.read(18*512)==bytes(18*512)
        assert {n:digest(out/n) for n in others}==others
        assert boot.stat().st_size==staged.stat().st_size
        receipt={'image_sha256':digest(qemu.ROOT/'dist/nullgrav-0.3.0-x86_64.img'),
            'simulated_interrupted_committed_trigger':True,
            'key_capsule_overwritten_before_boot':True,'startup_resumed_without_input':True,
            'boot_records_invalidated':True,'automatic_shutdown_observed':True,
            'other_virtual_disks_unchanged':True,'physical_devices_attached':False}
        (out/'purge-resume-verification.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps(receipt,indent=2),flush=True)
    finally: stop()

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--continue-created', action='store_true', help='Continue this test session after manually creating and importing public fixtures')
    p.add_argument('--resume-trigger-only', action='store_true', help='Use the completed public test snapshot to verify startup erasure with an already destroyed key')
    args = p.parse_args()
    if args.resume_trigger_only:
        resume_trigger_test(); return
    qemu.OUT = OUT; qemu.STATE = OUT/'qemu-session.json'
    image = qemu.ROOT/'dist/nullgrav-0.3.0-x86_64.img'
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    try:
        if not args.continue_created:
            qemu.start(image)
            wait_for(ui_ready, 'native-dashboard')
            time.sleep(1)
            qemu.key('n'); qemu.text(PASS); qemu.key('ret'); qemu.text(PASS); qemu.key('ret')
            wait_for(lambda: record() == (1, 0), 'vault-created')
            time.sleep(8)
            qemu.key('3'); qemu.key('down'); qemu.key('ret'); time.sleep(1)
            qemu.key('ret'); time.sleep(3)  # FIELD.TXT, encrypted and readback verified.
        assert record() == (1, 0), 'This must be the fresh public test vault'
        qemu.key('esc'); qemu.key('1'); time.sleep(1)
        other_before = {name: digest(OUT/name) for name in ['import-source.img', 'clone-target.img']}
        print('Created native vault; imported public fixture. Testing failure count...', flush=True)
        unlock('wrong public password one', 1)
        capture('first-failure')
        unlock('wrong public password two', 2)
        capture('second-failure')
        stop()
        assert record()[1] == 2
        qemu.start(resume=True)
        wait_for(ui_ready, 'reboot-with-two-failures'); time.sleep(1)
        assert record()[1] == 2
        print('Two failures survived a complete QEMU power cycle.', flush=True)
        unlock(PASS, 0)
        capture('successful-reset')
        print('Correct passphrase reset the persistent count.', flush=True)
        unlock('wrong public password one', 1)
        unlock('wrong public password two', 2)
        qemu.key('u'); time.sleep(1); capture('last-attempt-warning'); qemu.key('esc')
        boot = OUT/'boot-test.img'
        before = OUT/'before-erasure.img'; shutil.copyfile(boot, before)
        original_size = boot.stat().st_size
        qemu.key('u'); qemu.text('third wrong public password'); qemu.key('ret')
        triggered = False; completed = False; last_capture = time.monotonic()
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            r = record()
            if r and r[1] == 3: triggered = True
            try:
                qemu.command('query-status')
                if triggered and time.monotonic() - last_capture > .8:
                    capture('erase-progress'); last_capture = time.monotonic()
            except (OSError, ValueError):
                completed = True; break
            time.sleep(.15)
        assert triggered, 'Third failure was not durably triggered'
        assert completed, 'VM did not complete erasure and shut down'
        assert boot.stat().st_size == original_size
        changed = 0
        with before.open('rb') as a, boot.open('rb') as b:
            while True:
                old = a.read(1024*1024); new = b.read(1024*1024)
                if not old: break
                assert old != new, f'Unchanged MiB batch {changed}'
                changed += 1
        with boot.open('rb') as f:
            assert f.read(18*512) == bytes(18*512), 'MBR/gap not invalidated'
            f.seek(2048*512); assert f.read(512)[510:] != b'\x55\xaa'
            f.seek((2048+6)*512); assert f.read(512)[510:] != b'\x55\xaa'
            f.seek(383*1024*1024); assert b'PUBLIC UNALLOCATED TAIL' not in f.read(100)
        other_after = {name: digest(OUT/name) for name in other_before}
        assert other_after == other_before, 'Another attached disk changed'
        purged_hash = digest(boot)
        print(f'Erased and verified {changed} MiB; MBR invalid; other disks unchanged; VM shut down.', flush=True)
        shutil.copyfile(boot, OUT/'erased-card.img')
        qemu.STATE.unlink(missing_ok=True)
        # Reimage ONLY our regular VM file; preserve its capacity. Booting a new
        # FAT image demonstrates that logical erasure did not lock/damage it.
        with image.open('rb') as src, boot.open('r+b') as dst:
            shutil.copyfileobj(src, dst, 1024*1024); dst.flush()
        qemu.start(resume=True)
        wait_for(ui_ready, 'reimaged-reusable-card'); time.sleep(1)
        capture('reusable-card-dashboard')
        assert boot.stat().st_size == original_size
        receipt = {'started_at_utc': started, 'image_sha256': digest(image),
            'native_initialization_and_import': True, 'two_failures_survived_power_cycle': True,
            'correct_unlock_reset_count': True, 'third_failure_trigger_observed': triggered,
            'erased_bytes': original_size, 'every_mib_batch_changed': changed,
            'mbr_and_gap_zero_verified': True, 'fat_boot_and_backup_invalid': True,
            'unallocated_tail_overwritten': True, 'other_disks_sha256_before': other_before,
            'other_disks_sha256_after': other_after, 'erased_image_sha256': purged_hash,
            'automatic_shutdown_observed': completed, 'reimaged_card_booted': True,
            'physical_devices_attached': False, 'flash_forensic_sanitization_proven': False}
        (OUT/'purge-verification.json').write_text(json.dumps(receipt, indent=2)+'\n')
        print(json.dumps(receipt, indent=2), flush=True)
    finally: stop()

if __name__ == '__main__': main()

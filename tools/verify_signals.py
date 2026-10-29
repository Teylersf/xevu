"""Exercise the animated native UI on disposable file-backed QEMU media only."""
import datetime, hashlib, json, time
import qemu

OUT=qemu.ROOT/'test-output/signals-vm-0.3.0'

def digest(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def capture(label):
    p=OUT/(label+'.ppm');qemu.command('screendump',{'filename':str(p)})
    data=p.read_bytes();parts=data.split(b'\n',3)
    return data,parts[3] if len(parts)==4 and parts[1]==b'1280 720' else b''

def crop(rgb,x,y,w,h):
    return b''.join(rgb[((y+row)*1280+x)*3:((y+row)*1280+x+w)*3] for row in range(h))

def main():
    qemu.OUT=OUT;qemu.STATE=OUT/'qemu-session.json'
    image=qemu.ROOT/'dist/nullgrav-0.3.0-x86_64.img'
    started=datetime.datetime.now(datetime.timezone.utc).isoformat()
    qemu.start(image,memory_mib=1024)
    before={n:digest(OUT/n) for n in ['boot-test.img','import-source.img','clone-target.img']}
    try:
        deadline=time.monotonic()+100;boot_frames=[];ready=False
        while time.monotonic()<deadline:
            data,rgb=capture('boot-candidate')
            if rgb and rgb[(1*1280+1)*3:(1*1280+1)*3+3]==bytes([196,243,107]):
                if len(boot_frames)<12:
                    (OUT/f'boot-{len(boot_frames):02}.ppm').write_bytes(data);boot_frames.append(hashlib.sha256(rgb).hexdigest())
            if rgb and rgb[(170*1280+16)*3:(170*1280+16)*3+3]==bytes([196,243,107]):
                ready=True;break
            time.sleep(.18)
        assert ready,'Native dashboard did not appear'
        time.sleep(.7);capture('dashboard')
        print('Native animated boot and dashboard reached.',flush=True)
        qemu.key('6');time.sleep(1)
        frames=[]
        for i in range(18):
            _,rgb=capture(f'signal-{i:02}');assert rgb
            frames.append(hashlib.sha256(rgb).hexdigest());time.sleep(.16)
        assert len(set(frames))>=5,'Live stream did not animate'
        qemu.key('spc');time.sleep(.8)
        _,held=capture('held-one');time.sleep(.5);_,held2=capture('held-two')
        assert held==held2,'Hold did not stop frame changes'
        qemu.key('r');time.sleep(.8);_,fresh=capture('fresh-sample')
        assert crop(held,660,275,560,62)!=crop(fresh,660,275,560,62),'R did not change the public sample'
        assert crop(held,282,225,337,394)==crop(fresh,282,225,337,394),'Sample generation resumed the held scope'
        print('Live motion, hold, and fresh independent random samples verified.',flush=True)
        qemu.key('g');qemu.key('spc');time.sleep(.6)
        _,off=capture('motion-off-one');time.sleep(.5);_,off2=capture('motion-off-two')
        assert off==off2,'G did not disable motion'
        qemu.key('g');time.sleep(.7);_,live=capture('motion-resumed-one');time.sleep(.5);_,live2=capture('motion-resumed-two')
        assert live!=live2,'G did not resume motion'
        qemu.key('n');time.sleep(.6)
        _,modal=capture('modal-one');time.sleep(.5);_,modal2=capture('modal-two')
        assert modal==modal2,'Effects continued behind a passphrase prompt'
        qemu.text('gpublic');time.sleep(.4);capture('masked-input');qemu.key('esc');time.sleep(.5)
        with (OUT/'boot-test.img').open('rb') as f:f.seek(8192);assert f.read(1024)==bytes(1024)
        qemu.key('1');time.sleep(.6);capture('final-dashboard')
        after={n:digest(OUT/n) for n in before};assert after==before,'Visual UI wrote a disk image'
        qemu.key('q');shutdown=False;deadline=time.monotonic()+20
        while time.monotonic()<deadline:
            try:qemu.command('query-status')
            except (OSError,ValueError):shutdown=True;break
            time.sleep(.2)
        assert shutdown,'Keyboard shutdown failed'
        receipt={'started_at_utc':started,'image_sha256':digest(image),
            'boot_animation_distinct_frames':len(set(boot_frames)),
            'signal_lab_distinct_frames':len(set(frames)),'signal_frames_captured':len(frames),
            'hold_freezes_stream':True,'new_random_sample_changes_while_held':True,
            'g_disables_and_resumes_motion':True,'passphrase_prompt_pauses_effects':True,
            'cancelled_prompt_did_not_arm_retry_counter':True,
            'all_three_virtual_disks_unchanged':True,'keyboard_shutdown_observed':True,
            'physical_devices_attached':False,'cryptographic_audit':False}
        (OUT/'signals-verification.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps(receipt,indent=2),flush=True)
    finally:
        try:qemu.command('quit')
        except (OSError,ValueError):pass
        qemu.STATE.unlink(missing_ok=True)

if __name__=='__main__':main()

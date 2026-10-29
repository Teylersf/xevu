"""Verify the release's native boot artwork and survival past UEFI's 5-minute deadline.

Run under the same Linux/WSL environment as qemu.py. Only test images are used.
"""
import datetime,hashlib,json,time
import qemu
def capture(label):
    path=qemu.OUT/(label+'.ppm');qemu.command('screendump',{'filename':str(path)});return path,path.read_bytes()
def main():
    if qemu.STATE.exists():
        try:qemu.command('quit')
        except (OSError,ValueError):pass
        qemu.STATE.unlink(missing_ok=True)
    proc=qemu.start();started=time.monotonic();wall=datetime.datetime.now(datetime.timezone.utc).isoformat();splash=False
    try:
        while time.monotonic()-started<35:
            p,data=capture('boot-candidate');parts=data.split(b'\n',3)
            if len(parts)==4 and parts[1]==b'1280 720' and parts[3][(1*1280+1)*3:(1*1280+1)*3+3]==bytes([196,243,107]):
                (qemu.OUT/'boot-splash.ppm').write_bytes(data);splash=True;print('Native NULLGRAV boot artwork captured.',flush=True);break
            if proc.poll() is not None:raise RuntimeError('VM exited during boot')
            time.sleep(.35)
        if not splash:raise RuntimeError('Native boot artwork was not observed')
        for checkpoint in [45,120,240,365]:
            while time.monotonic()-started<checkpoint:
                if proc.poll()is not None:raise RuntimeError(f'VM exited with code {proc.returncode} before the watchdog deadline')
                time.sleep(.5)
            status=qemu.command('query-status')
            if not status.get('running'):raise RuntimeError('VM is not running')
            print(f'Native session alive after {checkpoint} seconds.',flush=True)
            if checkpoint==45:capture('release-dashboard')
        qemu.key('5');time.sleep(2);capture('long-session-system');elapsed=time.monotonic()-started
        source=qemu.ROOT/'dist/nullgrav-0.2.0-x86_64.img'
        with source.open('rb')as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
        receipt={'started_at_utc':wall,'elapsed_seconds':round(elapsed,2),'survived_firmware_watchdog_deadline':True,'native_boot_artwork_observed':splash,'qmp_running':True,'image_sha256':digest,'physical_devices_attached':False,'security_audit':False}
        (qemu.OUT/'long-session-verification.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt),flush=True)
    finally:
        try:qemu.command('quit')
        except (OSError,ValueError):pass
        qemu.STATE.unlink(missing_ok=True)
if __name__=='__main__':main()

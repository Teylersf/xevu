"""QEMU/OVMF controls for regular-file test images only.

Windows: wsl -d Ubuntu -- python3 /mnt/e/aicaller/tools/qemu.py start
         wsl -d Ubuntu -- python3 /mnt/e/aicaller/tools/qemu.py capture dashboard
No physical host disk is ever passed to QEMU.
"""
from pathlib import Path
import argparse,json,os,shutil,socket,struct,subprocess,time
from build_image import FatImage
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'test-output'
STATE=OUT/'qemu-session.json'
def connection():
    state=json.loads(STATE.read_text());s=socket.create_connection(('127.0.0.1',state['port']),timeout=10);f=s.makefile('rwb');json.loads(f.readline());f.write(b'{"execute":"qmp_capabilities"}\n');f.flush()
    while 'return' not in json.loads(f.readline()):pass
    return s,f
def command(cmd,args=None):
    s,f=connection()
    try:
        f.write((json.dumps({'execute':cmd,'arguments':args or {}})+'\n').encode());f.flush()
        while True:
            r=json.loads(f.readline())
            if 'return' in r:return r['return']
            if 'error' in r:raise RuntimeError(r['error'])
    finally:f.close();s.close()
def key(name):command('human-monitor-command',{'command-line':f'sendkey {name} 70'});time.sleep(.14)
def text(value):
    for c in value:
        if c.isupper():name='shift-'+c.lower()
        elif c.islower() or c.isdigit():name=c
        else:name={' ':'spc','-':'minus','_':'shift-minus','.':'dot','/':'slash',':':'shift-semicolon','!':'shift-1','@':'shift-2'}.get(c)
        if not name:raise ValueError(f'Unsupported test input character: {c!r}')
        key(name)
def start(image=None,resume=False,display='none',memory_mib=512):
    if display not in ('none','gtk','sdl'):raise SystemExit('Unsupported display backend')
    if not 256<=memory_mib<=8192:raise SystemExit('Memory must be between 256 and 8192 MiB')
    OUT.mkdir(parents=True,exist_ok=True)
    if STATE.exists():
        try:command('query-status');raise SystemExit('A test VM is already running; stop it explicitly first.')
        except (OSError,ValueError):pass
    source=image or ROOT/'dist/xevu-0.5.0-x86_64.img'
    source=Path(source).resolve()
    if not source.is_file() or source.suffix.lower()!='.img':raise SystemExit('Use a regular test .img file, never a physical device.')
    boot=OUT/'boot-test.img'
    if not resume:shutil.copyfile(source,boot)
    elif not boot.exists():raise SystemExit('No saved test image to resume.')
    # A larger physical source proves replication excludes unallocated tail data
    # and can fit the 256 MiB image on a smaller destination.
    if not resume:
        with boot.open('r+b') as f:
            f.truncate(384*1024*1024);f.seek(383*1024*1024);f.write(b'PUBLIC UNALLOCATED TAIL FIXTURE / MUST NOT BE CLONED')
    target=OUT/'clone-target.img'
    if not resume or not target.exists():
        with target.open('wb')as f:f.truncate(256*1024*1024)
    data=FatImage(64);data.add('FIELD.TXT',b'XEVU PUBLIC TEST FIXTURE\r\nAn offline file workspace for every kind of work.\r\nThis is not sensitive content.\r\n')
    w,h=240,160;pixels=bytearray()
    for y in range(h):
        for x in range(w):pixels.extend((40+(x%100),60+(y%100),100 if (x//20+y//20)%2 else 210))
    bmp=bytearray(54);bmp[:2]=b'BM';struct.pack_into('<I',bmp,2,54+len(pixels));struct.pack_into('<I',bmp,10,54);struct.pack_into('<IiiHHI',bmp,14,40,w,h,1,24,0);data.add('TEST.BMP',bmp+pixels)
    fixtures=OUT/'fixtures'
    if fixtures.exists():
        for file in sorted(fixtures.iterdir()):
            if file.is_file():data.add(file.name,file.read_bytes())
    if not resume or not (OUT/'import-source.img').exists():data.write(OUT/'import-source.img')
    var=OUT/'OVMF_VARS.fd'
    if not resume or not var.exists():shutil.copyfile('/usr/share/OVMF/OVMF_VARS_4M.fd',var)
    with socket.socket()as reserve:reserve.bind(('127.0.0.1',0));port=reserve.getsockname()[1]
    cmd=['qemu-system-x86_64','-name','XEVU 0.5.0 - Signal Console' if display!='none' else 'XEVU 0.5.0 - Headless test VM','-machine','q35,accel=tcg','-cpu','max','-m',str(memory_mib)+'M','-display',display,'-net','none','-no-reboot','-qmp',f'tcp:127.0.0.1:{port},server=on,wait=off','-drive','if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd','-drive',f'if=pflash,format=raw,file={var}','-device','qemu-xhci,id=xhci','-device','usb-kbd,bus=xhci.0','-device',('usb-mouse' if display=='none' else 'usb-tablet')+',bus=xhci.0','-object','rng-random,filename=/dev/urandom,id=rng0','-device','virtio-rng-pci,rng=rng0']
    media=[(boot,False),(OUT/'import-source.img',True),(target,False)]
    saved_import=OUT/'last-windows-import.json'
    if saved_import.exists():
        info=json.loads(saved_import.read_text(encoding='utf-8'))
        imported=Path(info['image']).resolve(strict=True)
        allowed=(ROOT/'test-output').resolve()
        if allowed not in imported.parents or not imported.is_file() or imported.suffix.lower()!='.img':
            raise SystemExit('Saved transfer image is invalid. The vault was not modified.')
        media.append((imported,True))
    for i,(p,read_only) in enumerate(media):
        escaped=str(p).replace(',',',,')
        cmd+=['-drive',f'if=none,id=media{i},format=raw,file={escaped}'+(',readonly=on'if read_only else ''),'-device',f'usb-storage,bus=xhci.0,drive=media{i}'+(',bootindex=1'if i==0 else '')]
    with (OUT/'qemu.log').open('wb')as log:proc=subprocess.Popen(cmd,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    STATE.write_text(json.dumps({'pid':proc.pid,'port':port,'boot':str(boot),'target':str(target),'display':display,'memory_mib':memory_mib,'source_sha256_before':__import__('hashlib').file_digest((OUT/'import-source.img').open('rb'),'sha256').hexdigest()},indent=2))
    time.sleep(2)
    if proc.poll()is not None:raise SystemExit((OUT/'qemu.log').read_text())
    print(json.dumps({'pid':proc.pid,'port':port,'status':command('query-status')}),flush=True)
    return proc
def main():
    global OUT,STATE
    p=argparse.ArgumentParser();p.add_argument('action',choices=['start','run','resume','capture','key','text','stop','status']);p.add_argument('value',nargs='?',default='screen');p.add_argument('--image',type=Path);p.add_argument('--display',choices=['none','gtk','sdl'],default='none');p.add_argument('--memory-mib',type=int,default=512);p.add_argument('--session',choices=['test','interactive'],default='test');a=p.parse_args()
    if a.session=='interactive':OUT=OUT/'interactive-vm-0.5.0';STATE=OUT/'qemu-session.json'
    options={'display':a.display,'memory_mib':a.memory_mib}
    if a.action=='start':start(a.image,**options)
    elif a.action=='run':start(a.image,**options).wait()
    elif a.action=='resume':start(resume=True,**options).wait()
    elif a.action=='capture':
        if not a.value.replace('-','').replace('_','').isalnum():raise SystemExit('Use a simple capture label')
        out=OUT/(a.value+'.ppm');command('screendump',{'filename':str(out)});print(out)
    elif a.action=='key':key(a.value)
    elif a.action=='text':text(a.value)
    elif a.action=='status':print(json.dumps(command('query-status')))
    elif a.action=='stop':
        try:command('quit')
        except (OSError,ValueError):pass
        STATE.unlink(missing_ok=True)
if __name__=='__main__':main()

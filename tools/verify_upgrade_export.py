"""Control the owned migration/Windows-wrapper fixture session only."""
import json,time,hashlib
import qemu,verify_export,vm_export
from upgrade_vm import journal_record

def main():
    info=json.loads((qemu.ROOT/'test-output/current-export-upgrade.json').read_text())
    directory=qemu.ROOT/info['updated_relative']
    if not directory.name=='updated' or not directory.parent.name.startswith('export-upgrade-'):raise ValueError('Not a test migration session')
    qemu.OUT=directory;qemu.STATE=directory/'qemu-session.json';verify_export.OUT=directory
    verify_export.wait_for(verify_export.ready,'upgrade-dashboard');time.sleep(.5)
    qemu.key('u');qemu.text('public import test passphrase');qemu.key('ret')
    def unlocked():
        with (directory/'boot-test.img').open('rb')as f:f.seek(8192);raw=f.read(1024)
        records=[r for r in [journal_record(raw[:512]),journal_record(raw[512:])]if r]
        return records and max(records)[1]==0
    verify_export.wait_for(unlocked,'old-passphrase-unlocked');time.sleep(1)
    qemu.key('2');time.sleep(.4);verify_export.capture('migrated-vault')
    block=next(v for v in qemu.command('query-block')if v.get('inserted',{}).get('node-name','').startswith('ngexport_'))
    image=block['inserted']['file']
    qemu.key('e');time.sleep(.4);qemu.key('ret');qemu.text('EXPORT');qemu.key('ret')
    def done():
        try:return len(vm_export.committed_files(image))==1
        except Exception:return False
    verify_export.wait_for(done,'migrated-export');time.sleep(.4);verify_export.capture('migrated-export-complete')
    qemu.key('q')
    print('Old passphrase unlocked the migrated 0.4.0 vault; its original file was exported. Windows wrapper can finish.',flush=True)

if __name__=='__main__':main()

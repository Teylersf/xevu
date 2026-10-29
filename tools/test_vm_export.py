"""Export integrity, path safety and vault migration tests on disposable images."""
import hashlib,json,struct,tempfile,unittest
from pathlib import Path
from build_image import FatImage
from fat_reader import FatReader
from vm_export import save_exports,windows_name,create_bridge,cleanup_bridge
from upgrade_vm import migrate
import vm_session

class ExportTests(unittest.TestCase):
    def setUp(self):
        self.allowed=(vm_session.ROOT/'test-output').resolve();self.allowed.mkdir(exist_ok=True)
        self.temp=tempfile.TemporaryDirectory(prefix='export-test-',dir=self.allowed);self.root=Path(self.temp.name).resolve()
        self.out=self.root/'ordinary-pc-folder';self.out.mkdir()
    def tearDown(self):
        assert self.allowed in self.root.parents;self.temp.cleanup()
    def bridge(self,name='original & notes.txt',data=b'public\x00\xff\r\nexact bytes',claimed=None):
        image=FatImage(64,label='WINDOWS_OUT')
        image.add('NULLGRAV_EXPORTS/'+name,data)
        record={'version':1,'name':name,'bytes':len(data),'sha256':claimed or hashlib.sha256(data).hexdigest()}
        image.add('NG_EXPORT_LOG/test.JSON',json.dumps(record).encode())
        path=self.root/'bridge.img';image.write(path);return path,record,data
    def test_exact_plaintext_copy_and_same_existing_copy(self):
        image,record,data=self.bridge()
        with image.open('rb')as f:before=hashlib.file_digest(f,'sha256').hexdigest()
        result=save_exports(image,self.out)
        self.assertEqual((self.out/record['name']).read_bytes(),data);self.assertEqual(result[0]['sha256'],record['sha256'])
        save_exports(image,self.out)
        with image.open('rb')as f:self.assertEqual(hashlib.file_digest(f,'sha256').hexdigest(),before)
    def test_no_overwrite_and_no_write_after_tampering(self):
        image,record,_=self.bridge();old=self.out/record['name'];old.write_bytes(b'keep existing')
        with self.assertRaises(FileExistsError):save_exports(image,self.out)
        self.assertEqual(old.read_bytes(),b'keep existing');old.unlink()
        image,_,_=self.bridge(claimed='0'*64)
        with self.assertRaises(ValueError):save_exports(image,self.out)
        self.assertFalse(list(self.out.iterdir()))
    def test_rejects_paths_and_windows_device_names(self):
        for name in ['../outside','C:\\temp\\bad','NUL.txt','COM¹.bin','file:stream','bad?.png','CONOUT$','trailing.']:
            self.assertFalse(windows_name(name))
        self.assertTrue(windows_name('normal.pdf'))
        image,_,_=self.bridge(name='NUL.txt')
        with self.assertRaises(ValueError):save_exports(image,self.out)
        self.assertFalse(list(self.out.iterdir()))
    def test_cleanup_only_removes_owned_temporary_bridge(self):
        image,_,_=self.bridge();save_exports(image,self.out)
        with self.assertRaises(ValueError):cleanup_bridge(image,image.with_suffix('.json'))
        self.assertTrue(image.exists())
        temporary=create_bridge(self.root/'owned');receipt=temporary.with_suffix('.json');receipt.write_text('{}')
        cleanup_bridge(temporary,receipt)
        self.assertFalse(temporary.exists());self.assertFalse(receipt.exists())
        self.assertEqual(len(list(self.out.iterdir())),1)
    def test_fat_cycles_are_bounded(self):
        image,record,_=self.bridge(data=b'x'*1000)
        with FatReader(image)as fat:
            entry=fat.find(['NULLGRAV_EXPORTS',record['name']]);offset=(fat.start+fat.reserved)*512+entry.cluster*4
        with image.open('r+b')as f:f.seek(offset);f.write(struct.pack('<I',entry.cluster))
        with FatReader(image)as fat:
            with self.assertRaises(ValueError):fat.read(['NULLGRAV_EXPORTS',record['name']])
    def test_migration_preserves_ciphertext_and_retry_state(self):
        source=self.root/'source';source.mkdir();dest=self.root/'updated'
        header=b'opaque test key capsule'.ljust(128,b'\0');cipher=b'opaque encrypted test object'
        image=FatImage(64);image.add('EFI/BOOT/BOOTX64.EFI',b'old public fixture')
        image.add('NULLGRAV/VAULT/VAULT.NGK',header);image.add('NULLGRAV/VAULT/OBJECT.NGV',cipher)
        source_image=source/'boot-test.img';image.write(source_image)
        record=bytearray(512);record[:8]=b'NGTRY001';struct.pack_into('<Q',record,8,7);record[16]=2
        record[32:64]=hashlib.sha256(header).digest();record[64:96]=hashlib.sha256(record[:64]).digest()
        with source_image.open('r+b')as f:f.seek(8192);f.write(record*2)
        new_efi=self.root/'new.efi';new_efi.write_bytes(b'new public fixture')
        result=migrate(source,dest,new_efi);self.assertTrue(result['retry_state_preserved'])
        with FatReader(dest/'boot-test.img')as fat:
            self.assertEqual(fat.read(['NULLGRAV','VAULT','OBJECT.NGV']),cipher)
            self.assertEqual(fat.read_at(8192,1024),record*2)
            self.assertEqual(fat.read(['EFI','BOOT','BOOTX64.EFI']),b'new public fixture')
        with self.assertRaises(FileExistsError):migrate(source,dest,new_efi)

if __name__=='__main__':unittest.main()

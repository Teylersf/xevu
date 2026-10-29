"""Focused tests for the read-only VM transfer helper. No physical devices."""
import hashlib,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import vm_import

class TransferTests(unittest.TestCase):
    def setUp(self):
        self.allowed=(vm_import.ROOT/'test-output').resolve();self.allowed.mkdir(exist_ok=True)
        self.temp=tempfile.TemporaryDirectory(prefix='import-helper-test-',dir=self.allowed)
        self.root=Path(self.temp.name).resolve()
    def tearDown(self):
        assert self.allowed in self.root.parents
        self.temp.cleanup()
    def test_source_preserved_and_image_is_a_new_copy(self):
        source=self.root/'My research & notes.txt';source.write_bytes(b'PUBLIC SYNTHETIC TEST DATA')
        expected=source.read_bytes();result=vm_import.build_transfer([str(source)],self.root/'images')
        self.assertEqual(source.read_bytes(),expected)
        self.assertEqual(result['files'][0]['sha256'],hashlib.sha256(expected).hexdigest())
        image=Path(result['image']);self.assertTrue(image.is_file());self.assertTrue(result['read_only_attachment'])
        self.assertTrue(result['plaintext_transfer_copy']);self.assertIn(expected,image.read_bytes())
        self.assertEqual(json.loads(image.with_suffix('.json').read_text())['file_count'],1)
    def test_rejects_ambiguous_names_limits_and_outside_output(self):
        a=self.root/'NOTE.TXT';a.write_bytes(b'abc')
        folder=self.root/'second';folder.mkdir();b=folder/'note.txt';b.write_bytes(b'def')
        with self.assertRaises(ValueError):vm_import.build_transfer([str(a),str(b)],self.root/'images')
        with patch.object(vm_import,'MAX_FILE',2):
            with self.assertRaises(ValueError):vm_import.build_transfer([str(a)],self.root/'images')
        with self.assertRaises(ValueError):vm_import.build_transfer([str(a)],vm_import.ROOT/'dist')
        with self.assertRaises(ValueError):vm_import.safe_image(a)
        self.assertEqual(a.read_bytes(),b'abc');self.assertEqual(b.read_bytes(),b'def')
    def test_unicode_long_filename_keeps_original_lfn(self):
        from build_image import short_name,lfn_entries
        name='research.\u753b\u50cf'
        alias,long=short_name(name,1)
        self.assertTrue(long);self.assertEqual(len(alias),11);alias.decode('ascii')
        self.assertTrue(lfn_entries(name,alias))

if __name__=='__main__':unittest.main()

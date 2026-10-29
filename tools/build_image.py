"""Build a deterministic MBR/FAT32 removable-media image. Never opens a device.

Python's standard library is sufficient. --tree can include a prepared test vault;
the public release image never contains a passphrase or a pre-unlocked vault.
"""
from pathlib import Path
import argparse, hashlib, json, math, struct

SECTOR=512
class Node:
    def __init__(self,name,data=None,parent=None):
        self.name,self.data,self.parent=name,data,parent
        self.children={};self.cluster=0;self.count=0
    @property
    def directory(self): return self.data is None

def short_name(name,index):
    parts=name.rsplit('.',1);stem=parts[0];ext=parts[1] if len(parts)>1 else ''
    valid=lambda s: all(c.isascii() and (c.isalnum() or c in '_-') for c in s)
    exact=len(stem)<=8 and len(ext)<=3 and valid(stem) and valid(ext) and name==name.upper()
    alias_ext=''.join(c if c.isascii() and (c.isalnum() or c in '_-') else '_' for c in ext.upper())[:3]
    alias=(stem.upper().ljust(8)+ext.upper().ljust(3)).encode('ascii') if exact else (''.join(c for c in stem.upper() if c.isascii() and c.isalnum())[:5].ljust(5,'_')+f'~{index:02}'[-3:]+alias_ext.ljust(3)).encode('ascii')
    return alias,not exact

def lfn_entries(name,alias):
    checksum=0
    for b in alias: checksum=(((checksum&1)<<7)+(checksum>>1)+b)&255
    chars=list(struct.unpack('<'+'H'*(len(name.encode('utf-16-le'))//2),name.encode('utf-16-le')))+[0]
    count=math.ceil(len(chars)/13);chars.extend([0xffff]*(count*13-len(chars)));out=[]
    for n in reversed(range(count)):
        part=chars[n*13:n*13+13];e=bytearray(32);e[0]=(n+1)|(0x40 if n==count-1 else 0);e[11]=0x0f;e[13]=checksum
        for pos,c in zip([1,3,5,7,9,14,16,18,20,22,24,28,30],part):struct.pack_into('<H',e,pos,c)
        out.append(bytes(e))
    return out

class FatImage:
    def __init__(self,size_mib=256):
        if not 64<=size_mib<=32768:raise ValueError('image size must be 64..32768 MiB')
        self.size=size_mib*1024*1024;self.start=2048;self.total=self.size//512-self.start
        self.reserved=32;self.spc=1 if size_mib<=512 else 8
        self.fats=1
        while True:
            clusters=(self.total-self.reserved-2*self.fats)//self.spc
            needed=math.ceil((clusters+2)*4/512)
            if needed==self.fats:break
            self.fats=needed
        if clusters<65525:raise ValueError('image too small for FAT32')
        self.root=Node('');self.nodes=[self.root];self.next_cluster=2
    def add(self,path,data):
        node=self.root;parts=path.replace('\\','/').strip('/').split('/')
        for name in parts[:-1]:
            if name not in node.children:child=Node(name,parent=node);node.children[name]=child;self.nodes.append(child)
            node=node.children[name]
        if parts[-1] in node.children:raise ValueError('duplicate image path')
        child=Node(parts[-1],bytes(data),node);node.children[child.name]=child;self.nodes.append(child)
    def directory_bytes(self,node):
        entries=[]
        def entry(alias,cluster,size,directory):
            e=bytearray(32);e[:11]=alias;e[11]=0x10 if directory else 0x20
            struct.pack_into('<H',e,20,cluster>>16);struct.pack_into('<H',e,26,cluster&0xffff);struct.pack_into('<I',e,28,size);return bytes(e)
        if node is not self.root:
            entries.append(entry(b'.          ',node.cluster,0,True));entries.append(entry(b'..         ',node.parent.cluster if node.parent is not self.root else 0,0,True))
        for i,child in enumerate(node.children.values(),1):
            alias,long=short_name(child.name,i)
            if long:entries.extend(lfn_entries(child.name,alias))
            entries.append(entry(alias,child.cluster,0 if child.directory else len(child.data),child.directory))
        return b''.join(entries)+bytes(32)
    def write(self,destination):
        cluster_size=512*self.spc
        for node in self.nodes:
            length=len(self.directory_bytes(node)) if node.directory else len(node.data)
            node.count=max(1,math.ceil(length/cluster_size));node.cluster=self.next_cluster;self.next_cluster+=node.count
        if self.next_cluster*4>self.fats*512:raise ValueError('image is full')
        fat=bytearray(self.fats*512);struct.pack_into('<II',fat,0,0x0ffffff8,0xffffffff)
        for node in self.nodes:
            for i in range(node.count):struct.pack_into('<I',fat,(node.cluster+i)*4,node.cluster+i+1 if i+1<node.count else 0x0fffffff)
        destination=Path(destination)
        if str(destination).startswith('\\\\.\\') or destination.suffix.lower()!='.img':raise ValueError('output must be a regular .img file')
        destination.parent.mkdir(parents=True,exist_ok=True)
        with destination.open('wb') as f:
            f.truncate(self.size)
            mbr=bytearray(512);mbr[446]=0x80;mbr[450]=0xef;struct.pack_into('<II',mbr,454,self.start,self.total);mbr[510:]=b'\x55\xaa';f.write(mbr)
            b=bytearray(512);b[:3]=b'\xeb\x58\x90';b[3:11]=b'NULLGRAV';struct.pack_into('<H',b,11,512);b[13]=self.spc;struct.pack_into('<H',b,14,self.reserved);b[16]=2;b[21]=0xf8;struct.pack_into('<HH',b,24,63,255);struct.pack_into('<I',b,28,self.start);struct.pack_into('<I',b,32,self.total);struct.pack_into('<I',b,36,self.fats);struct.pack_into('<I',b,44,self.root.cluster);struct.pack_into('<HH',b,48,1,6);b[64]=0x80;b[66]=0x29;struct.pack_into('<I',b,67,0x4e475630);b[71:82]=b'NULLGRAV   ';b[82:90]=b'FAT32   ';b[510:]=b'\x55\xaa'
            info=bytearray(512);struct.pack_into('<I',info,0,0x41615252);struct.pack_into('<I',info,484,0x61417272);struct.pack_into('<II',info,488,0xffffffff,0xffffffff);struct.pack_into('<I',info,508,0xaa550000)
            for sector,data in [(0,b),(1,info),(6,b),(7,info)]:f.seek((self.start+sector)*512);f.write(data)
            for i in range(2):f.seek((self.start+self.reserved+i*self.fats)*512);f.write(fat)
            data_start=self.start+self.reserved+2*self.fats
            for node in self.nodes:
                f.seek((data_start+(node.cluster-2)*self.spc)*512);f.write(self.directory_bytes(node) if node.directory else node.data)
        with destination.open('rb') as check:
            return hashlib.file_digest(check,'sha256').hexdigest()

def main():
    root=Path(__file__).resolve().parents[1];p=argparse.ArgumentParser();p.add_argument('--boot',type=Path,default=root/'target/x86_64-unknown-uefi/release/nullgrav-boot.efi');p.add_argument('--output',type=Path,default=root/'dist/nullgrav-0.3.0-x86_64.img');p.add_argument('--size-mib',type=int,default=256);p.add_argument('--tree',type=Path);args=p.parse_args()
    image=FatImage(args.size_mib);image.add('EFI/BOOT/BOOTX64.EFI',args.boot.read_bytes());image.add('NULLGRAV/README.TXT',b'NULLGRAV 0.3 ENGINEERING PREVIEW\r\nNo secret content included. Not audited or approved for classified data.\r\nPress N to create a vault; U to unlock; 1-6 to navigate; Q to shut down.\r\n')
    if args.tree:
        for file in sorted(args.tree.rglob('*')):
            if file.is_file():image.add(file.relative_to(args.tree).as_posix(),file.read_bytes())
    digest=image.write(args.output);receipt={'image':args.output.name,'bytes':args.output.stat().st_size,'sha256':digest,'boot_sha256':hashlib.sha256(args.boot.read_bytes()).hexdigest(),'format':'MBR / FAT32 ESP / x86-64 UEFI','includes_secrets':False if not args.tree else 'custom tree; caller must inspect'}
    args.output.with_suffix('.json').write_text(json.dumps(receipt,indent=2)+'\n');args.output.with_suffix('.sha256').write_text(digest+'  '+args.output.name+'\n');print(json.dumps(receipt,indent=2))
if __name__=='__main__':main()

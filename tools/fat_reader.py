"""Bounded, read-only FAT32 reader for our regular-file VM disk images.

This is not an NTFS driver or a physical-disk mounting tool. No write API exists.
"""
from pathlib import Path
from dataclasses import dataclass
import os,stat,struct

@dataclass(frozen=True)
class Entry:
    name:str
    cluster:int
    size:int
    directory:bool

class FatReader:
    def __init__(self,path):
        self.path=Path(path).resolve(strict=True)
        if not stat.S_ISREG(self.path.stat().st_mode) or self.path.suffix.lower()!='.img':raise ValueError('Use a regular VM .img file')
        self.size=self.path.stat().st_size
        if not 32*1024*1024<=self.size<=512*1024*1024:raise ValueError('VM image size out of bounds')
        self.f=self.path.open('rb')
        try:
            mbr=self.read_at(0,512)
            if mbr[510:]!=b'\x55\xaa':raise ValueError('Invalid MBR')
            parts=[]
            for i in range(4):
                p=mbr[446+i*16:462+i*16]
                if p[4]:parts.append(p)
            if len(parts)!=1 or parts[0][4]!=0xef:raise ValueError('Expected one FAT ESP')
            self.start,self.sectors=struct.unpack_from('<II',parts[0],8)
            if self.start<2048 or (self.start+self.sectors)*512>self.size:raise ValueError('Invalid partition extent')
            b=self.read_at(self.start*512,512)
            if b[510:]!=b'\x55\xaa' or struct.unpack_from('<H',b,11)[0]!=512:raise ValueError('Invalid FAT boot sector')
            self.spc=b[13];self.reserved=struct.unpack_from('<H',b,14)[0];self.fats=b[16]
            self.fat_sectors=struct.unpack_from('<I',b,36)[0];self.root_cluster=struct.unpack_from('<I',b,44)[0]
            if self.spc not in (1,2,4,8,16,32,64,128) or not 1<=self.fats<=2 or self.reserved<1:raise ValueError('Invalid FAT geometry')
            self.data_sector=self.start+self.reserved+self.fats*self.fat_sectors
            self.clusters=(self.start+self.sectors-self.data_sector)//self.spc
            if self.clusters<65525 or self.clusters+2>self.fat_sectors*512//4:raise ValueError('FAT allocation bounds invalid')
            self.cluster_size=self.spc*512
            self.fat=self.read_at((self.start+self.reserved)*512,self.fat_sectors*512)
        except BaseException:
            self.f.close();raise
    def __enter__(self):return self
    def __exit__(self,*_):self.f.close()
    def read_at(self,offset,count):
        if offset<0 or count<0 or offset+count>self.size:raise ValueError('Image read out of bounds')
        self.f.seek(offset);data=self.f.read(count)
        if len(data)!=count:raise ValueError('Truncated image')
        return data
    def chain(self,start,limit):
        out=[];seen=set();c=start
        while True:
            if c<2 or c>=self.clusters+2 or c in seen or len(out)>=limit:raise ValueError('Invalid, cyclic or excessive FAT chain')
            seen.add(c);out.append(c)
            c=struct.unpack_from('<I',self.fat,c*4)[0]&0x0fffffff
            if c>=0x0ffffff8:return out
    def cluster_offset(self,c):
        if not 2<=c<self.clusters+2:raise ValueError('Invalid cluster')
        return (self.data_sector+(c-2)*self.spc)*512
    def read_entry(self,entry,limit=129*1024*1024):
        if entry.directory or entry.size>limit:raise ValueError('File exceeds read bounds')
        if entry.size==0:return b''
        needed=(entry.size+self.cluster_size-1)//self.cluster_size
        chain=self.chain(entry.cluster,needed+1)
        if len(chain)!=needed:raise ValueError('File size disagrees with allocation')
        return b''.join(self.read_at(self.cluster_offset(c),self.cluster_size)for c in chain)[:entry.size]
    def directory(self,cluster=None):
        cluster=self.root_cluster if cluster is None else cluster
        chain=self.chain(cluster,128*1024//self.cluster_size+1)
        raw=b''.join(self.read_at(self.cluster_offset(c),self.cluster_size)for c in chain)
        out=[];lfn={};expected=0;checksum=None
        for p in range(0,len(raw),32):
            e=raw[p:p+32]
            if e[0]==0:return out
            if e[0]==0xe5:lfn={};expected=0;continue
            if e[11]==0x0f:
                seq=e[0]&31
                if e[0]&0x40:lfn={};expected=seq;checksum=e[13]
                if not 1<=seq<=20 or seq!=expected or e[13]!=checksum or e[12]!=0 or e[26:28]!=b'\0\0':raise ValueError('Invalid long filename sequence')
                lfn[seq]=b''.join(e[a:a+n]for a,n in [(1,10),(14,12),(28,4)]);expected-=1;continue
            if e[11]&8:lfn={};expected=0;continue
            alias=e[:11]
            if lfn:
                c=0
                for b in alias:c=(((c&1)<<7)+(c>>1)+b)&255
                if expected!=0 or c!=checksum:raise ValueError('Long filename checksum mismatch')
                units=b''.join(lfn[i]for i in range(1,len(lfn)+1))
                codes=struct.unpack('<'+'H'*(len(units)//2),units);end=codes.index(0)if 0 in codes else len(codes)
                name=struct.pack('<'+'H'*end,*codes[:end]).decode('utf-16-le')
            else:
                base=alias[:8].decode('cp437').rstrip();ext=alias[8:].decode('cp437').rstrip()
                name=base+('.'+ext if ext else '')
            lfn={};expected=0
            if name in ('.','..'):continue
            if not name or any(c in name for c in '/\\\0:'):raise ValueError('Invalid filename component')
            if any(v.name.casefold()==name.casefold()for v in out):raise ValueError('Ambiguous FAT filename')
            c=(struct.unpack_from('<H',e,20)[0]<<16)|struct.unpack_from('<H',e,26)[0]
            out.append(Entry(name,c,struct.unpack_from('<I',e,28)[0],bool(e[11]&16)))
            if len(out)>512:raise ValueError('Directory exceeds entry bound')
        return out
    def find(self,parts):
        cluster=self.root_cluster
        for i,name in enumerate(parts):
            entry=next((v for v in self.directory(cluster)if v.name.casefold()==name.casefold()),None)
            if entry is None:raise FileNotFoundError('/'.join(parts))
            if i+1<len(parts):
                if not entry.directory:raise ValueError('Path component is not a directory')
                cluster=entry.cluster
        return entry
    def read(self,parts,limit=129*1024*1024):return self.read_entry(self.find(parts),limit)
    def walk(self,cluster=None,prefix=(),depth=0):
        if depth>16:raise ValueError('Directory nesting limit exceeded')
        for entry in self.directory(cluster):
            path=(*prefix,entry.name)
            if entry.directory:yield from self.walk(entry.cluster,path,depth+1)
            else:yield path,entry

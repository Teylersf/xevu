"""Generate public, synthetic media for tests. Never uses private input files."""
from pathlib import Path
import io,struct
from PIL import Image,ImageDraw
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'test-output/fixtures'
OUT.mkdir(parents=True,exist_ok=True)
frames=[]
for i in range(20):
    im=Image.new('RGB',(320,180),(14,20,15));d=ImageDraw.Draw(im)
    d.rectangle((20+i*8,45,75+i*8,100),fill=(196,243,107));d.text((20,145),'XEVU PUBLIC VIDEO FIXTURE',fill=(228,232,222))
    b=io.BytesIO();im.save(b,'JPEG',quality=80);frames.append(b.getvalue())
    if i==0:im.save(OUT/'PHOTO.JPG');im.save(OUT/'GRAPH.PNG')
(OUT/'VIDEO.NGM').write_bytes(b'NGMJPG01'+struct.pack('<II',5,len(frames))+b''.join(struct.pack('<I',len(f))+f for f in frames))
try:
    import fitz
    doc=fitz.open()
    for number in range(1,3):
        p=doc.new_page(width=640,height=360);p.insert_text((45,65),'XEVU / PUBLIC DOCUMENT FIXTURE',fontsize=22);p.insert_text((45,120),f'Page {number} / No sensitive content.',fontsize=16)
    doc.save(OUT/'PUBLIC.PDF');doc.close()
    import subprocess,sys
    subprocess.run([sys.executable,str(ROOT/'tools/prepare_document.py'),str(OUT/'PUBLIC.PDF'),str(OUT/'PAGES.NGD')],check=True)
except ImportError:print('PyMuPDF absent; PDF fixture omitted.')
print('Generated synthetic test fixtures:',OUT)

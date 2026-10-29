"""Render a PDF into native offline page images. Output is NOT encrypted.

Requires PyMuPDF (`python -m pip install pymupdf`) on a trusted preparation
computer. Import the .ngd into the vault afterwards. Does not modify the PDF.
"""
from pathlib import Path
import argparse,struct
def main():
    import fitz
    p=argparse.ArgumentParser();p.add_argument('input',type=Path);p.add_argument('output',type=Path);p.add_argument('--pages',type=int,default=100);a=p.parse_args()
    if a.output.exists():raise SystemExit('Output exists; refusing overwrite.')
    if not 1<=a.pages<=1000:raise SystemExit('Page bound must be 1..1000')
    document=fitz.open(a.input)
    if document.needs_pass:raise SystemExit('Unlock protected PDFs on your trusted preparation computer first.')
    count=min(document.page_count,a.pages)
    if count<1:raise SystemExit('No pages')
    size=16
    try:
        with a.output.open('xb') as out:
            out.write(b'NGDOC001'+struct.pack('<II',1,count))
            for i in range(count):
                page=document[i];scale=min(1280/page.rect.width,1800/page.rect.height)
                pixels=page.get_pixmap(matrix=fitz.Matrix(scale,scale),colorspace=fitz.csRGB,alpha=False)
                data=pixels.tobytes('jpeg',jpg_quality=82)
                size+=4+len(data)
                if len(data)>2*1024*1024 or size>128*1024*1024:raise ValueError('Native document memory bound exceeded; reduce page count')
                out.write(struct.pack('<I',len(data)));out.write(data)
    except Exception:
        a.output.unlink(missing_ok=True)
        raise
    print(f'Created {a.output}: {count} rendered pages. Import into the encrypted vault. Keep the original PDF separately if needed.')
if __name__=='__main__':main()

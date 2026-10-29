"""Render XEVU's original vector identity and marketing layouts.

Pillow is needed for raster assets; --pdf also needs ReportLab and PyMuPDF.
The generated abstract artwork is embedded as supplied, never synthesized here.
"""
from pathlib import Path
import argparse, hashlib, json
from PIL import Image, ImageDraw, ImageFont

ROOT=Path(__file__).resolve().parents[1]
BRAND=ROOT/'assets/brand'
MARKETING=ROOT/'marketing'
IDENTITY=json.loads((BRAND/'identity.json').read_text(encoding='utf-8'))
C=IDENTITY['colors']
MARK=IDENTITY['mark']
GLYPHS=[(0,MARK),(32,IDENTITY['letters']['E']),(60,IDENTITY['letters']['V']),(88,IDENTITY['letters']['U'])]
FONTS=ROOT/'assets/fonts'

def font(size,mono=False):
    return ImageFont.truetype(str(FONTS/('IBMPlexMono-Regular.ttf' if mono else 'BarlowSemiCondensed-SemiBold.ttf')),size)

def vector_parts(word=False):
    return [
        [(x+px,py)for px,py in poly]
        for x,polys in (GLYPHS if word else [(0,MARK)]) for poly in polys]

def svg(word,color):
    width=108 if word else 24
    paths=''.join('<polygon points="'+' '.join(f'{x},{y}'for x,y in p)+'"/>'for p in vector_parts(word))
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} 24" role="img" aria-label="XEVU"><title>XEVU</title><g fill="{color}">{paths}</g></svg>\n'

def draw_identity(im,xy,height=48,color=None,word=False):
    d=ImageDraw.Draw(im);scale=height/24;x,y=xy
    for poly in vector_parts(word):d.polygon([(round(x+px*scale),round(y+py*scale))for px,py in poly],fill=color or C['lime'])

def type_at(im,xy,value,size=30,color=None,mono=False):
    ImageDraw.Draw(im).text(xy,value,font=font(size,mono),fill=color or C['ivory'],anchor='lt')

def wrapped(im,xy,value,width,size=28,color=None,mono=False,leading=1.35):
    x,y=xy;f=font(size,mono);line=''
    for word in value.split():
        proposed=(line+' '+word).strip()
        if f.getlength(proposed)>width and line:
            type_at(im,(x,y),line,size,color,mono);y+=round(size*leading);line=word
        else:line=proposed
    if line:type_at(im,(x,y),line,size,color,mono)
    return y+round(size*leading)

def art_canvas(size):
    # A new editorial layout using the unaltered artwork as its background.
    from PIL import ImageOps
    return ImageOps.fit(Image.open(ROOT/'assets/xevu-artwork.png').convert('RGB'),size,method=Image.Resampling.LANCZOS)

def brand_assets():
    BRAND.mkdir(exist_ok=True);MARKETING.mkdir(exist_ok=True)
    for label,color in [('lime',C['lime']),('ivory',C['ivory']),('graphite',C['graphite'])]:
        for word,kind in [(False,'mark'),(True,'wordmark')]:
            (BRAND/f'xevu-{kind}-{label}.svg').write_text(svg(word,color),encoding='utf-8',newline='\n')
            scale=8 if word else 24
            im=Image.new('RGBA',((108 if word else 24)*scale,24*scale))
            draw_identity(im,(0,0),24*scale,color,word)
            im.save(BRAND/f'xevu-{kind}-{label}.png',optimize=True)
    rust='// Generated from assets/brand/identity.json by tools/brand_assets.py.\n'
    rust+="pub type Polygon = &'static [(i32, i32)];\n"
    rust+='pub const MARK: &[Polygon] = &[\n'
    rust+=''.join('    &['+', '.join(f'({x}, {y})'for x,y in p)+'],\n'for p in MARK)+'];\n'
    rust+='pub const WORD: &[(i32, &[Polygon])] = &[\n'
    for x,polys in GLYPHS:
        rust+='    ('+str(x)+', &[\n'+''.join('        &['+', '.join(f'({px}, {py})'for px,py in p)+'],\n'for p in polys)+'    ]),\n'
    rust+='];\n'
    native=ROOT/'crates/field-boot/src/ui/brand_mark.rs'
    native.parent.mkdir(parents=True,exist_ok=True)
    native.write_text(rust,encoding='utf-8',newline='\n')

    # Large launch banner, reusable as a README or social cover.
    im=art_canvas((1600,900));d=ImageDraw.Draw(im)
    draw_identity(im,(74,74),56,C['ivory'],True)
    type_at(im,(76,170),'OFFLINE FILE WORKSPACE',19,C['lime'],True)
    type_at(im,(68,260),'CARRY',112)
    type_at(im,(68,380),'WHAT MATTERS.',112)
    wrapped(im,(76,545),'Encrypted files. Explicit transfers. A workspace you can carry.',650,32,C['muted'])
    d.line((76,728,760,728),fill=C['line'],width=2)
    type_at(im,(76,758),'OPEN SOURCE  /  USB + SD  /  NATIVE UEFI',18,C['lime'],True)
    type_at(im,(76,843),'ENGINEERING PREVIEW  /  UNAUDITED  /  USE TEST DATA',15,C['muted'],True)
    im.save(MARKETING/'xevu-launch-1600x900.png',optimize=True)

    # Social card: original composition, not a crop that loses the message.
    for size,name in [((1200,630),'xevu-social-1200x630.png'),((1080,1080),'xevu-social-1080x1080.png'),((1080,1350),'xevu-social-1080x1350.png')]:
        w,h=size;im=Image.new('RGB',size,C['graphite']);d=ImageDraw.Draw(im)
        from PIL import ImageOps
        if h==630:
            im=art_canvas(size);d=ImageDraw.Draw(im)
            draw_identity(im,(58,52),42,C['ivory'],True)
            type_at(im,(56,156),'Carry what',74);type_at(im,(56,241),'matters.',74)
            wrapped(im,(60,354),'An offline workspace for your files.',500,29,C['muted'])
            type_at(im,(60,545),'OPEN SOURCE / ENGINEERING PREVIEW',15,C['lime'],True)
            type_at(im,(60,592),'Unaudited. Use test data.',14,C['muted'],True)
        else:
            art=ImageOps.fit(Image.open(ROOT/'assets/xevu-artwork.png').convert('RGB'),(w,620 if h==1350 else 470),method=Image.Resampling.LANCZOS,centering=(.8,.5))
            im.paste(art,(0,h-art.height-86))
            draw_identity(im,(66,66),56,C['ivory'],True)
            type_at(im,(62,192),'Carry what',94);type_at(im,(62,294),'matters.',94)
            wrapped(im,(68,425),'Your files. Your work. Your way.',900,31,C['muted'])
            d.line((68,h-88,w-68,h-88),fill=C['line'],width=2)
            type_at(im,(68,h-62),'OPEN SOURCE / ENGINEERING PREVIEW',17,C['lime'],True)
            type_at(im,(68,h-33),'Unaudited. Use test data.',14,C['muted'],True)
        im.save(MARKETING/name,optimize=True)

    # Identity overview for a fast visual review and press-kit cover.
    im=Image.new('RGB',(1600,1000),C['graphite']);d=ImageDraw.Draw(im)
    type_at(im,(70,54),'XEVU / IDENTITY 01',19,C['muted'],True)
    d.line((70,110,1530,110),fill=C['line'],width=2)
    draw_identity(im,(76,186),192,C['lime'])
    draw_identity(im,(400,214),112,C['ivory'],True)
    type_at(im,(402,356),'Carry what matters.',40,C['muted'])
    type_at(im,(76,503),'ONE WORKSPACE. EVERY KIND OF WORK.',36)
    wrapped(im,(78,568),'Creative projects, business documents, independent research and personal archives. Encrypted copies in the vault. Original files when you export.',1050,29,C['muted'])
    for i,(key,color)in enumerate([(k,C[k])for k in ['graphite','panel','ivory','lime']]):
        x=78+i*360;d.rectangle((x,730,x+326,824),fill=color,outline=C['line'],width=2)
        type_at(im,(x,850),key.upper(),18,C['ivory'],True);type_at(im,(x,883),color,18,C['muted'],True)
    type_at(im,(78,956),'ORIGINAL VECTOR IDENTITY / BOUNDED SECURITY CLAIMS / ENGINEERING PREVIEW',15,C['muted'],True)
    im.save(MARKETING/'xevu-brand-board.png',optimize=True)

def make_pdfs():
    from reportlab.pdfgen import canvas
    from reportlab.lib.colors import HexColor
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.lib.utils import ImageReader
    import fitz
    output=ROOT/'output/pdf';output.mkdir(parents=True,exist_ok=True)
    pdfmetrics.registerFont(TTFont('Barlow',str(FONTS/'BarlowSemiCondensed-SemiBold.ttf')))
    pdfmetrics.registerFont(TTFont('Plex',str(FONTS/'IBMPlexMono-Regular.ttf')))
    width,height=612,792
    def page(c):c.setFillColor(HexColor(C['graphite']));c.rect(0,0,width,height,fill=1,stroke=0)
    def txt(c,x,y,value,size=12,color='ivory',mono=False):
        c.setFillColor(HexColor(C[color]));c.setFont('Plex'if mono else'Barlow',size);c.drawString(x,y,value)
    def para(c,x,y,value,available=516,size=12,color='muted',mono=False):
        face='Plex'if mono else'Barlow';line=''
        for word in value.split():
            candidate=(line+' '+word).strip()
            if pdfmetrics.stringWidth(candidate,face,size)>available and line:
                txt(c,x,y,line,size,color,mono);y-=size*1.45;line=word
            else:line=candidate
        if line:txt(c,x,y,line,size,color,mono);y-=size*1.45
        return y
    def wordmark(c,x,y,h=32,color='ivory'):
        c.setFillColor(HexColor(C[color]));s=h/24
        for poly in vector_parts(True):
            path=c.beginPath();path.moveTo(x+poly[0][0]*s,y+h-poly[0][1]*s)
            for px,py in poly[1:]:path.lineTo(x+px*s,y+h-py*s)
            path.close();c.drawPath(path,fill=1,stroke=0)
    def footer(c,number):
        txt(c,48,31,'XEVU / ENGINEERING PREVIEW / UNAUDITED / USE TEST DATA',8,'muted',True)
        txt(c,553,31,f'{number:02}',8,'muted',True)
    path=output/'xevu-product-sheet.pdf';c=canvas.Canvas(str(path),pagesize=(width,height));c.setTitle('XEVU - Carry what matters')
    page(c);wordmark(c,48,708,34);txt(c,48,672,'OFFLINE FILE WORKSPACE',10,'lime',True)
    txt(c,46,606,'Carry what matters.',48)
    para(c,48,566,'Encrypted files. Explicit transfers. A workspace you can carry.',size=16)
    c.drawImage(ImageReader(str(ROOT/'assets/xevu-artwork.png')),48,292,width=516,height=246,mask='auto',preserveAspectRatio=True,anchor='c')
    y=284
    for title,body in [
        ('01 / KEEP YOUR ORIGINALS','Import documents, photos, video and other files as encrypted objects with hidden original names.'),
        ('02 / CHOOSE WHAT MOVES','Select source drives explicitly. Export a normal file with its original name and exact contents.'),
        ('03 / TAKE IT WITH YOU','Boot from supported USB/SD media, or explore the native preview in a virtual machine.')]:
        txt(c,48,y,title,12,'lime',True);y=para(c,48,y-23,body,size=12)-17
    txt(c,48,77,'github.com/Teylersf/xevu',12)
    txt(c,48,57,'Firmware-backed preview. Limited native viewers. FAT destinations; C: via Windows helper.',8,'muted')
    txt(c,48,44,'Three failed native unlocks erase the boot medium. Keep separate backups.',8,'muted')
    footer(c,1);c.showPage();c.save()

    path=output/'xevu-brand-guide.pdf';c=canvas.Canvas(str(path),pagesize=(width,height));c.setTitle('XEVU - Identity and launch guide')
    page(c);wordmark(c,48,681,55);txt(c,48,639,'IDENTITY + LAUNCH GUIDE / 01',10,'lime',True)
    txt(c,46,562,'Carry what matters.',42)
    para(c,48,518,'XEVU is pronounced zeh-voo. A short, abstract identity for an offline file workspace across creative, professional, research and personal uses.',size=16)
    y=393
    for title,body in [('POSITION','A portable, offline file workspace. Encrypted copies stay in the vault; explicit exports restore original files.'),('THE MARK','Four paths meet around an open center. The split X suggests controlled exchange without a shield, padlock or industry emblem.'),('THE VOICE','Direct, capable and calm. Show what works. Name the preview limits. Avoid fear, secrecy theater and absolute security claims.')]:
        txt(c,48,y,title,11,'lime',True);y=para(c,48,y-24,body,size=14)-28
    footer(c,1);c.showPage()
    page(c);txt(c,48,724,'VISUAL SYSTEM',30);txt(c,48,688,'GRAPHITE / IVORY / LIME',10,'lime',True)
    for i,key in enumerate(['graphite','panel','ivory','lime']):
        x=48+i*133;c.setFillColor(HexColor(C[key]));c.setStrokeColor(HexColor(C['line']));c.rect(x,530,115,110,fill=1,stroke=1)
        txt(c,x,507,key.upper(),10,'ivory',True);txt(c,x,488,C[key],10,'muted',True)
    y=421
    for title,body in [('TYPOGRAPHY','Original geometric vector wordmark. Barlow Semi Condensed SemiBold for headlines. IBM Plex Mono for labels and technical details. Fonts retain their OFL licenses.'),('LOGO USE','Keep the mark open and monochrome. Use lime on graphite, ivory on dark surfaces, or graphite on light surfaces. Minimum clear space: one quarter of the mark height. Minimum digital mark size: 24 px.'),('ART DIRECTION','Abstract manufactured forms, tactile graphite and titanium, one restrained lime edge. Neutral to industry. No military insignia, weapons, rockets, antigravity devices or compliance seals.')]:
        txt(c,48,y,title,11,'lime',True);y=para(c,48,y-24,body,size=14)-27
    footer(c,2);c.showPage()
    page(c);txt(c,48,724,'LAUNCH + CLAIMS',30);txt(c,48,688,'CLEAR COPY. EVIDENCE FIRST.',10,'lime',True)
    y=634
    for title,body in [
        ('SHORT DESCRIPTION','XEVU is an open-source, offline file workspace for carrying encrypted documents and media, with explicit drive access and original-file export.'),
        ('LAUNCH POST','Meet XEVU. Carry what matters: project files, documents, media and personal archives. Try the native USB/SD preview or VM, explore the code, and help shape what comes next.'),
        ('SUPPORTED CLAIMS','Encrypted vault contents and names. Explicit drive selection. Verified original-file export. No network stack or telemetry in the native image. Public source and verification evidence.'),
        ('PREVIEW LIMITS','This is firmware-backed UEFI software, not a completed standalone OS. It is unsigned and unaudited. No government or regulatory approval is claimed. Native viewers and filesystems are limited. Use test data. Three failed native unlocks erase the boot medium.'),
        ('NAME STATUS','Public web screening found no obvious same-name operating-system product, but XEVU appears in unrelated names. No trademark or domain clearance is claimed. See marketing/NAME_RESEARCH.md.')]:
        txt(c,48,y,title,11,'lime',True);y=para(c,48,y-24,body,size=13)-26
    footer(c,3);c.showPage();c.save()
    # Independent PDF reopen/text checks and rasterized proofs for visual QA.
    qa=ROOT/'test-output/brand-pdf';qa.mkdir(parents=True,exist_ok=True)
    for path in output.glob('xevu-*.pdf'):
        with fitz.open(path)as doc:
            assert 'XEVU'in ''.join(p.get_text()for p in doc)
            for i,p in enumerate(doc):p.get_pixmap(matrix=fitz.Matrix(1.5,1.5)).save(qa/f'{path.stem}-{i+1}.png')

def manifest():
    files=sorted(p for folder in [BRAND,MARKETING,ROOT/'output/pdf']for p in folder.rglob('*')if p.is_file()and p.name!='manifest.json')
    data={p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()for p in files}
    (MARKETING/'manifest.json').write_text(json.dumps({'brand':'XEVU','sha256':data},indent=2)+'\n',encoding='utf-8',newline='\n')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--pdf',action='store_true');a=p.parse_args()
    brand_assets()
    if a.pdf:make_pdfs()
    manifest();print('Prepared XEVU vector identity, transparent logos and marketing layouts.'+(' PDFs rendered for visual review.'if a.pdf else''))

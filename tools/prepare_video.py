"""Convert a video to bounded silent native MJPEG. Does NOT encrypt the result.

Only run this on a trusted preparation computer. The output is plaintext until
imported with fieldctl. Original video is never changed. FFmpeg is required.
"""
from pathlib import Path
import argparse,struct,subprocess,tempfile
def main():
    p=argparse.ArgumentParser();p.add_argument('input',type=Path);p.add_argument('output',type=Path);p.add_argument('--fps',type=int,default=10);p.add_argument('--seconds',type=int,default=60);a=p.parse_args()
    if a.output.exists():raise SystemExit('Output exists; refusing overwrite.')
    if not 1<=a.fps<=30 or not 1<=a.seconds<=300:raise SystemExit('fps 1..30; seconds 1..300')
    with tempfile.TemporaryDirectory(prefix='xevu-video-') as temp:
        pattern=str(Path(temp)/'frame-%06d.jpg')
        subprocess.run(['ffmpeg','-nostdin','-v','error','-i',str(a.input),'-t',str(a.seconds),'-vf',f'fps={a.fps},scale=640:360:force_original_aspect_ratio=decrease','-q:v','5','-an',pattern],check=True)
        files=sorted(Path(temp).glob('*.jpg'))
        if not files or len(files)>10000:raise SystemExit('No frames or excessive frame count')
        total=16+sum(4+f.stat().st_size for f in files)
        if total>128*1024*1024:raise SystemExit('Converted video exceeds the native memory bound; reduce duration/resolution.')
        with a.output.open('xb') as out:
            out.write(b'NGMJPG01'+struct.pack('<II',a.fps,len(files)))
            for f in files:
                data=f.read_bytes();out.write(struct.pack('<I',len(data)));out.write(data)
    print(f'Created {a.output}: {len(files)} frames, silent. Import it into the encrypted vault.')
if __name__=='__main__':main()

"""A complete product film: real native captures, editorial planar camera moves."""
import json
import subprocess
import sys
from pathlib import Path

source, output = map(Path, sys.argv[1:3])
output.mkdir(parents=True, exist_ok=True)
font = Path(sys.argv[3]) if len(sys.argv) > 3 else Path('/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc')
if not font.is_file():
    raise SystemExit('Supply a CJK font as the third argument (film only).')
W, H, FPS = 1280, 800, 24
BG = '0xF4F4F7'
scenes = []


def export(args):
    print(f'Exporting {args[-1]}', flush=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-filter_threads', '1',
                    '-filter_complex_threads', '1', *args[:-1], '-threads', '2', args[-1]],
                   check=True, timeout=180)


def label(value, name, size, x, y, color='0x344154'):
    path = output / f'{name}.txt'
    path.write_text(value, encoding='utf-8')
    return f"drawtext=fontfile='{font}':textfile='{path}':fontsize={size}:fontcolor={color}:x={x}:y={y}"


def canvas():
    return ['-f', 'lavfi', '-i', f'color=c={BG}:s={W}x{H}:r={FPS}']


def scene(name, duration, inputs, graph):
    path = output / f'{name}.mp4'
    graph += f',fade=t=in:st=0:d=0.24:color={BG},fade=t=out:st={duration-0.24}:d=0.24:color={BG},setsar=1,format=yuv420p[v]'
    export([*inputs, '-filter_complex', graph, '-map', '[v]', '-t', str(duration),
            '-r', str(FPS), '-c:v', 'libx264', '-preset', 'fast', '-crf', '19', str(path)])
    scenes.append((path, duration))


def hero(name, image, heading, detail, duration=4, reverse=False):
    # A native recording is a texture on a moving plane. No app state/material is fabricated.
    ease = f'(0.5-0.5*cos(PI*in/{duration*FPS-1}))'
    tilt = ease if reverse else f'(1-{ease})'
    perspective = (f'perspective=x0=180*{tilt}:y0=-120*{tilt}:'
                   f'x1=W-120*{tilt}:y1=170*{tilt}:x2=30*{tilt}:y2=H+95*{tilt}:'
                   f'x3=W-40*{tilt}:y3=H-90*{tilt}:sense=destination:eval=frame')
    graph = ('[0:v]format=rgba,scale=500:-1,pad=1280:800:70:(oh-ih)/2:color=black@0,'
             + perspective + ',split[face][mask];'
             '[mask]colorchannelmixer=rr=0:gg=0:bb=0:aa=0.13,gblur=sigma=12[shadow];'
             '[1:v][shadow]overlay=14:22[base];[base][face]overlay=0:0,'
             + label(heading, name+'-title', 36, 850, 290) + ','
             + label(detail, name+'-detail', 21, 850, 356, '0x86868B'))
    scene(name, duration, ['-loop', '1', '-framerate', str(FPS), '-i', str(source/image), *canvas()], graph)


hero('01-hero', '01-decision.png', '一瞬，决断。', '选择，回到你手中。')
scene('02-fullscreen', 4, ['-framerate', '20', '-i', str(source/'fullscreen-%03d.png')],
      '[0:v]scale=1280:800:force_original_aspect_ratio=increase,crop=1280:800,'
      'tpad=stop_mode=clone:stop_duration=4,'
      + label('按一下，答案浮现。', 'fullscreen-title', 34, '(w-tw)/2', 72) + ','
      + label('Ctrl + Alt + Space · 只有文字，没有遮罩', 'fullscreen-note', 19, '(w-tw)/2', 132, '0x697586'))
scene('03-widget', 4, ['-framerate', '20', '-i', str(source/'widget-%03d.png')],
      '[0:v]scale=1536:960:force_original_aspect_ratio=increase,'
      "crop=1280:800:x='(iw-ow)/2+18*sin(t*0.7)':y='(ih-oh)/2-10*cos(t*0.7)',"
      'tpad=stop_mode=clone:stop_duration=4,'
      + label('让答案，留在身边。', 'widget-title', 34, '(w-tw)/2', 72) + ','
      + label('常驻浮窗 · 背景透明度独立调整', 'widget-note', 19, '(w-tw)/2', 132, '0x697586'))
hero('04-goal', '01-decision.png', '答案，由你命名。', '目标、选项、快捷键。', reverse=True)
hero('05-layout', '05-widget-layout.png', '放在顺手的位置。', '拖动、缩放，也能精确调整。')
hero('06-appearance', '03-appearance.png', '每次出现，刚刚好。', '动画、透明度、字号与时长。', reverse=True)
for kind, title in [('clear', '清晰纯色'), ('glass', '流光玻璃'), ('script', '柔彩手写')]:
    scene('07-art-'+kind, 2, ['-loop', '1', '-framerate', str(FPS), '-i', str(source/('art-'+kind+'.png'))],
          '[0:v]scale=1344:840:force_original_aspect_ratio=increase,'
          "crop=1280:800:x='(iw-ow)/2+8*sin(t)':y='(ih-oh)/2',"
          + label(title, 'art-'+kind+'-title', 30, '(w-tw)/2', 80) + ','
          + label('原生字效预览 · 示例答案', 'art-'+kind+'-note', 15, '(w-tw)/2', 744, '0x86868B'))
hero('08-cloud', '02-models.png', '连接你信任的模型。', '专用决策与常规模型。', duration=3)
hero('09-local', '09-local.png', '也可以，留在本机。', '连接已运行的本地服务。', duration=3, reverse=True)
brand = Path(__file__).resolve().parents[2]/'docs/assets/lockup.png'
scene('10-signature', 3, ['-loop', '1', '-framerate', str(FPS), '-i', str(brand), *canvas()],
      '[0:v]scale=900:-1[brand];[1:v][brand]overlay=x=(W-w)/2:y=230-12*cos(t),'
      + label('给眼前的选择，一个答案。', 'signature', 32, '(w-tw)/2', 500) + ','
      + label('按需读取 · 本地与云端 · 默认不保存画面', 'privacy', 17, '(w-tw)/2', 585, '0x86868B'))
listing = output/'scenes.txt'
listing.write_text('\n'.join(f"file '{p.resolve()}'" for p, _ in scenes))
video = output/'blink-verdict-showcase.mp4'
export(['-f', 'concat', '-safe', '0', '-i', str(listing), '-c', 'copy', '-movflags', '+faststart', str(video)])

# All chapters remain in the GIF; speed and palette are bounded for mobile delivery.
# Use RGB PNG frames and Pillow to avoid version-dependent FFmpeg GIF palette conversion.
from PIL import Image
frames_dir = output/'gif-frames'
frames_dir.mkdir(exist_ok=True)
export(['-i', str(video), '-vf', 'setpts=PTS/1.5,fps=5,scale=512:320:flags=lanczos', str(frames_dir/'%04d.png')])
paths = sorted(frames_dir.glob('*.png'))
sample = Image.new('RGB', (160*8, 100*3))
for index in range(24):
    with Image.open(paths[min(len(paths)-1, index*len(paths)//24)]) as frame:
        sample.paste(frame.resize((160, 100)), (index % 8 * 160, index // 8 * 100))
palette = sample.quantize(colors=64, method=Image.Quantize.MEDIANCUT)
frames = []
for path in paths:
    with Image.open(path) as frame:
        frames.append(frame.convert('RGB').quantize(palette=palette, dither=Image.Dither.NONE))
frames[0].save(output/'showcase.gif', save_all=True, append_images=frames[1:],
               duration=200, loop=0, optimize=True, disposal=1)
provenance = json.loads((source/'provenance.json').read_text(encoding='utf-8-sig'))
provenance['editing'] = 'Complete native product tour; planar perspective, eased camera moves, independent shadows, cuts and captions. No simulated app state or live material effects.'
provenance['chapters'] = [p.stem for p, _ in scenes]
provenance['film_seconds'] = sum(d for _, d in scenes)
provenance['readme_preview'] = {'width': 512, 'height': 320, 'fps': 5, 'all_chapters': True,
                                'bytes': (output/'showcase.gif').stat().st_size}
if (source/'idle-sample.json').exists():
    provenance['hidden_host_sample'] = json.loads((source/'idle-sample.json').read_text(encoding='utf-8-sig'))
(output/'showcase.json').write_text(json.dumps(provenance, ensure_ascii=False, indent=2)+'\n')
print('GIF bytes:', (output/'showcase.gif').stat().st_size, flush=True)

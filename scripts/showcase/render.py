"""A quiet product film, edited from the Windows executable's native previews."""
import json
import subprocess
import sys
from pathlib import Path

source = Path(sys.argv[1])
output = Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)
font = Path(sys.argv[3]) if len(sys.argv) > 3 else Path('/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc')
if not font.is_file():
    raise SystemExit('Supply a CJK font as the third argument; it is used only for film titles.')
background = '0xF4F4F7'


def export(arguments):
    print(f'Exporting {arguments[-1]}', flush=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-filter_threads', '1',
                    '-filter_complex_threads', '1', *arguments[:-1], '-threads', '2', arguments[-1]],
                   check=True, timeout=180)


def text(value, name, size, x, y, color='0x1D1D1F'):
    path = output / f'{name}.txt'
    path.write_text(value, encoding='utf-8')
    return f"drawtext=fontfile='{font}':textfile='{path}':fontsize={size}:fontcolor={color}:x={x}:y={y}"


segments = []


def scene(name, duration, inputs, graph):
    path = output / f'{name}.mp4'
    graph += f',fade=t=in:st=0:d=0.45:color={background},fade=t=out:st={duration-0.45}:d=0.45:color={background},setsar=1,format=yuv420p[v]'
    export([*inputs, '-filter_complex', graph, '-map', '[v]', '-t', str(duration), '-r', '30',
            '-c:v', 'libx264', '-preset', 'fast', '-crf', '19', str(path)])
    segments.append(path)


canvas = ['-f', 'lavfi', '-i', f'color=c={background}:s=1280x800:r=30']
scene('01-introduction', 3, canvas,
      '[0:v]' + text('BlinkVerdict', 'brand', 28, '(w-tw)/2', 232) + ','
      + text('给眼前的选择，一个答案。', 'intro', 46, '(w-tw)/2', 330) + ','
      + text('一瞬决断', 'name', 20, '(w-tw)/2', 430, '0x8A8A8E'))

# These are full desktop crops of the real native overlay, including its real backdrop.
# Editorial scaling never adds blur, refraction, or an invented app state.
for prefix, duration, heading, subtitle in [
    ('fullscreen', 4, '按一下，让答案浮现。', 'Ctrl + Alt + Space'),
]:
    scene('02-one-shortcut', duration,
          ['-framerate', '20', '-i', str(source / f'{prefix}-%03d.png')],
          '[0:v]scale=1280:800:force_original_aspect_ratio=increase,crop=1280:800,'
          'tpad=stop_mode=clone:stop_duration=4,'
          + text(heading, 'shortcut-title', 32, '(w-tw)/2', 72) + ','
          + text(subtitle, 'shortcut-keys', 19, '(w-tw)/2', 132, '0x69696D') + ','
          + text('外观预览 · 示例答案', 'sample', 14, '(w-tw)/2', 742, '0x8A8A8E'))

scene('03-your-choice', 5,
      ['-loop', '1', '-i', str(source / '01-decision.png'), *canvas],
      '[0:v]scale=550:670:force_original_aspect_ratio=decrease[ui];'
      '[1:v][ui]overlay=100:(H-h)/2,'
      + text('目标由你设定。', 'goal', 36, 744, 300) + ','
      + text('答案，也由你命名。', 'answers', 23, 744, 365, '0x8A8A8E'))

scene('04-always-near', 5,
      ['-framerate', '20', '-i', str(source / 'widget-%03d.png')],
      '[0:v]scale=1536:960:force_original_aspect_ratio=increase,crop=1280:800,'
      'tpad=stop_mode=clone:stop_duration=5,'
      + text('留在身边。', 'widget-title', 34, '(w-tw)/2', 82) + ','
      + text('等待你的下一次按键', 'widget-subtitle', 20, '(w-tw)/2', 142, '0x69696D') + ','
      + text('外观预览 · 示例答案', 'widget-sample', 14, '(w-tw)/2', 742, '0x8A8A8E'))

scene('05-your-rhythm', 4,
      ['-loop', '1', '-i', str(source / '03-appearance.png'), *canvas],
      '[0:v]scale=550:670:force_original_aspect_ratio=decrease[ui];'
      '[1:v][ui]overlay=100:(H-h)/2,'
      + text('轻柔浮现。', 'rhythm-title', 36, 744, 300) + ','
      + text('恰到好处。', 'rhythm-subtitle', 23, 744, 365, '0x8A8A8E'))

scene('06-signature', 3, canvas,
      '[0:v]' + text('BlinkVerdict', 'end-brand', 46, '(w-tw)/2', 300) + ','
      + text('一瞬决断', 'end-name', 23, '(w-tw)/2', 382, '0x8A8A8E') + ','
      + text('Windows · 本地与云端模型', 'end-note', 17, '(w-tw)/2', 566, '0x8A8A8E'))

listing = output / 'scenes.txt'
listing.write_text('\n'.join(f"file '{p.resolve()}'" for p in segments))
video = output / 'blink-verdict-showcase.mp4'
export(['-f', 'concat', '-safe', '0', '-i', str(listing), '-c', 'copy', '-movflags', '+faststart', str(video)])
# README gets a short loop focused on the two modes, without the settings tour.
loop_listing = output / 'loop.txt'
loop_listing.write_text('\n'.join(f"file '{p.resolve()}'" for p in [segments[1], segments[3]]))
loop = output / 'loop.mp4'
export(['-f', 'concat', '-safe', '0', '-i', str(loop_listing), '-t', '8', '-c', 'copy', str(loop)])
export(['-i', str(loop), '-filter_complex',
        'fps=18,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle',
        '-loop', '0', str(output / 'showcase.gif')])
provenance = json.loads((source / 'provenance.json').read_text(encoding='utf-8-sig'))
provenance['editing'] = '24-second quiet product film; 8-second README loop; native previews with sample YES. Titles, cuts, scaling and white fades only. No simulated glass, no model requests.'
(output / 'showcase.json').write_text(json.dumps(provenance, ensure_ascii=False, indent=2) + '\n')

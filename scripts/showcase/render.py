"""Edit genuine Windows-native captures into a short showcase. No UI mockups."""
import json
import subprocess
import sys
from pathlib import Path

source = Path(sys.argv[1])
output = Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)
scenes = [
    ("01-decision.png", "01 / MAKE IT YOURS", "A clear goal. Your own answers."),
    ("02-models.png", "02 / CONNECT YOUR MODEL", "Native decisions. Local or cloud."),
    ("03-appearance.png", "03 / LESS, BUT BETTER", "One visual language. Your own rhythm."),
    ("04-wordart.png", "04 / A LITTLE CHARACTER", "Choose the expression that fits."),
    ("05-widget-layout.png", "05 / FIND YOUR PLACE", "Move. Resize. Lock. Stay focused."),
]
font = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
segments = []
for i, (filename, title, subtitle) in enumerate(scenes):
    segment = output / f"scene-{i}.mp4"
    graph = (
        "[0:v]scale=960:710:force_original_aspect_ratio=decrease,"
        "pad=960:710:(ow-iw)/2:(oh-ih)/2:color=0x111722[ui];"
        "[1:v][ui]overlay=(W-w)/2:126,"
        f"drawtext=fontfile={font}:text='{title}':fontsize=24:fontcolor=0xb9d8ff:x=(w-tw)/2:y=34,"
        f"drawtext=fontfile={font}:text='{subtitle}':fontsize=16:fontcolor=0x9caec3:x=(w-tw)/2:y=76,"
        "fade=t=in:st=0:d=0.22,fade=t=out:st=1.7:d=0.22,format=yuv420p[v]"
    )
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-loop", "1", "-i", str(source / filename), "-f", "lavfi", "-i", "color=c=0x0a101b:s=1280x900:r=15", "-filter_complex", graph, "-map", "[v]", "-t", "1.95", "-r", "15", "-c:v", "libx264", "-preset", "fast", str(segment)], check=True)
    segments.append(segment)
# A real sequence of native overlay frames is edited as the final scene.
segment = output / "scene-5.mp4"
graph = (
    "[0:v]scale=640:-1[ui];[1:v][ui]overlay=(W-w)/2:(H-h)/2,"
    f"drawtext=fontfile={font}:text='BLINKVERDICT':fontsize=38:fontcolor=0xe7effa:x=(w-tw)/2:y=140,"
    f"drawtext=fontfile={font}:text='One shortcut. A moment of clarity.':fontsize=22:fontcolor=0xaec9eb:x=(w-tw)/2:y=205,"
    f"drawtext=fontfile={font}:text='Windows native capture / edited presentation':fontsize=14:fontcolor=0x8795a9:x=(w-tw)/2:y=826,"
    "fade=t=in:st=0:d=0.2,fade=t=out:st=1.6:d=0.3,format=yuv420p[v]"
)
subprocess.run(["ffmpeg", "-v", "error", "-y", "-framerate", "15", "-i", str(source / "widget-%03d.png"), "-f", "lavfi", "-i", "color=c=0x0a101b:s=1280x900:r=15", "-filter_complex", graph, "-map", "[v]", "-t", "1.95", "-c:v", "libx264", "-preset", "fast", str(segment)], check=True)
segments.append(segment)
listing = output / "scenes.txt"
listing.write_text("\n".join(f"file '{p.resolve()}'" for p in segments))
video = output / "blink-verdict-showcase.mp4"
subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", str(listing), "-c", "copy", "-movflags", "+faststart", str(video)], check=True)
subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(video), "-filter_complex", "fps=12,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle", "-loop", "0", str(output / "showcase.gif")], check=True)
provenance = json.loads((source / "provenance.json").read_text(encoding="utf-8-sig"))
provenance["editing"] = "Native window captures; editorial titles, scaling, fades and cuts; 12 fps GIF. Sample YES from the built-in preview; no model request."
(output / "showcase.json").write_text(json.dumps(provenance, ensure_ascii=False, indent=2) + "\n")

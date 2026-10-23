"""Build an allowlisted Windows candidate archive, licenses, manifest and SHA-256."""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parents[2]
os.chdir(root)
version = tomllib.loads(Path('Cargo.toml').read_text())['package']['version']
development = sys.argv[1:] == ['--development']
if sys.argv[1:] and not development:
    raise SystemExit('Usage: package.py [--development]')
if not development and '-rc.' not in version:
    raise SystemExit('This preparation workflow only packages release candidates. Complete stable gates first.')
name = 'blink-verdict-windows-x64-preview' if development else f'blink-verdict-{version}-windows-x64'
stage = Path('dist') / name
stage.mkdir(parents=True, exist_ok=False)
for source, dest in [('target/release/blink-verdict.exe', 'blink-verdict.exe'),
                     ('docs/portable.md', 'README.md'), ('LICENSE', 'LICENSE'),
                     ('CHANGELOG.md', 'CHANGELOG.md')]:
    shutil.copyfile(source, stage / dest)

metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1']))
notices = ['BlinkVerdict third-party notices\n\nIncludes build-time and target-specific crates from Cargo.lock.\n']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['name'] == 'blink-verdict':
        continue
    directory = Path(package['manifest_path']).parent
    texts = [f for f in directory.iterdir() if f.is_file() and
             (f.name.lower().startswith(('license', 'unlicense', 'copying', 'notice')))]
    if not texts:
        raise SystemExit(f'Missing license text for {package["name"]}')
    notices.append(f'\n{"="*72}\n{package["name"]} {package["version"]}\nSPDX: {package["license"]}\n')
    for text in sorted(texts):
        notices.append(f'\n--- {text.name} ---\n{text.read_text(encoding="utf-8")}\n')
(stage / 'THIRD_PARTY_NOTICES.txt').write_text(''.join(notices), encoding='utf-8')
# Rust's standard library is statically linked but is outside Cargo metadata.
sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
library_notice = sysroot / 'share/doc/rust/COPYRIGHT-library.html'
shutil.copyfile(library_notice, stage / 'RUST_LIBRARY_NOTICES.html')
commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
manifest = {
    'version': version, 'source_commit': commit, 'target': 'x86_64-pc-windows-msvc',
    'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
    'cargo_lock_sha256': hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest(),
    'authenticode_signed': False, 'status': 'development preview' if development else 'release candidate; manual acceptance gates remain',
    'files': {f.name: hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(stage.iterdir())},
}
(stage / 'build-info.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
archive = stage.parent / (stage.name + '.zip')
with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
    for file in sorted(stage.iterdir()):
        output.write(file, arcname=f'{stage.name}/{file.name}')
checksum = f'{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n'
Path('dist/SHA256SUMS.txt').write_text(checksum, encoding='ascii')
print(f'Candidate: {archive.name}; {archive.stat().st_size} bytes')
print(checksum, end='')

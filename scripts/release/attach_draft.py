"""Attach a verified artifact to an existing draft. Never create/publish/replace a release."""
import hashlib
import json
import subprocess
import sys
import tomllib
import zipfile
from pathlib import Path

folder = Path(sys.argv[1])
version = tomllib.loads(Path('Cargo.toml').read_text())['package']['version']
checksums = folder / 'SHA256SUMS.txt'
lines = checksums.read_text().splitlines()
if len(lines) != 1:
    raise SystemExit('Expected exactly one candidate archive')
digest, name = lines[0].split()
if Path(name).name != name or name != f'blink-verdict-{version}-windows-x64.zip':
    raise SystemExit('Unexpected candidate filename')
archive = folder / name
if hashlib.sha256(archive.read_bytes()).hexdigest() != digest:
    raise SystemExit('Candidate checksum mismatch')
with zipfile.ZipFile(archive) as candidate:
    manifest = next(n for n in candidate.namelist() if n.endswith('/build-info.json'))
    root = manifest.rsplit('/', 1)[0] + '/'
    build = json.loads(candidate.read(manifest))
    for filename, expected in build['files'].items():
        if hashlib.sha256(candidate.read(root + filename)).hexdigest() != expected:
            raise SystemExit('Internal file checksum mismatch')
    if build['version'] != version or build['target'] != 'x86_64-pc-windows-msvc':
        raise SystemExit('Candidate version/target mismatch')
tag = 'v' + version
release = json.loads(subprocess.check_output([
    'gh', 'release', 'view', tag, '--json', 'isDraft,isPrerelease,targetCommitish,assets']))
if not release['isDraft'] or not release['isPrerelease'] or release['targetCommitish'] != build['source_commit']:
    raise SystemExit('Only the matching existing draft prerelease can receive this artifact')
attachments = [archive, checksums, *sorted(Path('docs/releases').glob(version + '*'))]
existing = {a['name'] for a in release['assets']}
if existing.intersection(p.name for p in attachments):
    raise SystemExit('An attachment already exists; refusing to replace it')
subprocess.run(['gh', 'release', 'upload', tag, *map(str, attachments)], check=True)
print('Verified candidate attached; release stays draft and unpublished.')

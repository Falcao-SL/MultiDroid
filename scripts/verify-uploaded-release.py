import hashlib,json,sys
from pathlib import Path
release=json.loads(Path(sys.argv[1]).read_text())
if not release['draft']:raise SystemExit('New uploads must be verified in a draft release')
files={p.name:p for p in Path('out').iterdir() if p.is_file() and p.name!='RELEASE-NOTES.md'}
assets={asset['name']:asset for asset in release['assets']}
if set(files)!=set(assets):raise SystemExit('Draft release has an incomplete or unexpected artifact set')
for name,path in files.items():
 asset=assets[name]
 if asset['state']!='uploaded' or asset['size']!=path.stat().st_size:raise SystemExit('Upload size/state mismatch: '+name)
 if asset.get('digest'):
  with path.open('rb') as stream:expected='sha256:'+hashlib.file_digest(stream,'sha256').hexdigest()
  if expected!=asset['digest']:raise SystemExit('Upload digest mismatch: '+name)
print('Draft assets verified; ready for atomic public release.')

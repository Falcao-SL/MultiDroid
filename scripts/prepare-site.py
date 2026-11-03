#!/usr/bin/env python3
"""Stage static HTML with content-based asset URLs; no runtime JS or cache purges required."""
import argparse,hashlib,re,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args();output=args.output.resolve();source=ROOT/'site'
if output==ROOT or output in ROOT.parents or output==source or source in output.parents:
 raise SystemExit('Use a separate empty staging directory')
if output.exists() and any(output.iterdir()):raise SystemExit('Staging directory must be empty')
shutil.copytree(source,output,dirs_exist_ok=True)
digests={}
def replace(match):
 relative=match.group(1)
 asset=(output/relative).resolve()
 if output not in asset.parents or not asset.is_file():raise SystemExit('Invalid or missing site asset: '+relative)
 if relative not in digests:digests[relative]=hashlib.sha256(asset.read_bytes()).hexdigest()[:12]
 return relative+'?v='+digests[relative]
pattern=r'(assets/(?:[A-Za-z0-9_.-]+/)*[A-Za-z0-9_.-]+\.(?:css|png|svg|ico|webp|jpg|mp4))(?:\?v=[A-Za-z0-9._-]+)?'
for page in output.rglob('*.html'):
 page.write_text(re.sub(pattern,replace,page.read_text()))
print(f'Staged static site with {len(digests)} content-versioned assets.')

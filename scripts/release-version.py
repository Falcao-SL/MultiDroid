import os,re,tomllib
from pathlib import Path
root=Path(__file__).resolve().parents[1]
tag=os.environ['REQUESTED_TAG']
if not re.fullmatch(r'v\d+\.\d+(?:\.\d+)?(?:-[A-Za-z0-9.]+)?',tag): raise SystemExit('Invalid release tag')
version=tag[1:]; base=version.split('-')[0]
rust=tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['package']['version']
native=version==(root/'mac/VERSION').read_text().strip()
if not native and base!=rust: raise SystemExit('Version must match rust/Cargo.toml or mac/VERSION')
with open(os.environ['GITHUB_OUTPUT'],'a') as output:
 for key,value in {'tag':tag,'name':version,'native':str(native).lower(),'preview':str('-' in version).lower()}.items(): output.write(f'{key}={value}\n')
print('Validated',tag,'native Mac' if native else 'desktop preview')

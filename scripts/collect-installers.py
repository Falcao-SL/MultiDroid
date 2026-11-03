import os,shutil
from pathlib import Path
root=Path(__file__).resolve().parents[1]
version=os.environ['VERSION'];platform=os.environ['PLATFORM'];out=root/'rust/out';out.mkdir(exist_ok=True)
patterns={'windows':{'*-setup.exe':'windows-x64-setup.exe'},'linux':{'*.AppImage':'linux-x86_64.AppImage','*.deb':'linux-amd64.deb','*.rpm':'linux-x86_64.rpm'}}[platform]
for pattern,suffix in patterns.items():
 files=list((root/'rust/target/packages').glob(pattern))
 if len(files)!=1:raise SystemExit(f'Expected one {pattern}, found {len(files)}')
 target=out/f'MultiDroid-Preview-{version}-{suffix}'
 shutil.copyfile(files[0],target)
 if target.stat().st_size<100_000:raise SystemExit('Installer is unexpectedly small')
 print(target.name)

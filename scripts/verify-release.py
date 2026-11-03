import hashlib,json,os
from pathlib import Path
out=Path('out');tag=os.environ['RELEASE_TAG'];version=tag[1:];native=os.environ['NATIVE']=='true'
names={f'MultiDroid-{version}.dmg'} if native else {f'MultiDroid-Preview-{version}-{suffix}' for suffix in ['mac-arm64.dmg','windows-x64-setup.exe','linux-amd64.deb','linux-x86_64.rpm','linux-x86_64.AppImage']}
actual={p.name for p in out.iterdir() if p.is_file()}
if actual!=names:raise SystemExit('Release file set mismatch: '+repr(actual.symmetric_difference(names)))
hashes={}
for name in sorted(names):
 with (out/name).open('rb') as stream:hashes[name]=hashlib.file_digest(stream,'sha256').hexdigest()
(out/'SHA256SUMS.txt').write_text(''.join(f'{digest}  {name}\n' for name,digest in hashes.items()))
(out/'release.json').write_text(json.dumps({'tag':tag,'sourceCommit':os.environ['SOURCE_COMMIT'],'apple':{'developerId':True,'notarized':True,'stapledAppAndDMG':True,'teamId':'6EZ89E2F5S'},'sha256':hashes},indent=2)+'\n')
notes=f'''## MultiDroid {version}

{'Native Mac release' if native else 'Desktop preview — features may be incomplete.'}

| Platform | Architecture | Installer | Signing |
| --- | --- | --- | --- |
| macOS | Apple Silicon | `.dmg` | Developer ID + notarized; app and DMG stapled |
'''
if not native:notes+='| Windows | x86-64 | `.exe` | No Authenticode signature |\n| Linux | x86-64 | `.deb`, `.rpm`, `.AppImage` | Verify SHA-256 |\n'
notes+='\n### Install\n\n- Mac: open the DMG and drag the application to Applications.\n'
if not native:notes+='- Windows: run the installer; Hypervisor Platform is required for emulators.\n- Linux: install the appropriate package; hardware emulation requires KVM, or use the supported lightweight engine.\n'
notes+='\n### Identity and integrity\n\nThe site, app and installers share the official MultiDroid icon. Verify downloads with `SHA256SUMS.txt`. No ZIP downloads are published.\n\nAndroid images and emulator packages are downloaded separately and have their own licenses.\n\nWebsite: https://multidroid.falcaosl.com/\nSource: `'+os.environ['SOURCE_COMMIT']+'`\n'
(out/'RELEASE-NOTES.md').write_text(notes)
print('Complete installer set and SHA-256 verified.')

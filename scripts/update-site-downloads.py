#!/usr/bin/env python3
"""Refresh static downloads only from assets actually published on GitHub.

Used by sync-site-on-release.yml after every published release (stable or
preview). Reads the public Releases API — no draft assets — and rewrites
site/index.html + site/releases.json so multidroid.falcaosl.com needs no
client-side GitHub calls.
"""
import json,os,re,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
url='https://api.github.com/repos/Falcao-SL/MultiDroid/releases?per_page=50'
headers={'Accept':'application/vnd.github+json','User-Agent':'MultiDroid-site-publisher'}
if os.environ.get('GH_TOKEN'):headers['Authorization']='Bearer '+os.environ['GH_TOKEN']
with urllib.request.urlopen(urllib.request.Request(url,headers=headers),timeout=60) as response:
 releases=json.load(response)
releases=[release for release in releases if not release['draft']]
stable=next((release for release in releases if not release['prerelease'] and any(re.fullmatch(r'MultiDroid-[\d.]+\.dmg',asset['name']) for asset in release['assets'])),None)
preview=next((release for release in releases if release['prerelease'] and any(asset['name'].endswith('-windows-x64-setup.exe') for asset in release['assets'])),None)
if not stable and not preview:
 raise SystemExit('No published MultiDroid release with recognized installers was found')
page=ROOT/'site/index.html';html=page.read_text()
manifest={}
for channel,release in [('stable',stable),('preview',preview)]:
 if not release:continue
 if not re.fullmatch(r'v\d+\.\d+(?:\.\d+)?(?:-[A-Za-z0-9.]+)?',release['tag_name']):raise SystemExit('Unexpected release tag')
 assets=[{'name':asset['name'],'url':asset['browser_download_url'],'size':asset['size']} for asset in release['assets'] if asset['state']=='uploaded']
 for asset in assets:
  if not asset['url'].startswith('https://github.com/Falcao-SL/MultiDroid/releases/download/') or not re.fullmatch(r'[A-Za-z0-9._+-]+',asset['name']):raise SystemExit('Unexpected asset URL or filename')
 manifest[channel]={'tag':release['tag_name'],'url':release['html_url'],'assets':assets}
 for key,suffix in ([('mac-stable','.dmg')] if channel=='stable' else [('windows-preview','-windows-x64-setup.exe'),('linux-preview','-linux-amd64.deb')]):
  asset=next((a for a in assets if a['name'].endswith(suffix)),None)
  if asset:
   html=re.sub(r'(data-download="'+key+r'" href=")[^"]+"',lambda m:m[1]+asset['url']+'"',html)
 if channel=='stable':
  html=re.sub(r'<p data-version="mac-stable">.*?</p>',lambda m:'<p data-version="mac-stable">'+release['tag_name'].removeprefix('v')+' · Apple Silicon<br>macOS 26 or later</p>',html)
if preview:
 html=re.sub(r'https://github.com/Falcao-SL/MultiDroid/releases/tag/v0\.3\.0-[A-Za-z0-9.]+',preview['html_url'],html)
 html=re.sub(r'(<h3>(?:Windows|Linux)</h3><p>)[^<]+(<br>)',lambda m:m[1]+preview['tag_name'].removeprefix('v')+' · x86-64'+m[2],html)
# The preview section has one direct link for each actual package, including Mac.
links=[]
if preview:
 for asset in manifest['preview']['assets']:
  if asset['name'].endswith(('.dmg','.exe','.deb','.rpm','.AppImage')):
   platform='macOS' if asset['name'].endswith('.dmg') else 'Windows' if asset['name'].endswith('.exe') else 'Linux'
   extension=next(ext for ext in ('.AppImage','.dmg','.exe','.deb','.rpm') if asset['name'].endswith(ext))
   links.append('<a href="'+asset['url']+'">'+platform+' '+extension+' ↗</a>')
 html=re.sub(r'<!-- preview-packages:start -->.*?<!-- preview-packages:end -->',lambda m:'<!-- preview-packages:start --><div class="all-packages"><h3>All preview packages · '+preview['tag_name']+'</h3><p>Desktop preview. Features may differ from the native Mac release.</p><div>'+''.join(links)+'</div></div><!-- preview-packages:end -->',html,flags=re.S)
page.write_text(html)
(ROOT/'site/releases.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('Static download links updated from published assets; no client-side GitHub dependency.')

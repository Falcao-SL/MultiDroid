#!/usr/bin/env python3
"""Generate site, application and installer assets from the official MultiDroid SVG/tokens."""
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import cairosvg
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT/'docs/design/icone/multidroid-icon.svg'
TOKENS = ROOT/'docs/design/tokens.json'
source = SOURCE.read_bytes()
tokens = json.loads(TOKENS.read_text())
image = Image.open(io.BytesIO(cairosvg.svg2png(bytestring=source,output_width=1024,output_height=1024))).convert('RGBA')
subprocess.run([sys.executable, str(ROOT/'docs/design/icone/export_icons.py')], check=True)
outputs = [str(path.relative_to(ROOT)) for path in (ROOT/'docs/design/icone/AppIcon.icon').rglob('*') if path.is_file()]
outputs += [str(path.relative_to(ROOT)) for folder in ('png','android') for path in (ROOT/'docs/design/icone'/folder).rglob('*') if path.is_file()]
def save(img, relative, **kwargs):
    path = ROOT/relative; path.parent.mkdir(parents=True,exist_ok=True)
    img.save(path, **kwargs); outputs.append(relative)
for size in (16,24,32,48,64,128,256,512,1024):
    save(image.resize((size,size),Image.Resampling.LANCZOS),f'rust/app/icons/{size}x{size}.png',optimize=True)
save(image, 'rust/app/icons/icon.ico', sizes=[(s,s) for s in (16,24,32,48,64,128,256)])
save(image, 'rust/app/icons/icon.icns')
for relative in ('site/assets/multidroid-mark.svg','rust/app/icons/brand.svg'):
    (ROOT/relative).write_bytes(source); outputs.append(relative)
save(image.resize((192,192),Image.Resampling.LANCZOS),'site/assets/icon-1.png',optimize=True)
accent=tokens['cor']['acento']['azul']; paper=tokens['cor']['claro']['fundo']; ink=tokens['cor']['claro']['tinta']
# Brand-only graphics: platform wizards retain familiar native interaction patterns.
def canvas(w,h): return Image.new('RGB',(w,h),paper)
header=canvas(150,57); header.paste(image.resize((42,42)),(101,7),image.resize((42,42))); ImageDraw.Draw(header).rectangle((0,49,88,56),fill=accent); save(header,'branding/nsis-header.bmp')
sidebar=canvas(164,314); side=ImageDraw.Draw(sidebar); side.rectangle((0,0,164,10),fill=accent); mark=image.resize((100,100));sidebar.paste(mark,(32,40),mark);side.text((18,165),'MultiDroid',font=ImageFont.load_default(size=24),fill=ink);side.text((18,207),'ONE DESK.\nMANY ANDROIDS.',font=ImageFont.load_default(size=12),fill=ink,spacing=7);save(sidebar,'branding/nsis-sidebar.bmp')
dmg=canvas(660,440); draw=ImageDraw.Draw(dmg);draw.rectangle((0,0,660,12),fill=accent);draw.text((36,37),'MultiDroid',font=ImageFont.load_default(size=33),fill=ink);draw.text((36,87),'Your Android workspace.',font=ImageFont.load_default(size=16),fill=ink);draw.line((285,235,375,235),fill=ink,width=3);draw.line((360,224,375,235,360,246),fill=ink,width=3);draw.text((118,367),'Drag the app to Applications',font=ImageFont.load_default(size=23),fill=ink);save(dmg,'branding/dmg-background.png',optimize=True)
og=canvas(1200,630);ogdraw=ImageDraw.Draw(og);ogdraw.rectangle((0,0,1200,12),fill=accent);mark=image.resize((156,156));og.paste(mark,(70,68),mark);ogdraw.text((70,265),'MultiDroid',font=ImageFont.load_default(size=76),fill=ink);ogdraw.text((70,373),'One workspace. Many Androids.',font=ImageFont.load_default(size=29),fill=ink);ogdraw.text((70,532),'macOS  /  Windows  /  Linux',font=ImageFont.load_default(size=20),fill=ink);preview=Image.open(ROOT/'site/assets/desk-5.webp').convert('RGB');preview.thumbnail((600,360),Image.Resampling.LANCZOS);og.paste(preview,(590,140));save(og,'site/assets/og-workspace.png',optimize=True)
css='/* Generated from docs/design/tokens.json. */\n:root{--brand-paper:'+paper+';--brand-ink:'+ink+';--brand-accent:'+accent+';--brand-radius:'+str(tokens['forma']['raio'])+'px;}\n'
(ROOT/'site/assets/brand-tokens.css').write_text(css);outputs.append('site/assets/brand-tokens.css')
manifest={'source':'docs/design/icone/multidroid-icon.svg','sourceSHA256':hashlib.sha256(source).hexdigest(),'tokensSHA256':hashlib.sha256(TOKENS.read_bytes()).hexdigest(),'outputs':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in outputs}}
(ROOT/'branding/manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(f'Synchronized {len(outputs)} assets from the official icon and design tokens.')

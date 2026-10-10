#!/usr/bin/env python3
"""Gera todos os ícones do MultiDroid a partir de multidroid-icon.svg (fonte única).

Camadas do SVG: bg (fundo), back (aparelho de trás), shadow (sombra dura),
front (aparelho da frente). Saídas, todas em docs/design/icone/:

  AppIcon.icon/                 Icon Composer: iPhone, iPad, Mac e Apple Watch
                                (fundo no icon.json com variante escura; uma
                                camada SVG por aparelho, e a sombra entre eles)
  png/ios-{default,dark,tinted}.png   fallback 1024 (tinted só em cinzas)
  png/app-store-1024.png        PNG achatado para fichas e sites
  png/visionos/{0-back,1-middle,2-front}.png   pilha de imagens do Vision Pro
  android/res/…                 ícone adaptativo (foreground, background,
                                monochrome), PNGs legados e redondos; vale para
                                Android e Wear OS
  android/play-store-512.png    ícone da Play Store

Uso (Pillow + cairosvg; cairo do Homebrew no macOS):
  DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib python3 export_icons.py
"""
import io
import json
import os
import re
import shutil

import cairosvg
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = open(os.path.join(HERE, "multidroid-icon.svg")).read()
HEAD = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">'
BG_DARK = "#0B0C10"  # fundo do modo escuro (o claro está no SVG)


def group(name):
    return re.search(rf'<g id="{name}">.*?</g>', SRC, re.S).group(0)


def fill_of(name):
    return re.search(rf'<g id="{name}">.*?fill="(#[0-9A-Fa-f]{{6}})"', SRC, re.S).group(1)


def svg(*names, bg=None):
    body = "".join(group(n) for n in names)
    if bg:
        body = f'<rect width="1024" height="1024" fill="{bg}"/>' + body
    return f"{HEAD}{body}</svg>\n"


def render(text, px=1024):
    png = cairosvg.svg2png(bytestring=text.encode(), output_width=px, output_height=px)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def p3(hex_color):
    """sRGB hex → string display-p3 do icon.json."""
    def lin(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    def enc(c):
        c = min(max(c, 0.0), 1.0)
        return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055

    r, g, b = (lin(int(hex_color[i:i + 2], 16) / 255) for i in (1, 3, 5))
    m = [(0.8225, 0.1774, 0.0), (0.0332, 0.9669, 0.0), (0.0171, 0.0724, 0.9108)]
    return "display-p3:" + ",".join(f"{enc(a * r + b_ * g + c * b):.5f}" for a, b_, c in m) + ",1.00000"


def out(*parts):
    path = os.path.join(HERE, *parts)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    return path


BG = fill_of("bg")

# ── Icon Composer ──────────────────────────────────────────────────────────
icon = os.path.join(HERE, "AppIcon.icon")
shutil.rmtree(icon, ignore_errors=True)
for name in ("front", "shadow", "back"):
    open(out("AppIcon.icon", "Assets", f"{name}.svg"), "w").write(svg(name))
layer = lambda n: {"name": n, "image-name": f"{n}.svg", "glass": True}  # noqa: E731
icon_json = {
    "fill-specializations": [
        {"value": {"solid": p3(BG)}},
        {"appearance": "dark", "value": {"solid": p3(BG_DARK)}},
    ],
    # Primeiro grupo = o de cima. Três profundidades: frente, sombra, trás.
    "groups": [
        {"name": "Front", "layers": [layer("front")],
         "shadow": {"kind": "neutral", "opacity": 0.5}, "translucency": {"enabled": False, "value": 0}},
        {"name": "Shadow", "layers": [{**layer("shadow"), "glass": False}],
         "shadow": {"kind": "none", "opacity": 0}, "translucency": {"enabled": False, "value": 0}},
        {"name": "Back", "layers": [layer("back")],
         "shadow": {"kind": "neutral", "opacity": 0.4}, "translucency": {"enabled": False, "value": 0}},
    ],
    "supported-platforms": {"circles": ["watchOS"], "squares": "shared"},
}
open(out("AppIcon.icon", "icon.json"), "w").write(json.dumps(icon_json, indent=2) + "\n")

# ── PNGs Apple ─────────────────────────────────────────────────────────────
light = render(svg("back", "shadow", "front", bg=BG))
dark = render(svg("back", "shadow", "front", bg=BG_DARK))
glyph = render(svg("back", "shadow", "front"))
tinted = Image.new("RGBA", glyph.size, (0, 0, 0, 255))
tinted.alpha_composite(glyph.convert("LA").convert("RGBA"))
light.convert("RGB").save(out("png", "ios-default.png"))
dark.convert("RGB").save(out("png", "ios-dark.png"))
tinted.convert("RGB").save(out("png", "ios-tinted.png"))
light.convert("RGB").save(out("png", "app-store-1024.png"))
# Vision Pro: pilha de 3 camadas 1024 (a de trás opaca, as outras com alfa).
render(svg(bg=BG)).convert("RGB").save(out("png", "visionos", "0-back.png"))
render(svg("back", "shadow")).save(out("png", "visionos", "1-middle.png"))
render(svg("front")).save(out("png", "visionos", "2-front.png"))

# ── Android e Wear OS (adaptive icon) ──────────────────────────────────────
# O glifo cabe no círculo garantido de 66 dp do canvas de 108 dp: a diagonal
# da caixa do glifo (no SVG de 1024) é escalada para 64 dp, centrada.
paths = re.findall(r'<path d="([^"]+)" fill="(#[0-9A-Fa-f]{6})"( fill-rule="evenodd")?', SRC)
_bbox = render(svg("back", "shadow", "front")).getbbox()  # caixa real do glifo, em unidades do SVG
x0, y0, x1, y1 = _bbox
diag = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
scale = 64 / diag
tx, ty = 54 - (x0 + x1) / 2 * scale, 54 - (y0 + y1) / 2 * scale
SHADOW_D = set(re.findall(r'd="([^"]+)"', group("shadow")))
BACK_FILL_D = re.findall(r'd="([^"]+)"', group("back"))[0]


def vector(color_override=None):
    items = []
    for d, color, evenodd in paths:
        if color_override and (d in SHADOW_D or d == BACK_FILL_D):
            continue  # monocromático: sem sombra, e o aparelho de trás só como contorno
        fill = color_override or color
        rule = ' android:fillType="evenOdd"' if evenodd else ""
        items.append(f'    <path android:fillColor="{fill}"{rule} android:pathData="{d}"/>')
    return ('<vector xmlns:android="http://schemas.android.com/apk/res/android"\n'
            '    android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108">\n'
            f'  <group android:translateX="{tx:.3f}" android:translateY="{ty:.3f}" '
            f'android:scaleX="{scale:.5f}" android:scaleY="{scale:.5f}">\n' + "\n".join(items) +
            "\n  </group>\n</vector>\n")


open(out("android", "res", "drawable", "ic_launcher_foreground.xml"), "w").write(vector())
open(out("android", "res", "drawable", "ic_launcher_monochrome.xml"), "w").write(vector("#FFFFFFFF"))
open(out("android", "res", "values", "ic_launcher_background.xml"), "w").write(
    f'<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <color name="ic_launcher_background">{BG}</color>\n</resources>\n')
adaptive = ('<?xml version="1.0" encoding="utf-8"?>\n'
            '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
            '    <background android:drawable="@color/ic_launcher_background"/>\n'
            '    <foreground android:drawable="@drawable/ic_launcher_foreground"/>\n'
            '    <monochrome android:drawable="@drawable/ic_launcher_monochrome"/>\n'
            '</adaptive-icon>\n')
for n in ("ic_launcher", "ic_launcher_round"):
    open(out("android", "res", "mipmap-anydpi-v26", f"{n}.xml"), "w").write(adaptive)

# PNGs legados (pré-API 26) e da Play Store mostram só a área visível do
# adaptativo (os 72 dp centrais), então o glifo é escalado para ela: a mesma
# proporção que o ícone tem no iPhone.
canvas = 1024
g = render(svg("back", "shadow", "front"))
bbox = g.getbbox()
gw, gh = bbox[2] - bbox[0], bbox[3] - bbox[1]
k = (64 / 72 * canvas) / (gw ** 2 + gh ** 2) ** 0.5
crop = g.crop(bbox).resize((round(gw * k), round(gh * k)), Image.LANCZOS)
square = Image.new("RGBA", (canvas, canvas), BG)
square.alpha_composite(crop, ((canvas - crop.width) // 2, (canvas - crop.height) // 2))
square.resize((512, 512), Image.LANCZOS).convert("RGB").save(out("android", "play-store-512.png"))
for dens, px in (("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)):
    sq = square.resize((px, px), Image.LANCZOS)
    mask = Image.new("L", (px * 4, px * 4), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, px * 4 - 1, px * 4 - 1], px * 4 // 6, fill=255)
    rounded = sq.copy(); rounded.putalpha(mask.resize((px, px), Image.LANCZOS))
    rounded.save(out("android", "res", f"mipmap-{dens}", "ic_launcher.png"))
    circle = Image.new("L", (px * 4, px * 4), 0)
    ImageDraw.Draw(circle).ellipse([0, 0, px * 4 - 1, px * 4 - 1], fill=255)
    rnd = sq.copy(); rnd.putalpha(circle.resize((px, px), Image.LANCZOS))
    rnd.save(out("android", "res", f"mipmap-{dens}", "ic_launcher_round.png"))

print("ícones do MultiDroid gerados a partir de multidroid-icon.svg")

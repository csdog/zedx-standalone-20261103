"""Export the confirmed prototype glyphs for native SVG image rendering."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parent.parent
source = (ROOT / 'prototype/index.html').read_text()
destination = ROOT / 'assets/icons/zedx'
destination.mkdir(parents=True, exist_ok=True)
manifest = {}
for attributes, identity, body in re.findall(
    r'<symbol\b([^>]*?id="([^"]+)"[^>]*)>(.*?)</symbol>', source, re.S
):
    name = identity.removeprefix('glyph-')
    appearance = 'light' if name.endswith('-light') else 'dark'
    name = name.removesuffix('-light')
    attributes = re.sub(r'\s*id="[^"]+"', '', attributes)
    view_box = re.search(r'viewBox="([^"]+)"', attributes).group(1).split()
    # GPUI's multicolor image loader rasterizes SVGs at scale 1, so provide retina-sized images.
    width, height = (float(value) * 4 for value in view_box[2:])
    color = '#6c707e' if appearance == 'light' else '#c9cdd4'
    if name in ['terminal', 'spark', 'code']:
        attributes += (f' fill="none" stroke="{color}" stroke-width="1.6"'
                       ' stroke-linecap="round" stroke-linejoin="round"')
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg"{attributes}'
           f' width="{width:g}" height="{height:g}" color="{color}">{body}</svg>\n')
    asset = f'icons/zedx/{name}-{appearance}.svg'
    (ROOT / 'assets' / asset).write_text(svg.replace('currentColor', color))
    manifest.setdefault(name, {})[appearance] = asset
for name, variants in manifest.items():
    if 'light' not in variants:
        dark = (ROOT / 'assets' / variants['dark']).read_text()
        asset = f'icons/zedx/{name}-light.svg'
        (ROOT / 'assets' / asset).write_text(dark.replace('#c9cdd4', '#6c707e'))
        variants['light'] = asset
(ROOT / 'zedx/icon-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f'Exported {len(manifest)} light/dark native glyph pairs.')

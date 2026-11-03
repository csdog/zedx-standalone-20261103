import json
from pathlib import Path
import re
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]


class NativeVisualAssets(unittest.TestCase):
    def test_gutter_run_icon_is_large_outlined_local_svg(self):
        asset = ROOT / 'assets/icons/zedx_run.svg'
        root = ET.fromstring(asset.read_text())
        self.assertEqual(root.attrib['viewBox'], '0 0 16 16')
        paths = root.findall('{http://www.w3.org/2000/svg}path')
        self.assertEqual(len(paths), 1)
        self.assertEqual(paths[0].attrib['fill'], 'none')
        self.assertEqual(paths[0].attrib['stroke'], 'black')
        self.assertEqual(paths[0].attrib['stroke-width'], '1.4')
        self.assertNotRegex(asset.read_text(), r'<(?:script|image|foreignObject)\b')

    def test_every_native_icon_has_valid_light_and_dark_assets(self):
        manifest = json.loads((ROOT / 'zedx/icon-manifest.json').read_text())
        self.assertGreaterEqual(len(manifest), 39)
        for name, pair in manifest.items():
            with self.subTest(icon=name):
                self.assertEqual(set(pair), {'light', 'dark'})
                dimensions = []
                for appearance, asset in pair.items():
                    source = (ROOT / 'assets' / asset).read_text()
                    svg = ET.fromstring(source)
                    self.assertEqual(svg.tag, '{http://www.w3.org/2000/svg}svg')
                    self.assertNotIn('currentColor', source)
                    self.assertNotRegex(source, r'<(?:script|image|foreignObject)\b')
                    self.assertNotRegex(source, r'(?:href|url)\s*[=(]\s*["\']?https?:')
                    self.assertTrue(asset.endswith(f'-{appearance}.svg'))
                    view_box = svg.attrib['viewBox'].split()
                    self.assertEqual(float(svg.attrib['width']), float(view_box[2]) * 4)
                    self.assertEqual(float(svg.attrib['height']), float(view_box[3]) * 4)
                    dimensions.append(svg.attrib['viewBox'])
                self.assertEqual(dimensions[0], dimensions[1])

    def test_rust_icon_enum_resolves_to_the_packaged_manifest(self):
        source = (ROOT / 'crates/ui/src/zedx.rs').read_text()
        variants = re.search(r'pub enum Icon\s*\{(.*?)\}', source, re.S).group(1)
        manifest = json.loads((ROOT / 'zedx/icon-manifest.json').read_text())
        for variant in re.findall(r'\b([A-Z][A-Za-z]+)\s*,', variants):
            name = re.sub(r'(?<!^)(?=[A-Z])', '-', variant).lower()
            self.assertIn(name, manifest)

    def test_light_workbench_uses_idea_light_header_palette(self):
        family = json.loads((ROOT / 'assets/themes/zedx/zedx.json').read_text())
        style = next(theme['style'] for theme in family['themes'] if theme['name'] == 'ZedX Light')
        for key in ['background', 'title_bar.background', 'status_bar.background']:
            self.assertEqual(style[key], '#f7f8fa6b')
        self.assertEqual(style['surface.background'], '#ffffff')
        self.assertEqual(style['element.selected'], '#d4e2ff')
        self.assertEqual(style['text.muted'], '#6c707e')

    def test_menu_outline_assets_are_packaged_and_have_no_solid_fill(self):
        assets = list((ROOT / 'assets/icons/zedx_menu').glob('*.svg'))
        self.assertGreaterEqual(len(assets), 20)
        for asset in assets:
            svg = ET.fromstring(asset.read_text())
            group = svg.find('{http://www.w3.org/2000/svg}g')
            self.assertIsNotNone(group)
            self.assertEqual(group.attrib['fill'], 'none')
            self.assertEqual(group.attrib['stroke'], 'black')
            self.assertEqual(group.attrib['stroke-width'], '1' if asset.stem == 'zedx_run' else '1.3')

    def test_context_run_uses_the_toolbar_triangle_geometry(self):
        namespace = '{http://www.w3.org/2000/svg}'
        menu = ET.fromstring((ROOT / 'assets/icons/zedx_menu/zedx_run.svg').read_text())
        toolbar = ET.fromstring((ROOT / 'assets/icons/zedx/play-light.svg').read_text())
        self.assertEqual(menu.find('.//' + namespace + 'path').attrib['d'],
                         toolbar.find('.//' + namespace + 'path').attrib['d'])

    def test_sticky_panel_overlays_have_explicit_opaque_theme_colors(self):
        family = json.loads((ROOT / 'assets/themes/zedx/zedx.json').read_text())
        for theme in family['themes']:
            style = theme['style']
            self.assertEqual(style['panel.background'], '#00000000')
            self.assertRegex(style['panel.overlay_background'], r'^#[0-9a-fA-F]{6}(?:ff)?$')
            self.assertNotEqual(style['panel.overlay_background'][:7], '#000000')
            if theme['name'] == 'ZedX Light':
                self.assertEqual(style['panel.overlay_background'], '#ffffff')

    def test_material_is_translucent_and_editor_remains_opaque(self):
        family = json.loads((ROOT / 'assets/themes/zedx/zedx.json').read_text())
        self.assertEqual({theme['name'] for theme in family['themes']}, {'ZedX Light', 'ZedX Dark'})
        for theme in family['themes']:
            with self.subTest(theme=theme['name']):
                style = theme['style']
                self.assertEqual(style['window_background_appearance'], 'blurred')
                for key in ['background', 'title_bar.background', 'status_bar.background']:
                    color = style[key]
                    self.assertEqual(len(color), 9)
                    self.assertGreater(int(color[-2:], 16), 0)
                    self.assertLess(int(color[-2:], 16), 255)
                for key in ['editor.background', 'editor.gutter.background']:
                    self.assertRegex(style[key], r'^#[0-9a-fA-F]{6}(?:ff)?$')
                # The dock paints the material once; panel contents remain transparent.
                self.assertEqual(style['panel.background'], '#00000000')
                self.assertTrue(style['syntax'])

    def test_light_frame_and_panel_contents_expose_the_native_backdrop(self):
        family = json.loads((ROOT / 'assets/themes/zedx/zedx.json').read_text())
        style = next(theme['style'] for theme in family['themes'] if theme['name'] == 'ZedX Light')
        frame_keys = ['background', 'title_bar.background', 'title_bar.inactive_background',
                      'status_bar.background']
        self.assertEqual({style[key] for key in frame_keys}, {'#f7f8fa6b'})
        self.assertLessEqual(int(style['title_bar.background'][-2:], 16), 128)
        self.assertEqual(style['panel.background'], '#00000000')
        self.assertEqual(style['editor.background'], '#ffffff')
        self.assertEqual(style['window_background_appearance'], 'blurred')

    def test_panel_defaults_match_the_confirmed_prototype(self):
        source = (ROOT / 'assets/settings/default.json').read_text()
        source = re.sub(r'("(?:\\.|[^"\\])*")|//[^\n]*|/\*.*?\*/',
                        lambda match: match.group(1) or '', source, flags=re.S)
        config = json.loads(re.sub(r',(\s*[}\]])', r'\1', source))
        for section, key, expected in [
            ('project_panel', 'default_width', 265),
            ('outline_panel', 'default_width', 270),
            ('terminal', 'default_height', 202),
            ('agent', 'default_width', 270),
        ]:
            self.assertEqual(config[section][key], expected, section)
        for section, expected in [('project_panel', 'left'), ('git_panel', 'left'), ('agent', 'right')]:
            self.assertEqual(config[section]['dock'], expected)
        self.assertFalse(config['agent']['flexible'])
        self.assertEqual(config['agent']['threads_sidebar']['default_width'], 300)

    def test_fork_defaults_reference_registered_themes(self):
        for name in ['default.json', 'initial_user_settings.json']:
            source = (ROOT / 'assets/settings' / name).read_text()
            self.assertIn('"light": "ZedX Light"', source)
            self.assertIn('"dark": "ZedX Dark"', source)


if __name__ == '__main__':
    unittest.main()

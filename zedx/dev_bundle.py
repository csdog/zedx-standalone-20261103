"""Create a local bundle that continues to read assets from this checkout."""
from pathlib import Path
import os
import plistlib
import re
import shutil
import subprocess
import sys

if sys.platform != 'darwin':
    raise SystemExit('The app bundle requires macOS. Use script/zedx run on other platforms.')
root = Path(__file__).resolve().parent.parent
profile = sys.argv[1] if len(sys.argv) > 1 else 'debug'
binary = root / 'target' / profile / 'zedx'
if not binary.is_file():
    raise SystemExit(f'Build the binary with script/zedx build first: {binary}')
version = re.search(
    r'^version = "([^"]+)"', (root / 'crates/zed/Cargo.toml').read_text(), re.MULTILINE
).group(1)
contents = root / 'target' / profile / 'ZedX.app/Contents'
macos = contents / 'MacOS'
resources = contents / 'Resources'
macos.mkdir(parents=True, exist_ok=True)
resources.mkdir(parents=True, exist_ok=True)
executable = macos / 'zedx'
if executable.exists():
    executable.unlink()
os.link(binary, executable)

# iconutil requires every slot of the iconset, so resample the 1024px source per size.
iconset = root / 'target' / profile / 'AppIcon.iconset'
shutil.rmtree(iconset, ignore_errors=True)
iconset.mkdir(parents=True)
source = root / 'crates/zed/resources/app-icon@2x.png'
for name, pixels in {
    'icon_16x16.png': 16,
    'icon_16x16@2x.png': 32,
    'icon_32x32.png': 32,
    'icon_32x32@2x.png': 64,
    'icon_128x128.png': 128,
    'icon_128x128@2x.png': 256,
    'icon_256x256.png': 256,
    'icon_256x256@2x.png': 512,
    'icon_512x512.png': 512,
    'icon_512x512@2x.png': 1024,
}.items():
    subprocess.run(
        ['sips', '-z', str(pixels), str(pixels), str(source), '--out', str(iconset / name)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
subprocess.run(
    ['iconutil', '-c', 'icns', str(iconset), '-o', str(resources / 'AppIcon.icns')],
    check=True,
    stdout=subprocess.DEVNULL,
)
shutil.rmtree(iconset)

with (contents / 'Info.plist').open('wb') as output:
    plistlib.dump({
        'CFBundleExecutable': 'zedx',
        'CFBundleIdentifier': 'dev.zedx.ZedX',
        'CFBundleName': 'ZedX',
        'CFBundleDisplayName': 'ZedX',
        'CFBundleIconFile': 'AppIcon',
        'CFBundlePackageType': 'APPL',
        'CFBundleShortVersionString': version,
        'CFBundleVersion': version,
        'NSHighResolutionCapable': True,
        'NSPrincipalClass': 'NSApplication',
    }, output)
print(contents.parent)

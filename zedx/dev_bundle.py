"""Create a local development bundle that continues to read assets from this checkout."""
from pathlib import Path
import os
import plistlib
import sys

if sys.platform != 'darwin':
    raise SystemExit('The development app bundle requires macOS. Use script/zedx run on other platforms.')
root = Path(__file__).resolve().parent.parent
binary = root / 'target/debug/zedx'
if not binary.is_file():
    raise SystemExit('Build the binary with script/zedx build first.')
contents = root / 'target/debug/ZedX.app/Contents'
macos = contents / 'MacOS'
macos.mkdir(parents=True, exist_ok=True)
executable = macos / 'zedx'
if executable.exists():
    executable.unlink()
os.link(binary, executable)
with (contents / 'Info.plist').open('wb') as output:
    plistlib.dump({
        'CFBundleExecutable': 'zedx',
        'CFBundleIdentifier': 'dev.zedx.ZedX',
        'CFBundleName': 'ZedX',
        'CFBundleDisplayName': 'ZedX',
        'CFBundlePackageType': 'APPL',
        'CFBundleShortVersionString': '1.26.2',
        'CFBundleVersion': '1.26.2',
        'NSHighResolutionCapable': True,
        'NSPrincipalClass': 'NSApplication',
    }, output)
print(contents.parent)

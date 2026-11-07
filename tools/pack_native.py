#!/usr/bin/env python3
"""Empaquette la partie native du mod (`native/`) en `target/vulture.hnmod` (docs/api/mod-files.md d'Horizon).

    python3 tools/pack_native.py

Le `.hnmod` tient `horizon-native.json` et `mixins/vulture/` (le bundle tel quel, sans `target/`). C'est ce que le
launcher installe dans `horizon-native-mods/` et construit : un bundle construit depuis son dossier n'est pas
« installé », et le bouton Jouer du launcher l'ignore.
"""
import os
import re
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BUNDLE = os.path.join(ROOT, "native")
OUT = os.path.join(ROOT, "target", "vulture.hnmod")
# horizon-mixins de la build épinglée (native-mods.json des builds du launcher).
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
KEEP = (".rs", ".toml")


def main():
    with open(os.path.join(BUNDLE, "horizon-mixin.toml")) as f:
        toml = f.read()
    version = re.search(r'^version = "([^"]+)"', toml, re.M).group(1)
    horizon = re.search(r'^horizon_revision = "([0-9a-f]{40})"', toml, re.M).group(1)
    manifest = ("{\n"
                '  "game": "26.3",\n'
                f'  "horizon": "{horizon}",\n'
                '  "id": "vulture",\n'
                f'  "mixins": "{MIXINS}",\n'
                '  "schema": 1,\n'
                f'  "version": "{version}"\n'
                "}\n")
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    files = []
    for dirpath, dirnames, filenames in os.walk(BUNDLE):
        dirnames[:] = sorted(d for d in dirnames if d != "target")
        for name in sorted(filenames):
            if name.endswith(KEEP):
                files.append(os.path.join(dirpath, name))
    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr(zipfile.ZipInfo("horizon-native.json", (2026, 1, 1, 0, 0, 0)), manifest)
        for path in files:
            rel = os.path.relpath(path, BUNDLE).replace(os.sep, "/")
            with open(path, "rb") as f:
                z.writestr(zipfile.ZipInfo(f"mixins/vulture/{rel}", (2026, 1, 1, 0, 0, 0)), f.read())
    print(f"{OUT}: {len(files)} fichiers, version {version}")


if __name__ == "__main__":
    main()

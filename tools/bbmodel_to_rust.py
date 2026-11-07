#!/usr/bin/env python3
"""Convertit le modèle Blockbench du vautour en données Rust (SPECS §7).

    python3 tools/bbmodel_to_rust.py

Lit `assets/models/vulture.bbmodel` (format « free », UV en boîte) et écrit
`native/api/src/model.rs` : l'arbre des groupes (pivots en texels, rotations en degrés) et leurs cubes,
avec les UV de chaque face dans l'ordre de `Host::draw_cube` (bas, haut, nord, sud, ouest, est),
en texels de 16 par côté comme un modèle de bloc. Le jeu dessine ces cubes, la texture vient de
`content/assets/vulture/textures/entity/vulture/`.
"""
import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "assets", "models", "vulture.bbmodel")
OUT = os.path.join(ROOT, "native", "api", "src", "model.rs")
FACES = ["down", "up", "north", "south", "west", "east"]


def num(x):
    return repr(float(x))


def vec(v):
    return "[" + ", ".join(num(x) for x in v) + "]"


def main():
    with open(SRC) as f:
        model = json.load(f)
    scale = 16.0 / model["resolution"]["width"], 16.0 / model["resolution"]["height"]
    elements = {e["uuid"]: e for e in model["elements"]}
    groups = {g["uuid"]: g for g in model["groups"]}

    def cube(e):
        if any(e.get(k) not in (None, [0, 0, 0]) for k in ("rotation",)):
            raise SystemExit(f"cube {e['name']} tourné : tournez son groupe à la place")
        uvs = []
        for face in FACES:
            u0, v0, u1, v1 = e["faces"][face]["uv"]
            uvs.append(vec([u0 * scale[0], v0 * scale[1], u1 * scale[0], v1 * scale[1]]))
        return (f"Cube {{ name: {json.dumps(e['name'])}, from: {vec(e['from'])}, to: {vec(e['to'])}, "
                f"uv: [{', '.join(uvs)}] }}")

    def part(node, depth):
        g = groups[node["uuid"]]
        cubes = [cube(elements[c]) for c in node["children"] if isinstance(c, str) and elements[c].get("export", True)]
        children = [part(c, depth + 1) for c in node["children"] if isinstance(c, dict)]
        pad = "    " * (depth + 1)
        cubes_rs = "".join(f"\n{pad}    {c}," for c in cubes)
        children_rs = "".join(f"\n{pad}    {c}," for c in children)
        return (f"Part {{\n{pad}name: {json.dumps(g['name'])},\n{pad}pivot: {vec(g['origin'])},\n"
                f"{pad}rotation: {vec(g.get('rotation', [0, 0, 0]))},\n"
                f"{pad}cubes: &[{cubes_rs}\n{pad}],\n{pad}children: &[{children_rs}\n{pad}],\n{pad[:-4]}}}")

    roots = [n for n in model["outliner"] if isinstance(n, dict)]
    if len(roots) != 1:
        raise SystemExit("le modèle doit avoir un seul groupe racine")
    with open(OUT, "w") as f:
        f.write("// Généré par tools/bbmodel_to_rust.py depuis assets/models/vulture.bbmodel : ne pas éditer.\n")
        f.write("use crate::{Cube, Part};\n\n")
        f.write(f"pub static VULTURE: Part = {part(roots[0], 0)};\n")
    print(f"{OUT}: {len(elements)} cubes, {len(groups)} groupes")


if __name__ == "__main__":
    main()

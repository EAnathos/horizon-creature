# Vulture

A mod of the [Horizon Creature](../../README.md) series: a vulture you spawn, ride and fly. Planned: taming with thrown
raw chicken, flight stamina, a harness, two chests, talons, breeding, stats and variants ([SPECS.md](SPECS.md)).

**Requires** Horizon Creature (`creature`), the series' library.

> **Status:** prototype. What works and what is left: [SPECS.md, §0](SPECS.md#0-progress-as-of-2026-11-08).
> Single player only for now.

## What works

- The tawny vulture, `vulture:vulture`: `/summon vulture:vulture` or the **spawn egg** (creative tab
  "Horizon Creature").
- **Mount**: right-click it (no cheats needed). **Get off**: hold sneak, or `/ride @s dismount`.
- **Fly**:

  | Key | Effect |
  |---|---|
  | W A S D | Direction following your look |
  | Space | Climb, flapping |
  | (nothing) | Glide, slowly losing height |
  | Left Ctrl | Dive |

- Its own model and texture, animated (wings folded or flapping, head, legs), and its own sounds (hisses, hurt, death,
  wing flaps).

## Files

| Path | Contents |
|---|---|
| `content/` | The content half (`vulture.hmod`): entity, egg, textures, sounds, translations |
| `native/` | The native half (`vulture.hnmod`): `api/` (crate `mod-vulture`) and two mixin injections (`mount_click.rs`, `mount_hud.rs`) |
| `assets/models/vulture.bbmodel` | The Blockbench model, source of `native/api/src/model.rs` (`cargo creature model vulture`) |
| `module.toml` | Its entry for the series' tools and its creative tab items |

Build and install with the library: `cargo creature install vulture` (see the [root README](../../README.md)).

## Sounds

Made from [CC0](https://creativecommons.org/publicdomain/zero/1.0/) sounds from [freesound.org](https://freesound.org)
(hissing geese, wings, a vulture imitation), edited by `cargo creature sounds vulture`; the list is in
`tools/src/sounds.rs`.

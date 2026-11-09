# Vulture

A mod of the [Horizon Creature](../../README.md) series: a vulture you spawn, ride and fly. Planned: taming with thrown
raw chicken, flight steered by your look (banking into turns like a plane), a harness, two chests, talons, breeding and
stats ([SPECS.md](SPECS.md)).

**Requires** Horizon Creature (`creature`), the series' library.

> **Status:** prototype. What works and what is left: [SPECS.md, §0](SPECS.md#0-progress-as-of-2026-11-09).
> Single player only for now.

## What works

- The tawny vulture, `vulture:vulture`: `/summon vulture:vulture` or the **spawn egg** (creative tab
  "Horizon Creature").
- **Mount**: right-click it (no cheats needed). **Get off**: hold sneak, or `/ride @s dismount`.
- **Fly**, like a plane: it always flies forward and goes where you look.

  | Key | Effect |
  |---|---|
  | Look up / down | Climb (losing speed) / dive (gaining speed) |
  | Look left / right | Bank and turn in an arc |
  | W | Flap, up to cruising speed |
  | (nothing) | Glide, slowly losing height |
  | S | Brake |
  | Space | Strong flap; takes off from the ground |

- Its own model and texture, animated (wings folded, gliding, flapping, swept back in a dive, banking; head, legs), and
  its own sounds (hisses, hurt, death, wing flaps, take-off).

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

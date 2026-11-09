# Vulture

A mod of the [Horizon Creature](../../README.md) series: a vulture you spawn, ride and fly. Planned: taming with thrown
raw chicken, flight steered by your look (banking into turns like a plane), a harness, two chests, talons, breeding and
stats ([SPECS.md](SPECS.md)).

**Requires** Horizon Creature (`creature`), the series' library.

> **Status:** prototype. What works and what is left: [SPECS.md, §0](SPECS.md#0-progress-as-of-2026-11-09).
> Single player only for now.

## What works

Tried in game: the vulture, its spawn egg, mounting and the ridden flight. Built but not tried in game yet: the rest.

- The tawny vulture, `vulture:vulture`: `/summon vulture:vulture`, the **spawn egg** (creative tab "Horizon Creature"),
  or in the wild in deserts, badlands and savannas, where it soars in circles and lands again.
- **Taming**: throw it raw chicken (Q). It walks to the meat and eats it: 1 chance in 4 to tame it (hearts).
- **Its owner's right-click**: a **vulture harness** (5 leather, 2 string, 1 iron ingot) puts it on, a **chest** adds
  one (two at most), raw meat heals it, rotten flesh makes it want a mate; otherwise mount it (with a harness) or tell
  it to sit or follow. **Sneak + right-click**: its screen (harness, sit / follow, chests and their 15 slots each).
- **Fly**, like a plane: it always flies forward and goes where you look.

  | Key | Effect |
  |---|---|
  | Look left / right | Bank and turn in an arc |
  | Space (held) | Climb; takes off from the ground |
  | Sneak (held) | Descend |
  | W | Flap, up to cruising speed |
  | (nothing) | Glide, slowly losing height |
  | S | Brake |
  | R | Talons: catch the small creature below, let it go |
  | Sneak on the ground | Get off |

- A tamed vulture follows its owner in the air, defends them, and two in love have a baby that grows up in 20 minutes.
  Each vulture has its own health and speed.
- Neutral: hit it and it dives at you, with the vultures nearby.
- Drops feathers and meat (raw and cooked vulture).
- Its own model and texture, animated, and its own sounds (hisses, hurt, death, wing flaps, take-off, eating).

## Files

| Path | Contents |
|---|---|
| `content/` | The content half (`vulture.hmod`): entity, egg, textures, sounds, translations |
| `native/` | The native half (`vulture.hnmod`): `api/` (crate `mod-vulture`) and three mixin injections (`interact.rs`, `mount_hud.rs`, `ride_keys.rs`) |
| `assets/models/vulture.bbmodel` | The Blockbench model, source of `native/api/src/model.rs` (`cargo creature model vulture`) |
| `assets/models/baby.bbmodel` | The baby's model, source of `native/api/src/model_baby.rs` (`cargo creature model vulture baby`) |
| `module.toml` | Its entry for the series' tools and its creative tab items |

Build and install with the library: `cargo creature install vulture` (see the [root README](../../README.md)).

## Sounds

Made from [CC0](https://creativecommons.org/publicdomain/zero/1.0/) sounds from [freesound.org](https://freesound.org)
(hissing geese, wings, a vulture imitation), edited by `cargo creature sounds vulture`; the list is in
`tools/src/sounds.rs`.

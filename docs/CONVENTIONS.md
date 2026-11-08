# Development conventions

Rules shared by every module of the Horizon Creature series ([ARCHITECTURE.md](ARCHITECTURE.md)).

## Language

- Everything in the repository is in **English**: code, comments, docs, specs, commit messages.
- The game text is translated: every module keeps `assets/<id>/lang/en_us.json` and `fr_fr.json` in step (same keys).

## Ids and namespaces

- A module's mod id is its folder name, `[a-z0-9_]`, plural for a family of creatures (`butterflies`, `wyverns`) and
  singular for one species (`vulture`). The library is `creature`.
- A module only declares content in **its own namespace** (Horizon refuses two mods owning one namespace). Shared things
  live in `creature:`, species things in the mod's namespace: `creature:incubator`, `wyverns:fire_wyvern_egg`.
- Names: `<species>` for an entity (`vulture:vulture`, `wyverns:fire_wyvern`), `<species>_spawn_egg`, `<species>_egg`
  for a real egg, `<thing>` for an item. Entity texture paths: `<id>:entity/<species>/<variant>`.
- Lang keys follow vanilla: `entity.<id>.<name>`, `item.<id>.<name>`, `block.<id>.<name>`,
  `subtitles.entity.<id>.<sound>`. Only the library has an `itemGroup` key.
- Sounds: `<id>:entity.<species>.<sound>` in the module's `sounds.json`.
- Ids are permanent once a test world uses them: renaming breaks saves.

## Specs

- One `SPECS.md` per module, the source of truth for it; series-wide rules live in `docs/`. A mod's spec does not
  describe a library system: it links to `common/SPECS.md` and gives only its parameters (numbers, items, species).
- Markers: ✅ decided, ❓ proposal to confirm. Numbers are starting values, balanced in game.
- §0 is the progress: what is done (and tested in game), then what is left, in order. Dates are absolute
  (`2026-11-08`).
- The last section lists the open questions, struck through once answered with the answer.

## Code

- Native code follows the vulture's style: module docs (`//!`) explain the mechanism and link the spec section, Horizon
  internals that are relied on are named. Rust 1.99.1 (pinned by the launcher), edition 2024.
- A native crate may only use crates in Horizon's `Cargo.lock`. The tools (`tools/`) are a separate crate with their own
  lock.
- Server logic stays apart from rendering (multiplayer comes later).
- Generated files say so in their first line and are never edited by hand: `model.rs` (from `.bbmodel`), the shared
  creative tab.
- Before adding a system to a mod, check the library; before writing one that a second mod will need, put it in the
  library (ARCHITECTURE.md §3).

## Assets

- Models: Blockbench, "free" format, box UV, one root group, no rotated cubes (rotate groups). 64×64 textures unless a
  larger mob needs more.
- Sounds: CC0 only (freesound.org, licence checked), sources listed in the tool that makes them.

## Versions and commits

- Semver per module, content and native versions kept equal (ARCHITECTURE.md §6). One Horizon pin for the series.
- A commit touches one module when it can; a library change and the mod changes that need it go together.

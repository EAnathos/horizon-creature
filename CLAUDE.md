# Horizon Creature

A series of native Rust mods for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3): one mandatory
library, **Horizon Creature** (`common/`, mod id `creature`), and independent creature mods in `mods/<mod>/` that all
depend on it: `vulture` (prototype: tameable, rideable flying vulture), `butterflies` (and moths), `wyverns`
(dimension, harpies, wyverns, Harpy Queen boss), `snakes`, `worms`, `cyclops`, `fireflies` (specs only).

- **Architecture**: `docs/ARCHITECTURE.md` (modules, dependencies, what goes in the library, the single creative tab,
  adding a mod). **Conventions**: `docs/CONVENTIONS.md`. **Horizon issues** we raised or follow, and our workarounds:
  `docs/HORIZON-ISSUES.md` (keep it up to date; mod-API PRs go to arismoko/horizon, issues stay on horizon-rust).
- **Specs**: one `SPECS.md` per module, source of truth for it (✅ decided, ❓ to confirm; §0 tracks progress). A mod's
  spec links to `common/SPECS.md` for shared systems instead of describing them.
- **Rule of two**: a system used (or specified) by two or more mods goes in the library; species content stays in mods.
  Never duplicate a system between mods. No mod requires another: optional content between mods goes through
  conditional recipes (the Wyvern Staff: butterfly dust or gunpowder, cyclops eye or eye of ender; `mod_loaded`, `not`).
  Library features tied to some mods (the glass jar: Butterflies, Worms or Fireflies) switch on through registration and
  conditional recipes (`neoforge:conditions`, `mod_loaded`).
- **Creative tab**: only the library has one, generated from each module's `module.toml` (`creative` list) by
  `cargo creature tab`. Mods declare no tab.
- **Language**: everything in the repository is in English (code, comments, docs, commit messages). The game text is
  translated: keep each module's `assets/<id>/lang/en_us.json` and `fr_fr.json` in step. Conversations with the user are
  in English.

## Horizon

- Horizon sources and docs installed by the launcher: `~/.local/share/horizon-launcher/native/stage/`
  - `docs/mods.md` (content mods, `entity_type`, creative tabs, menus, spawns), `docs/native-mods.md` (`native_*` hooks,
    goals, mob rendering, "Dependencies between trait mods")
  - `crates/cargo-horizon-mod` (the `cargo horizon-mod` tool), `crates/horizon-client/src/game/native_mob_draw.rs`, `mods/`
- Example installed native mod: `~/.local/share/horizon-launcher/native/installed.incoming/mod_menu/`
  (`horizon-native.json`, `mixins/<id>/horizon-mixin.toml`).
- Rust 1.99.1 toolchain pinned by the launcher. `horizon` MCP tools available (instances, mods, `game_*` to drive and
  capture the game).
- Never modify Horizon's files in `~/.local/share/horizon-launcher`: read them, write code in this repository.

## Building and testing

- `cargo creature install [--instance <name>] [mod...]` writes the creative tab, builds each module's `content/`
  (`.hmod`) and `native/` (`.hnmod`), always with the library, installs them in the instance (default `creature-dev`) and
  runs its native build. The launcher's Play button ignores a bundle built from a folder: it must be installed as a
  `.hnmod`.
- Tools are Rust (`tools/`, commands `list`, `tab`, `model`, `sounds`, `pack`, `build`, `check`, `install`; alias in
  `.cargo/config.toml`).
- Before committing: rustfmt (tools and native code, `rustfmt.toml`), Clippy `-D warnings` on the tools,
  `cargo creature check`; CI (`.github/workflows/ci.yml`) runs them and `cargo creature build`. CI can't compile the
  native crates: run `cargo creature install` after native changes.
- No em dash in docs or text.
- The launcher binary is `~/Téléchargements/horizon-launcher.AppImage` (override with `HORIZON_LAUNCHER`).

## Known API constraints

- Native dependency: a mod's `native/horizon-mixin.toml` has `[dependencies] creature = "^0.1"`; the launcher gives its
  `[api]` crate a `creature` crate and runs the library's `hooks()` first. Content mods have no dependency field.
- A content mod can't add items to another mod's creative tab, but a tab may list other mods' items (missing ones are
  left out): hence the library's generated tab.
- A mod's living entity type can only copy `minecraft:zombie`, `minecraft:villager` or `minecraft:wandering_trader`
  (the vulture uses the wandering trader, whose behaviour is all goals).
- Rendering: `native_mob_submit` + `native_draw_cube`, through `creature::model::draw` (cubes, pivots,
  `<mod>:entity/...` textures).
- Riding: `/ride` and `Host::mount` (the vulture's right-click) put a player on any mob; Horizon copies the rider's keys
  into `Mob.rider`, which the `Fly` goal reads. These are internal fields: the Horizon revision is pinned, the same for
  every module.
- Code injected into the right-click (`native_interact_entity`) runs on a client-side copy: what it changes on a mob is
  lost, and `start_riding` there seats the player on the client only (the mob's goals never see a rider, the client
  moves the mount itself). Handle clicks in the server trait hook (`interact_entity`), change a mob through its goals
  (the vulture's `state::request`), mount with `Host::mount`, and read the player from `acting_player()` (its
  `sneaking`, `entity_id`).
- Mob sounds are always prefixed with `minecraft:` by Horizon: our mobs are muted and play their own through
  `creature::sounds` (`Host::play_sound`).
- Custom dimensions (Wyverns) are not documented in Horizon: to prototype first.

## Horizon workarounds

Each one is marked in the code with its issue; drop it when moving to a Horizon revision that fixes it:
#13971 (`cargo creature install` reads the build log), #13972 (clicks in the trait hook, `Host::mount`), #13973
(harness not in `Mob.extra.saddled`), #13974 (the vulture's screen opens next tick), #13975 (hitbox height 0.6 for the
seat), #6276 (the vulture's size by a cloned `MobType`). Full tracker: `docs/HORIZON-ISSUES.md`.

## Models

- Blockbench 5.2.2 (Flatpak `net.blockbench.Blockbench`) + jasonjgardner MCP plugin v1.10.1, `blockbench` server in
  `.mcp.json` (Blockbench must be open). `.bbmodel` files in `mods/<mod>/assets/models/`; `cargo creature model <mod>`
  turns `<mod>.bbmodel` into the mod's `native/api/src/model.rs`.

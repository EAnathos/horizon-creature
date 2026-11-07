# horizon-vulture

A native Rust mod for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3): a tameable vulture (thrown
raw chicken), rideable in flight with stamina, 2 chests, talons, breeding, variable stats and skins.

- **Spec**: `docs/SPECS.md` (source of truth; ✅ decided, ❓ to confirm; §0 tracks progress).
- **Language**: everything in the repository is in English (code, comments, docs, commit messages). The game text is
  translated: keep `content/assets/vulture/lang/en_us.json` and `fr_fr.json` in step. Conversations with the user are in
  English.

## Horizon

- Horizon sources and docs installed by the launcher: `~/.local/share/horizon-launcher/native/stage/`
  - `docs/mods.md` (content mods, `entity_type`, menus, spawns), `docs/native-mods.md` (`native_*` hooks, goals, mob
    rendering)
  - `crates/cargo-horizon-mod` (the `cargo horizon-mod` tool), `crates/horizon-client/src/game/native_mob_draw.rs`, `mods/`
- Example installed native mod: `~/.local/share/horizon-launcher/native/installed.incoming/mod_menu/`
  (`horizon-native.json`, `mixins/<id>/horizon-mixin.toml`).
- Rust 1.99.1 toolchain pinned by the launcher. `horizon` MCP tools available (instances, mods, `game_*` to drive and
  capture the game).
- Never modify Horizon's files in `~/.local/share/horizon-launcher`: read them, write code in this repository.

## Building and testing

- `cargo vulture install [instance]` builds `content/` (`.hmod`) and `native/` (`.hnmod`), installs both in the instance
  (default `vulture-dev`) and runs its native build. The launcher's Play button ignores a bundle built from a folder:
  it must be installed as a `.hnmod`.
- Tools are Rust (`tools/`, commands `model`, `sounds`, `pack`, `install`; alias in `.cargo/config.toml`).
- The launcher binary is `~/Téléchargements/horizon-launcher.AppImage` (override with `HORIZON_LAUNCHER`).

## Known API constraints

- A mod's living entity type can only copy `minecraft:zombie`, `minecraft:villager` or `minecraft:wandering_trader`
  (we use the wandering trader, whose behaviour is all goals).
- Rendering: `native_mob_submit` + `native_draw_cube` (cubes, pivots, `vulture:entity/...` textures).
- Riding: `/ride` and our `mount_click.rs` injection put a player on any mob; Horizon copies the rider's keys into
  `Mob.rider`, which the `Fly` goal reads. These are internal fields: the Horizon revision is pinned.
- Mob sounds are always prefixed with `minecraft:` by Horizon: the vulture is muted and plays its own with
  `Host::play_sound`.

## Models

- Blockbench 5.2.2 (Flatpak `net.blockbench.Blockbench`) + jasonjgardner MCP plugin v1.10.1, `blockbench` server in
  `.mcp.json` (Blockbench must be open). `.bbmodel` files in `assets/models/`; `cargo vulture model` turns
  `vulture.bbmodel` into `native/api/src/model.rs`.

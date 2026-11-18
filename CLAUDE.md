# Sunrise Creatures

A series of native Rust mods for **Sunrise** (formerly Horizon, a Rust reimplementation of Minecraft Java 26.3): one mandatory
library, **Sunrise Creatures** (`common/`, mod id `creature`), and independent creature mods in `mods/<mod>/` that all
depend on it: `vulture` (tameable, rideable flying vulture), `butterflies` (and moths), `worms`, `fireflies`,
`cyclops`, `fairies`, and `vanilla_bestiary` (Minecraft's creatures in the guide, in development). The other folders of
`mods/` are planned mods (a spec, a model).

- **Players**: `README.md` and each module's `README.md`. **Developers**: `CONTRIBUTING.md`.
- **Architecture**: `docs/ARCHITECTURE.md` (modules, dependencies, what goes in the library, the single creative tab,
  adding a mod). **Conventions**: `docs/CONVENTIONS.md`. **Sunrise issues** we raised or follow, and our workarounds:
  `docs/SUNRISE-ISSUES.md` (keep it up to date; issues go to sunrise-org/sunrise).
- **Specs**: one `SPECS.md` per module, source of truth for it (✅ decided, ❓ to confirm; §0 is its current status, not a log). A mod's
  spec links to `common/SPECS.md` for shared systems instead of describing them.
- **Rule of two**: a system used (or specified) by two or more mods goes in the library; species content stays in mods.
  Never duplicate a system between mods. No mod requires another: optional content between mods goes through
  conditional recipes (`neoforge:conditions`, `mod_loaded`, `not`).
  Library features tied to some mods (the glass jar: Butterflies, Worms, Fireflies or Fairies) switch on through registration and
  conditional recipes (`neoforge:conditions`, `mod_loaded`).
- **Creative tab**: only the library has one, generated from each module's `module.toml` (`creative` list) by
  `cargo creature tab`. Mods declare no tab.
- **Language**: everything in the repository is in English (code, comments, docs, commit messages). The game text is
  translated: keep each module's `assets/<id>/lang/en_us.json` and `fr_fr.json` in step. Conversations with the user are
  in English.

## Sunrise

Horizon became Sunrise, and the series now builds for Sunrise only (the last Horizon build is commit `e542ff7`): every
module runs on Sunrise (`ported = true` in a module's `module.toml` turns its code on; a module without it is packaged as content
only). The plan, what each step taught and what is left: `docs/SUNRISE-PORT.md` (keep it up
to date).

- Launcher `~/Téléchargements/sunrise-launcher-linux-x86_64.AppImage` (also in the app menu), data
  `~/.local/share/sunrise` (never modify it), instance `creature-dev`. Agent control: start the game with
  `sunrise-launcher play --instance creature-dev --world <w> --arg --control`, then `sunrise-ctl` (in
  `~/.local/share/sunrise/launcher/versions/<version>/`): `command`, `screenshot`, `entities`, `interact-entity`...
- Sources: `../sunrise-src` (sparse checkout of sunrise-org/sunrise: `crates/sunrise-mod-vocab` holds the hook
  declarations with their docs, `docs/modding/src` the modding book). The pinned SDK: `target/sunrise-sdk/`
  (fetched by `cargo creature native-check`; `surface.json` lists the whole API).
- A module's root is its package: `sunrise-mod.toml` (id, version, `[native]` pins, `[dependencies] creature`),
  `Cargo.toml` (`sunrise-mod-api` by path from `.sunrise/sdk/`), `src/`, `assets/<id>/`, `data/<id>/sunrise/`; beside it
  the repository's own `module.toml`, `SPECS.md`, `sources/` (Blockbench models, `sounds.toml`, Horizon's old
  manifests).
- Never modify Horizon's files in `~/.local/share/horizon-launcher` either.

## Building and testing

- `cargo creature install [--instance <name>] [mod...]` writes the creative tab, packages each module (staged in
  `target/stage/`, zipped to `target/<id>.smod`, checked by `cargo sunrise-mod check`), always with the library: a
  ported module is built by `sunrise-launcher native-build target/stage/native` (which also selects the build for the
  instance), a content-only one goes in the instance's `sunrise-mods/`. The launcher's "Installed mods" list shows only what its mod
  browser installed: our native modules never appear there (they are built into the game the instance plays, its
  version a `native-<rev>-<hash>` build; `sunrise-launcher instance list`).
- Tools are Rust (`tools/`, commands `list`, `tab`, `model`, `preview`, `sounds`, `vanilla`, `pack`, `build`, `check`,
  `native-check`, `install`; alias in `.cargo/config.toml`; the table in `CONTRIBUTING.md`).
- Before committing: rustfmt (tools and `src/`, `rustfmt.toml`), Clippy `-D warnings` on the tools,
  `cargo creature check`, `cargo creature native-check` (Clippy and tests of the ported crates on the pinned SDK); CI
  (`.github/workflows/ci.yml`) runs them and `cargo creature build`.
- A native mod's stderr reaches no log: debug by writing to a file.
- No em dash in docs or text.
- The tools find the launcher by `$SUNRISE_LAUNCHER`, the `PATH` or an AppImage in the download folder, and its data by
  `$SUNRISE_LAUNCHER_DATA` or `~/.local/share/sunrise` (`tools/src/launcher.rs`).

## Known API constraints

- A mod's living entity type can only copy `minecraft:zombie`, `minecraft:villager` or `minecraft:wandering_trader`
  (ours copy the trader, whose behaviour is all goals; the library takes its goals off at join).
- `entity_tick_post` fires only because the library answers `entity_tick_listened`: our behaviours (`creature::body`)
  run there, on the server. A behaviour can't hurt, give effects or set targets: it queues, the tick hook applies.
- Rendering: a mod's `EntityRenderer`, posed at the feet, unturned (`creature::model::living_pose`, `draw`).
- Riding: the rider's keys come from the server `player_input` hook; the rider is the mount's first passenger
  (`controlling_passenger` is empty unless vanilla steers it); `entity_mount` can cancel a dismount.
- `EntityRef::leash_holder` answers `Some(LeashHolder::Unleashed)` for a mob without a lead.
- A payload's string holds at most 4096 bytes (`PAYLOAD_MAX_STRING_BYTES`).
- More in `docs/SUNRISE-PORT.md` (what each port step taught).

## Sunrise workarounds

Each one is marked in the code with its issue and listed, with its limits and when to drop it, in
`docs/SUNRISE-ISSUES.md` (keep it up to date).

## Models

- Blockbench 5.2.2 (Flatpak `net.blockbench.Blockbench`) + jasonjgardner MCP plugin v1.10.1, `blockbench` server in
  `.mcp.json` (Blockbench must be open). `.bbmodel` files in `mods/<mod>/sources/models/`; `cargo creature model <mod>`
  turns `<mod>.bbmodel` into the mod's `src/model.rs`; `cargo creature preview` redraws every model's
  pictures in `docs/models/`, shown by the READMEs and specs (run it after changing a model; `cargo creature check` fails when either is stale).
  The `.bbmodel` files and the PNG icons are the sources: no script generates them.
- Sounds: `cargo creature sounds <mod>` cuts a mod's `.ogg` files from freesound downloads as its
  `sources/sounds.toml` says.

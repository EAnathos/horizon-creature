# Sunrise Creatures

A series of native Rust mods for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3): one mandatory
library, **Sunrise Creatures** (`common/`, mod id `creature`), and independent creature mods in `mods/<mod>/` that all
depend on it: `vulture` (tameable, rideable flying vulture), `butterflies` (and moths), `worms`, `fireflies`,
`cyclops`, `fairies`, and `vanilla_bestiary` (Minecraft's creatures in the guide, in development). The other folders of
`mods/` are planned mods (a spec, a model).

- **Players**: `README.md` and each module's `README.md`. **Developers**: `CONTRIBUTING.md`.
- **Architecture**: `docs/ARCHITECTURE.md` (modules, dependencies, what goes in the library, the single creative tab,
  adding a mod). **Conventions**: `docs/CONVENTIONS.md`. **Horizon issues** we raised or follow, and our workarounds:
  `docs/HORIZON-ISSUES.md` (keep it up to date; mod-API PRs and new modding issues go to arismoko/horizon, game bugs to horizon-rust).
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

Horizon became Sunrise. The port is on the `sunrise` branch (main still builds for Horizon): every module runs on
Sunrise (`ported = true` in a module's `module.toml` turns its code on; a module without it is packaged as content
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
  instance), a content-only one goes in the instance's `sunrise-mods/`.
- Tools are Rust (`tools/`, commands `list`, `tab`, `model`, `preview`, `sounds`, `vanilla`, `pack`, `build`, `check`,
  `native-check`, `install`; alias in `.cargo/config.toml`; the table in `CONTRIBUTING.md`).
- Before committing: rustfmt (tools and `src/`, `rustfmt.toml`), Clippy `-D warnings` on the tools,
  `cargo creature check`, `cargo creature native-check` (Clippy and tests of the ported crates on the pinned SDK); CI
  (`.github/workflows/ci.yml`) runs them and `cargo creature build`.
- A native mod's stderr reaches no log: debug by writing to a file.
- No em dash in docs or text.
- The tools find the launcher by `$SUNRISE_LAUNCHER`, the `PATH` or an AppImage in the download folder, and its data by
  `$SUNRISE_LAUNCHER_DATA` or `~/.local/share/sunrise` (`tools/src/launcher.rs`).

## Known API constraints (Horizon: the unported mods' code; their Sunrise answers are in docs/SUNRISE-PORT.md)

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

## Horizon workarounds

Each one is marked in the code with its issue and listed, with its limits and when to drop it, in
`docs/HORIZON-ISSUES.md` (keep it up to date).

## Models

- Blockbench 5.2.2 (Flatpak `net.blockbench.Blockbench`) + jasonjgardner MCP plugin v1.10.1, `blockbench` server in
  `.mcp.json` (Blockbench must be open). `.bbmodel` files in `mods/<mod>/sources/models/`; `cargo creature model <mod>`
  turns `<mod>.bbmodel` into the mod's `src/model.rs`; `cargo creature preview` redraws every model's
  pictures for `docs/MODELS.md` (run it after changing a model; `cargo creature check` fails when either is stale).
  The `.bbmodel` files and the PNG icons are the sources: no script generates them.
- Sounds: `cargo creature sounds <mod>` cuts a mod's `.ogg` files from freesound downloads as its
  `sources/sounds.toml` says.

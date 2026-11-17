# Porting the series to Sunrise

Horizon became **Sunrise** ([sunrise-org/sunrise](https://github.com/sunrise-org/sunrise), launcher at
[playsunrise.net](https://playsunrise.net), modding book at <https://sunrise-org.github.io/sunrise/>). The series is
still built for Horizon (pinned `fdc87ae`); this page lists what Sunrise is, what has to change for our modules to run
on it, and in what order. It is the working plan of the port: update it as items are done or answered (✅ decided,
❓ to confirm).

Last checked: 2026-11-17, Sunrise launcher 1.0.1, Nightly `nightly-2026-11-17-2d81263`, source `559fd52` (main).

## 1. What Sunrise is, for us

- **The mod API is a redesign, not a rename** of `horizon-client`: one crate, `sunrise-mod-api` (lib `mod_api`),
  shared by native and WebAssembly mods. Hooks are methods of `mod_api::Hooks` on a `#[derive(Default)]` type marked
  `#[mod_api::sunrise_mod]`; there is no hand-written `hooks()` factory any more.
- **Hosts are split by side**: a server hook gets `&mut dyn ServerHost`, a client hook a `ClientHost`, a hook on both
  sides a `HostSide`. A call on the wrong side does not compile. Level hooks get narrower hosts (`LevelHost`,
  `LevelRandomHost`, `LevelWriteHost`, `HitHost`).
- **Parts**: behaviour attached to one block, item, block entity, menu, fluid or entity type
  (`Registrar::entity_part(...)` in `Hooks::register`). Mob goals are an `EntityPart::register_goals` with a
  `GoalRegistrar` (`add_goal`, `add_target_goal`), goals reading and writing through a `GoalContext` (`move_to`,
  `look_at`, `jump`, `set_target`). Per-entity saved data is an `EntityPartState` (an attachment).
- **Rendering**: `Hooks::register_entity_renderers` registers an `EntityRenderer` per type; its `submit` gets an
  `EntityRenderState` (with a `LivingRenderState`: `body_rot`, `walk_animation_pos/speed`, `death_time`, `scale`,
  `is_baby`, `has_red_overlay`...) and a draw context with the same primitives our `creature::model::draw` uses
  (`cube`, `model_part`, `push`, `pop`, `translate`, `rotate_about`, `scale`). The pose starts at the feet, not
  turned: the renderer turns by `body_rot` itself. A renderer has **no host**: per-entity values are read in a hook
  and kept.
- **Engine internals** stay reachable from native code, but only through declared routes in the manifest:
  `[[native.mixin]]` (inject into a game function), `[[native.bridge]]` (a private field or method),
  `[[native.module]]`, `[[native.loop]]`, plus `internals` declarations. Reading `Mob` fields directly from the API
  crate is gone.
- **Prebuilt game**: the launcher compiles only the mod, against a prebuilt SDK of the pinned build; the game
  itself is never rebuilt.

## 2. Packaging and tools

| Today (Horizon) | On Sunrise | Our change |
|---|---|---|
| Two halves per module: `content/` (`.hmod`, `horizon-mod.toml`) and `native/` (`.hnmod`, `horizon-mixin.toml`, `api/` crate) | **One `.smod` per module**: a flat project with `sunrise-mod.toml`, `Cargo.toml`, `src/lib.rs`, `assets/`, `data/` at its root (optional `icon.png`) | Merge each module's `content/` and `native/api/` into one folder; `horizon-mixin.toml` and `horizon-native.json` are written by the launcher only, never by us |
| `[mod] id/version` + `[compatibility] horizon_revision` | `id`, `name`, `version`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native] game = "26.3"`, `sunrise = "<40-hex commit>"`, `mixins = "<40-hex horizon-mixins commit>"` | One pin for the series, now two commits (Sunrise and horizon-mixins); `cargo sunrise-mod new/update` writes them |
| `horizon-client = { git = ..., rev = ... }` in `api/Cargo.toml` | `sunrise-mod-api = { path = ".sunrise/sdk/api/crates/sunrise-mod-api" }`, the **only** accepted form (`E_SDK_SOURCE` otherwise) | `.sunrise/` fetched by any `cargo sunrise-mod` command, git-ignored, never packed |
| `[dependencies] creature = "^0.1"` in `horizon-mixin.toml`; none for content | `[dependencies]` in `sunrise-mod.toml`, for content and code alike (`version`, `optional`, `side`, `order`, `reason`, `url`); the launcher adds the `creature` crate as a path dependency | Same requirement, one place. Optional mod-to-mod content can use `optional = true` + `mod_api::if_installed!` beside our conditional recipes |
| `data/<id>/horizon/{entity_type,item,block,creative_tab}/` | `data/<id>/sunrise/...` | Rename the folder in every module |
| `cargo horizon-mod` | `cargo sunrise-mod` (`new`, `build`, `run`, `check`, `update`, `use-jar`, `internals`) | Installed (§6) |
| Content in `mods/`, native in `horizon-native-mods/`; `horizon-mods.txt` | Content and wasm `.smod` in `sunrise-mods/`, native `.smod` (Rust source) in `sunrise-native-mods/`; `mod.<id>:true` in `sunrise-mods.txt` | `tools/src/install.rs` |
| `horizon-launcher native-build` | `sunrise-launcher native-build [FOLDER] --instance <name> --trust-native-code` (a folder of mod projects, or the instance's installed ones) | `tools/src/install.rs`, `build.rs`, `pack.rs` |
| Launcher data `~/.local/share/horizon-launcher`, `$HORIZON_LAUNCHER(_DATA)` | `~/.local/share/sunrise` (`launcher/`, `game/`), AppImage `sunrise-launcher-linux-x86_64.AppImage` | `tools/src/launcher.rs` (`SUNRISE_LAUNCHER`, `SUNRISE_LAUNCHER_DATA`) |
| Instance `creature-dev` | `sunrise-launcher instance create creature-dev` | Done |
| Registry: playhorizon.dev/mods/submit | The Sunrise registry (`cargo sunrise-mod check` runs its checks) | ❓ the submission path, at release time |

Every tool command follows: `pack` and `build` produce `.smod`, `check` validates `sunrise-mod.toml` and the pins
(and can call `cargo sunrise-mod check`), `native-check` builds against the SDK instead of a Horizon checkout, CI's
`HORIZON_REV` becomes the Sunrise and mixins pins.

## 3. Content (data and assets)

**Tried 2026-11-17 ✅**: the content halves of the library, worms, vulture, butterflies and scorpions, with only
`horizon/` renamed `sunrise/` and `[dependencies] creature = "^0.1"` added to `horizon-mod.toml` (renamed
`sunrise-mod.toml`), pass `cargo sunrise-mod build` and `check`, and load in game with no refusal: `/summon` makes
each mob (drawn as a wandering trader until its renderer is ported), items show with their textures and names.

Most of it moves as is: items, recipes, loot tables, advancements, tags, lang, textures, sounds and NeoForge biome
modifiers (`data/<id>/neoforge/biome_modifier/`, our spawns) are read the same way. To change:

- `data/<id>/horizon/` → `data/<id>/sunrise/` (entity types, items, blocks, the creative tab).
- **Living templates**: still `minecraft:zombie`, `villager` or `wandering_trader`, plus now the tameable
  `minecraft:wolf`, `cat` and `parrot` (taming, sitting, following, owner saved). ❓ Whether the vulture should move
  to a tameable template (its own taming code would shrink) or keep the trader.
- **New entity-type keys** that replace workarounds: `sounds` (`ambient`, `hurt`, `death`, `step`, or `"none"`),
  `shadow_radius` / `shadow_strength`, `passenger_attachments` (the vulture's seat). See §5.
- **Creative tab**: same `creative_tab/*.json` format; to remove the jar from it when no jar mod is on, the client
  hook `build_creative_mode_tab_contents` replaces the `jar-tab` injection.
- **Conditional recipes**: `neoforge:conditions` with `mod_loaded` / `not` are kept (built-in conditions, plus
  `register_load_conditions` for our own). ✅ No change expected; ❓ confirm in game.
- **Brewing recipes** of a mod (fairies, scorpions `recipe/brewing/`): not documented in the book. ❓ Check whether
  Sunrise's brewing loader reads mod namespaces (the fork had it).
- **Mod enchantments** (scorpions' Poisonous): loaded from `data/<ns>/enchantment/` and joined to vanilla's tags; the
  effect itself is our code. ✅ Supported, our "not loaded at our pin" workaround can go.

## 4. Native code

About 17,700 lines of native Rust (library and eight mods) are written against `horizon-client`'s `Hooks` and
`&mut dyn Host`. Every module is a rewrite against `mod_api`, mostly mechanical but touching every file. Our hook
methods against Sunrise's catalog (2026-11-17):

| Our hook | On Sunrise |
|---|---|
| `entity_tick_post`, `level_tick_post`, `living_death`, `living_incoming_damage`, `use_item`, `use_item_on`, `tooltip`, `entity_join_level`, `entity_leave_level`, `entity_tame`, `entity_breed`, `entity_enter_love_mode`, `entity_drop_item`, `entity_change_block`, `living_change_target`, `loot_drops`, `explosion_detonate`, `explosion_prime`, `player_bucket_entity`, `player_logged_in`, `register_commands`, `menu_clicked`, `menu_closed`, `anvil_update`, `block_entity`, `block_entity_submit`, `block_entity_update_tag`, `screen_render_post`, `screen_mouse_*_pre`, `key`, `visuals`, `emissive` | Same names; signatures change with the host split and events (e.g. `living_death` now has `source: &HurtSource` and `weapon`, no `damage_type`/`attacker`) |
| `register_goals` (7 mods) | `EntityPart::register_goals` on the mod's entity type, goals as `mod_api::Goal` |
| `mob_submit` (7 mods) | `EntityRenderer::submit`, registered in `register_entity_renderers` |
| `interact_entity` (9 uses) | `entity_interact` / `entity_interact_specific` / `item_interact_mob` |
| `interaction_key` | `interaction_key_mapping_triggered` |
| `perform_command`, `tab_complete_v` | `command` / `run_mod_command`, `tab_complete` |
| `piston_move`, `destroy_block`, `entity_transform` | `piston_post` / `piston_push_reaction`, `living_destroy_block` / `break_block`, `living_conversion_pre/post` |
| `net_from_server` | Mod payloads (book: client networking, packets) |
| `menu_input_closed` | ❓ no direct counterpart found (`menu_closed`?) |

Port order ✅: the library first (it is every mod's dependency), then **worms** as the spike (small, ground mob,
goals + renderer + jar), measure the effort, then the other mods, the vulture last (riding, HUD, screen).

### The spike, stage 1 (branch `sunrise`, 2026-11-17) ✅

The library's `model`, `mob`, `crawl`, `sounds`, `colours` and `territory`, and worms without the jar and the
bestiary, are ported, built by `sunrise-launcher native-build` (25 s the first time, seconds after) and tried in game:
worms are drawn with their model, wiggle, crawl over the grass, cast a small shadow, take no trades and have none of
the trader's AI. Effort: about an hour for ~1,300 lines, mostly mechanical. What it taught:

- **Layout** ✅: a module's root holds its package (`sunrise-mod.toml`, `Cargo.toml`, `src/`, `assets/`, `data/`),
  and our own files beside it (`module.toml`, `SPECS.md`, `sources/` with the `.bbmodel` and `sounds.toml` that were
  in `assets/`). A package refuses unknown root files (`module.toml`), so the tools stage the package files in
  `target/stage/<id>/` and build from there (by hand for now).
- **Building**: `sunrise-launcher native-build target/stage --instance creature-dev --trust-native-code` builds every
  staged module and selects the build. `cargo sunrise-mod build` of a mod with a dependency wants the rustup toolchain
  `1.99.1-x86_64-unknown-linux-gnu`, which rustup won't link from the launcher's copy: skip it, the launcher builds.
- **Checking locally**: `cargo check` works against `.sunrise/sdk/` with the launcher's Rust
  (`~/.local/share/sunrise/launcher/rust/1.99.1-x86_64-unknown-linux-gnu/bin`, no Clippy there; the system's
  stable Clippy works). A mod needs the `creature` crate the launcher adds: a check copy with
  `creature = { path = ".../common", package = "mod-creature" }`, every crate on one SDK copy (two copies collide).
- **Goals**: an `EntityPart`'s goals join the template's. The library takes the template's off in
  `entity_join_level` (`MobRef::goals` / `remove_goal`, every goal not named `creature:...`): a trader's are
  `float`, `DrinkItem` (twice), `TradeWithPlayer`, `avoid_entity` (7), `panic`, `LookAtTradingPlayer`,
  `WanderToPosition`, `MoveTowardsRestriction`, `stroll`, `InteractGoal`, `look_at_player`. A goal can't write
  velocity or yaw: it steers through `move_to` (path), `look_at`, `jump`; keep `move_to`'s speed near 1 and the
  slowness in the `movement_speed` attribute (0.07 left the worm still).
- **Renderer**: the pose starts at the feet, unturned; `creature::model::living_pose` turns it by `body_rot` (180 -
  yaw, as vanilla) and tips a dying mob. Our generated models draw the right way up with it.
- **Debugging**: a native mod's stderr reaches no log; write to a file while debugging.

### Stage 2: the whole library (2026-11-17) ✅

Every module of the library compiles on `mod_api` (Clippy clean, its tests pass) and worms uses it all. Tried in
game: a jar catches a worm (survival: one jar used, the filled one in the inventory) and releases it on a garden,
the guide opens the book, which shows the worm's page with the facts just learned. What it taught:

- **Motion by hand** (`creature::body`): a goal can't set velocity, but a server hook can (`EntityRef::set_velocity`,
  `set_rotation`, `set_head_yaw`, `set_no_gravity`, `reset_fall_distance`). Our motion goals (`flutter`'s flight,
  `projectile`'s `Flight`, the mods' own) become `Behaviour`s on a `Body` (the mob read each tick), run from the
  mod's `entity_tick_post` (`body::tick`) and written back: the old goal code ports almost line for line. A
  projectile's hit is applied after (`projectile::tick_post`, `EntityRef::hurt`). There is no horizontal-collision
  read: a wall is a projectile slowed to under a third of its throw.
- **Items**: `use_item` and `use_on` are `ItemPart` methods (`r.item_part(r.item_key(id), part)`), not global hooks:
  the guide's and the jar's. Hand reads and writes are direct (`LivingRef::hand_item`, `set_hand_item`): the Horizon
  trick of marking a stack to find the selected slot is gone. `entity_interact` runs on both sides: answering on the
  client too stops the trade screen and the off hand (arismoko#3897 needs nothing more).
- **Placed jar**: a block entity type declared in `register` (`r.block_entity_type("jar")`) with a `BlockEntityPart`
  (`block_entity`, `block_entity_update_tag`, `block_entity_submit`); its data written with
  `BlockEntityRef::merge_data`. Pistons: `"push_reaction": "IMMOVEABLE"` in the block's JSON (no hook).
- **Book**: the screen is still `mod_screen_open`; drawing is on the event's `DrawContext` (`fill`, `text`,
  `gui_draw_item_with`, `item`); text colours need an alpha byte (`0xFF......`) or nothing shows; translations come
  from `ClientHost::translate`. The bestiary goes to the client as a typed payload (`play_to_client_typed`, a
  `String` chunk), whose handler reaches the client host through `cx.client_host()`. `/bestiary` is a typed command
  (`Command::op`, `run_mod_command`, `CommandSource::Player`).
- **The book's 3D view** draws a vanilla creature itself (`DrawContext::entity`, `GuiEntity::kind`), which Horizon
  couldn't. ❓ **Ours draw nothing that way** (the renderer is called, no error, nothing shows; a vanilla cow draws):
  their icon stands in. To investigate, then report on sunrise-org/sunrise.
- **Gone**: the creative tab injection (`build_creative_mode_tab_contents` hides the jar), the piston hook, the
  "switched off in Options > Mods" filter (Sunrise's Mods screen has no switch).

### Stage 3: layout and tools (2026-11-17) ✅

Every module is on the Sunrise layout (its package at its root; `sources/` for the Blockbench models, `sounds.toml` and
Horizon's old manifests). The tools follow: `pack` stages a module in `target/stage/<native|content>/<id>/` and zips it
to `target/<id>.smod`; `build` checks each with `cargo sunrise-mod check`; `install` builds the ported ones with
`sunrise-launcher native-build target/stage/native` and puts the others' content in `sunrise-mods/`; `check` reads the
`sunrise-mod.toml` pins (and CI's `SUNRISE_REV`); `native-check` compiles the ported crates on the SDK; `model`,
`preview`, `sounds`, `tab`, `recipes`, `vanilla` use the new paths. A module's Rust is built only once its
`module.toml` says `ported = true` (the library and worms today). Tried: `cargo creature install` puts all nine
modules in `creature-dev`, which loads them with no refusal.

### Left to port

The other mods' Rust, each onto the library's new shape (`EntityPart` goals or `body` behaviours, an
`EntityRenderer`, item parts, the host split), then `ported = true` and a test along its spec's "Testing in game":
butterflies and fireflies (flutter, colours), fairies (friend, flutter), scorpions (crawl, projectile, enchantment),
cyclops (projectile), the Vanilla Bestiary (vanilla creatures, now drawable in the book), the vulture last (riding,
its HUD, its screen: §5). Open points: our creatures in a screen (§4, stage 2), mod brewing recipes, the rider's input
(§5).

## 5. Our workarounds on Sunrise

Each line of docs/HORIZON-ISSUES.md, as Sunrise stands (book and API of `559fd52`; ❓ until tried in game):

| Issue | On Sunrise | What the port does |
|---|---|---|
| #13975 seat height | `passenger_attachments` in `entity_type` JSON | Hitbox back to ~1 block, seat declared |
| #6276 entity size | `entity_size` hook (NeoForge `EntityEvent.Size`) ✅ worms | Drop the cloned `MobType`, answer `entity_size` (`creature::mob::ours`) |
| arismoko#2738 mob sounds | `sounds` in `entity_type` JSON ✅ worms | Declare ambient, hurt, death, step; drop the muting and the sound queue (flaps and eating stay ours) |
| arismoko#3896 shadows | `shadow_radius`, `shadow_strength` in `entity_type` JSON ✅ worms | Declare each mob's shadow |
| arismoko#2740 render state in statics | Synced attachments (`sync: true`) readable on the client through `Host::attachments`; the renderer itself has no host | Keep render state per entity id, filled from synced attachments in a client hook: works in multiplayer |
| arismoko#2739 rider's sneak / #13972 client-side click | Server `player_input` hook (the `PlayerInput` flags, sneak included) ❓ fires for a single-player host?; `entity_mount` can cancel a dismount; `entity_controlling_passenger` | Drop `ride_keys.rs`; the vulture's `Fly` goal reads the rider's input from the server side |
| #13973 `saddled` steering | Riding is the engine's; no `Mob.extra` field from the API | Harness in an `EntityPartState` |
| #13974 `open_menu` from a click | ❓ test from `entity_interact` | Open the screen from the click |
| arismoko#2741 mount hearts | `gui_layer_pre` on `vehicle_health` ❓ heart count | Drop the `mount-hud` injection if the layer hook is enough, else a `[[native.mixin]]` |
| arismoko#4357 `mob_kind` prefix | ❓ check `living_death`'s `kind` on a mod mob | Compare `event.kind` directly |
| arismoko#3897 off-hand use | ❓ retest | Drop `hand::holds_server_used` if fixed |
| #13971 left-out mod exits 0 | ❓ retest with `sunrise-launcher native-build` | Keep the `build.log` check as a safety net |
| Jar removed from the tab (injection) | `build_creative_mode_tab_contents` | Drop `common/native/jar_tab.rs` |
| A vanilla mob in a screen (bestiary) | `DrawContext::draw_scene` with `SceneEntity` / `GuiEntity::kind("minecraft:cow")` | Draw the creature in place of the turning spawn egg |
| `Host::take_items` | Gone; inventory capabilities | Rewrite with the item handles |

## 6. Running Sunrise here

- `sunrise` in the applications menu (`sunrise-launcher desktop-entry`) starts the launcher. Signed in (account
  `Anathos_`), Nightly prepared, instance `creature-dev` made.
- Driving the game: start it with `sunrise-launcher play --instance creature-dev --world <world> --arg --control`, then
  `sunrise-ctl` (in the version folder, `~/.local/share/sunrise/launcher/versions/<version>/sunrise-ctl`): `wait-ready`,
  `command "<command>"`, `screenshot <path>`, `entities`, `logs`... Mods by hand: `.smod` files in the instance's
  `sunrise-mods/`, a line `mod.<id>:true` each in its `sunrise-mods.txt`.
- `cargo sunrise-mod` is installed with `cargo install --locked --git https://github.com/sunrise-org/sunrise
  cargo-sunrise-mod` (`cargo binstall` falls back to crates.io, where the crate isn't published).

- Launcher: `~/Téléchargements/sunrise-launcher-linux-x86_64.AppImage` (1.0.1), data in `~/.local/share/sunrise`.
  The Nightly `nightly-2026-11-17-2d81263` and the launcher's Rust 1.99.1 are installed.
- **Prepare needs Minecraft 26.3**: with no 26.3 in `~/.minecraft` (only snapshots), the launcher downloads it from
  Mojang once signed in.
- Every launcher command prints "Could not copy the earlier launcher's data: Symbolic links and reparse points cannot
  be relocated: ~/.local/share/horizon/.horizon-cache/mod-content/asset-index.json": the carry-over from Horizon's
  game folder stops on a symlink into `horizon-launcher/versions/`. Harmless for us (nothing of Horizon's data is
  needed). **To open** on sunrise-org/sunrise if it persists.
- The MCP server: `sunrise-launcher mcp` (the `horizon` server in `.mcp.json` still drives Horizon's launcher).

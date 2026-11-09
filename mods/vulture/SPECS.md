# Vulture: specification (v0.2)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3).
It adds a wild vulture that you tame by throwing raw chicken at it, ride, fit with chests, that catches creatures in its
talons and breeds.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)). Systems the
library shares are described there; this spec gives the vulture's own rules and numbers.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-09)

### Done

- **Phase 0 succeeded** (§1.2): entity `vulture:vulture` (a copy of the wandering trader with its goals replaced), drawn
  by the mod, ridden and flown. Tested in game, single player.
- **Mounting**: right-click on the vulture (no cheats needed) or `/ride` (cheats). Holding sneak gets off (confirmed in
  game).
- **Ridden flight** (§6.2), tried in game 2026-11-09: turned by the look, Space climbs, sneak descends, getting off only
  on the ground. The vulture takes no fall damage.
- **Model and texture of the tawny vulture**: `mods/vulture/assets/models/vulture.bbmodel` (Blockbench), converted to
  Rust by `cargo creature model vulture` and drawn by the library (`creature::model`). First version (17 cubes, 64×64)
  replaced on 2026-11-09 by the lean model of §7.1 (32 cubes, 128×128); the first version was tested in game, the lean
  one only in Blockbench so far.
- **Mount hearts**: only the hearts it has, no empty ones.
- **Sounds** (§7): the trader is muted; call (hiss, 3 variants + a rare call), hurt, death, wing flaps, takeoff and eat
  (those two not played by the code yet), made from CC0 freesound sounds by `cargo creature sounds vulture` (sources
  listed in `tools/src/sounds.rs`), played by the library (`creature::sounds`). English and French subtitles. Hurt and death: written, not tested in game yet.
- **Spawn egg** `vulture:vulture_spawn_egg`, in the series' "Horizon Creature" creative tab (2026-11-08; the vulture's
  own tab is gone).
- **Moved into the Horizon Creature series** (2026-11-08): `mods/vulture/`, depends on the library; built with the
  series' tools (`cargo creature install vulture`), tested in game.

- **Drops** (§4, 2026-11-09): `vulture:feather`, `vulture:raw_vulture` and `vulture:cooked_vulture` (mutton's food
  values; furnace, smoker and campfire), the loot table (0–2 feathers, 1–2 meat, cooked when it burns, Looting), in the
  creative tab. Built; not tested in game yet.
- **Wild vulture, first part** (§4, 2026-11-09): it soars (takes off every ~30 s on average, climbs 18 to 26 blocks,
  circles 30 to 60 s with a bank, spirals down and lands; left in the air, it glides down), walks about, looks at
  players; neutral, it fights back (3 damage) and nearby vultures join in (vanilla's `HurtByTarget` and
  `MeleeAttack`); spawns in deserts, badlands and savannas (weight 3, groups of 1 to 3, a NeoForge biome modifier).
  The flight is shared with the ridden one (`flight.rs`). Built; not tested in game yet.
- **Taming and the tamed vulture** (§5, §6, 2026-11-09): a hungry vulture walks to raw meat on the ground (16 blocks)
  and eats it; a wild one eating a raw chicken is tamed 1 time in 4 (hearts, smoke otherwise), by the player. Tamed:
  sits or follows (right-click), flies to its owner past 10 blocks and lands near them, teleports past 32, attacks
  what hurts its owner and what they attack, never its owner; eats from the ground only when hurt (+4). Its owner's
  right-click: harness on, a chest on (two at most), raw meat heals 4 when hurt, then mount (with a harness) or sit /
  follow. **Harness** `vulture:harness` (recipe, texture); harness and chests drawn on the model when worn. State in
  `Mob.extra`, saved in the `vulture:state` attachment (`state.rs`). Built; not tested in game yet.
- **The vulture's screen and chests** (§6.1, §6.3, 2026-11-09): sneak + right-click by its owner opens it (a server
  menu shown as a chest): harness (click to take it off, without chests), sit / follow, a button per chest (takes the
  last one off, its contents given back), and 15 slots per chest, saved in the `vulture:chests` attachment. On death
  it drops its harness, chests and their contents. Built; not tested in game yet.
- **Talons** (§6.4, 2026-11-09): ridden in flight, R (`key.vulture.talons`) catches the nearest small mob (at most
  1 × 1) or tamed animal up to 3 blocks below and carries it (−25 % cruising speed, half the climb); R again, the rider
  getting off or its death lets go. Built; not tested in game yet.
- **Stats, breeding and babies** (§2, §8, 2026-11-09): each vulture rolls its max health (16 to 30) and flight speed
  (0.6 to 1.2 × base) once, saved. Rotten flesh from its owner puts a grown tamed vulture in love for 30 s; two in love
  within 8 blocks meet and have a baby (tamed to the owner, each stat the average of both parents' and a roll), then
  wait 5 minutes. A baby is drawn at half size, can't wear a harness or be ridden, grows up in 20 minutes, a minute
  faster per raw meat fed. Its age is its own (Horizon doesn't age a wandering trader's copy), saved with its state.
  Built; not tested in game yet.
- **Hitbox, fighting in the air, animations** (§4, §7, 2026-11-09): its own 1.2 × 1.0 hitbox (a copy of its type with
  its own size, in place of the trader's 0.6 × 1.95); it fights on the wing (`DiveAttack`: climbs above its target,
  dives, strikes for 3, pulls away; in place of vanilla's ground melee); poses for pecking (eating), striking, sitting
  (settled, legs folded) and carrying (legs down). Built; not tested in game yet.
- **Landing on high points** (§4, 2026-11-09): after circling, the wild vulture picks the highest ground within 16
  blocks (a peak, a tree top) and glides down onto it. Built; not tested in game yet.
- **The baby's model** (§7, §8, 2026-11-09): a chick of its own (`assets/models/baby.bbmodel`, 64 × 64): a big
  grey-pink head with a downy crown, a short neck, a round downy body, stubby wings, a dark beak; the same groups as the
  grown vulture, so the same poses. Built; not tested in game yet.

### To do (v1)

- §6.2 Flight: Space / sneak for height and getting off only on the ground, written after the first flights (to try);
  the numbers, bank and stall to tune.
- §7.1 Lean model: written (2026-11-09); to check in game with its poses (folded, gliding, flapping, banking, diving).
- §7.2 New spawn egg: drawn (2026-11-09); to check in game.
- **v1 is written.** What is left is testing everything above in game, then tuning (flight numbers, taming odds,
  speeds, the dive attack) and fixing what the tests find.

What waits for v2 is listed apart, in §9.

---

## 1. Technical context

- **Format**: a Horizon native mod = a content mod (items, recipes, loot, textures, sounds, entity type in JSON)
  + Rust code built into Horizon (`horizon-mixin.toml`, `native_*` hooks). Tool: `cargo horizon-mod`.
- **Toolchain**: Rust 1.99.1, pinned by the launcher and already installed.
- **Target**: game 26.3, Horizon revision pinned at the start of development.
- **Mod id**: `vulture` (ids kept from before the series, so test worlds still load).
- **Dependency** ✅: the library `creature` (`[dependencies] creature = "^0.1"` in `native/horizon-mixin.toml`). Entity `vulture:vulture`, items `vulture:harness`, `vulture:feather`, `vulture:raw_vulture`,
  `vulture:cooked_vulture`.
- **Multiplayer** ✅: v1 = single player. LAN and the dedicated server come in v2 (for a LAN guest, their own client
  moves the mount, which needs its own work), but server logic is kept apart from rendering from the start.
- **Languages** ✅: everything in the repository is in English; the game text is translated (`en_us`, `fr_fr`).

### 1.1 What the Horizon API allows today (read in Horizon's `docs/mods.md` and `docs/native-mods.md`)

| Need | Available | Notes |
|---|---|---|
| New living entity type | ✅ tested in game | Only by copying a **zombie, villager or wandering trader**. `vulture:vulture` (wandering trader) works: `/summon`, egg, saving. |
| Custom AI | ✅ | `register_goals`: we clear the base model's (wandering trader's) goals and add ours |
| Custom model and texture | ✅ tested in game | `native_mob_submit`: we draw our cubes (`native_draw_cube`, pose stack, pivots) with `vulture:entity/...` textures |
| Per-entity saved data | ✅ | Server attachments on `Holder::Entity(id)` (owner, stats, variant, chests…) |
| Right-click on the entity | ✅ | `native_interact_entity` |
| Taming event | ✅ | `native_entity_tame` (vanilla only, to check) |
| Nearby items on the ground | ✅ | API 42 (position, stack, pickup delay) |
| Mod sounds | ✅ event played in game (`.ogg` files not listened to yet) | `assets/vulture/sounds.json` + `.ogg`, played by `Host::play_sound` (Horizon always prefixes mob sounds with `minecraft:`) |
| Inventory screen | ✅ | Mod menu types (1 to 256 slots) |
| Natural spawning | ✅ | `add_spawns` by biome |
| **Riding a mod entity and flying it** | ✅ phase 0 (2026-11-07) | `/ride` accepts any mob; Horizon copies the rider's keys into `Mob.rider` (forward, strafe, jump, sprint), which a goal reads to set the speed. Internal fields: Horizon revision pinned. Right-click → mount: a mixin injection that calls `start_riding` (`run_command` is refused during a player's turn). No cheats needed. |
| Carrying an entity (talons) | ❓ | Probably by making a creature another's passenger, or by teleporting it every tick. To check. |

### 1.2 Phase 0: technical prototype, before anything else

1. Declare `vulture:vulture` (a copy of the wandering trader), spawn it with `/summon` and draw it with a cube.
2. Mount a player on it and read their keys (WASD, Space, Ctrl) on the server.
3. Make it fly by setting its speed directly.

If step 2 or 3 is not possible, three options:
- **A**: contribute to Horizon (open source) a mount-type base model (horse, donkey, happy ghast);
- **B**: spawn a vanilla donkey, which can already be ridden, carry chests, vary its stats and breed, and draw it as a
  vulture;
- **C**: write the mount ourselves with player input and position hooks.

---

## 2. Variable stats ✅ (like horses)

> Shared system: common/SPECS.md §8.4. The vulture's ranges and rules below.

Each vulture rolls its stats when born or spawned:

| Stat | Range | Effect |
|---|---|---|
| Max health | 16 – 30 | |
| Flight speed | 0.6 – 1.2 × base speed | Cruising and diving speed (§6.2) |

- **Breeding**: as with vanilla horses, each stat of the baby is the average of both parents and a random value. Good
  bloodlines improve over time without passing the caps.
- Stats are never shown in game ✅, as in vanilla.

## 3. Skin variants (v2) ✅

**Not in v1** ✅: v1 has one look, the tawny vulture. The variants come in v2 (§9). Inspired by real species, each with
its own texture:

| Variant | Inspiration | Spawning |
|---|---|---|
| Tawny (`tawny`, FR fauve) | Griffon vulture (beige, white ruff) | Common, all biomes |
| Dark (`dark`, FR sombre) | Cinereous vulture (very dark brown) | Common, badlands |
| White (`white`, FR blanc) | Egyptian vulture (white, yellow face) | Common, desert |
| Bearded (`bearded`, FR barbu) | Bearded vulture (orange breast, black "beard") | Uncommon, savanna plateau / nearby mountains |
| King (`king`, FR royal) | King vulture (multicoloured head) | Rare (~3 %), everywhere |

Variant names are simple colour names ✅, so players need not know the species.

- A baby takes the variant of one of its parents (50/50), with a ~5 % chance of mutating to a random variant ✅.
- Secondary markings (spots, darker wings) could multiply the combinations, as for horses: later still ✅.

## 4. Wild vulture

| Property | Value |
|---|---|
| Size (hitbox) | ~1.2 × 1.0 blocks; visual wingspan ~4.5 blocks in flight (§7.1). Horizon gives a mod mob its template's size (the trader's 0.6 × 1.95): we try overriding it at spawn, else we keep the trader's ✅ |
| Behaviour ✅ | **Neutral and retaliates**: attacks whoever hits it (3 damage), and nearby vultures join in, like wolves/llamas |
| Biomes ✅ | Desert, badlands, savanna; groups of 1–3 |
| Wild flight | Slow circles high up, lands on high points |
| Scavenger | Drawn to meat on the ground and recently dead mobs (~16 blocks) |
| Drops ✅ | 0–2 **vulture feathers** + 1–2 **raw vulture meat** (cookable, ordinary meat ✅) |

## 5. Taming ✅

> Ownership, sit/follow and defending the owner are shared (common/SPECS.md §8.3); taming by thrown chicken is the
> vulture's own.

1. The player **throws** a raw chicken (Q key) near a wild vulture. Handing it over does not work.
2. The vulture spots the item (~16 blocks), lands on it and eats it (animation + particles).
3. **1 chance in 4** per chicken: hearts on success, smoke otherwise.
4. The owner is the player who threw the chicken.
5. Other raw meats are eaten with no effect; cooked chicken is ignored.

## 6. Tamed vulture ✅ (like the wolf)

- **Sit / follow**: when following, it flies above its owner and teleports if too far away.
- Defends its owner and attacks what the owner hits, like the wolf.
- Can be named with a name tag. No fall damage. Healing: raw meat (+4 HP).
- **Controls** ✅:
  - right-click = mount if it has a harness and you are its owner (only the owner rides it ✅), otherwise sit/follow;
  - sneak + right-click = the vulture's screen (which also has a sit/follow button);
  - right-click with rotten flesh = breeding mode (§8).

### 6.1 Harness ✅

- **Vulture harness** item, unstackable, recipe: 5 leather + 2 string + 1 iron ingot (U shape, iron in the middle) ✅.
- Needed to ride it **and** to carry chests ✅. Goes in a slot of the vulture's screen, and shows on the model.

### 6.2 Dynamic flight ✅

> Riding and flight are shared with Wyverns (common/SPECS.md §8.1). The vulture's keys and numbers below. **No
> stamina** ✅: the vulture flies as long as the rider wants.

Flown like a plane, not like a flying camera: the vulture always moves forward along its heading, the rider turns it
by **looking** where it should go ✅ and sets its height with **Space and sneak** ✅ (tried in game, 2026-11-09).

| Rider | Vulture |
|---|---|
| Looks **right / left** | **Banks** to that side and **turns in an arc** ✅, like a plane: the further the look is from its heading, the steeper the bank (up to ~50° ❓) and the tighter the turn (up to ~5° a tick ❓); it levels out as its heading reaches the look |
| W (Z on AZERTY) | Flaps: speeds up to its cruising speed (wing beats, flap sound) |
| (nothing) | Glides: keeps its speed slowly falling off, sinks gently when level |
| S | Air brake: slows down, sinks a little faster ❓ |
| Space (held) | Noses up and **climbs** with strong flaps ✅ (loses speed); on the ground, **takes off** |
| Sneak (held) | Noses down and **descends** ✅ (gains speed, up to the dive speed); Ctrl does the same |
| (neither) | Flies level |
| Sneak on the ground | Get off ✅: **only on the ground**, never in the air |
| "Talons" key ✅ (R, rebindable in Controls) | Catch / release (§6.4) |

- **Speed and pitch** trade like a glider's ✅: diving builds speed, climbing spends it. Too slow while climbing, it
  stalls and noses down by itself ❓, so it can't hang in the air.
- The **body follows the flight**: pitched with the climb or dive, rolled with the bank, the wings spread wide in a
  glide, beating when it flaps, swept back in a steep dive ❓.
- The pitch and the turn rate follow the look smoothly (no snapping), so the flight reads as momentum.
- A and D (Q and D on AZERTY) do nothing in the air ❓; the look's pitch does nothing either (the camera is free).
- On the ground: a slow walk (WASD), Space takes off, sneak gets off.
- Horizon gets a rider off on sneak before any mod hears of it: the `ride-keys` injection (`ride_keys.rs`) keeps them
  on in the air.
- ✅ Chests **do not weigh** on its flight. ✅ **No thermals.**
- Starting numbers ❓ (blocks per tick): cruise 0.6, dive up to 1.2, slowest 0.25 before a stall, glide sink 0.04 when
  level, take-off climb 0.4.

### 6.3 Chests ✅

- Up to **2 chests**, one on each side, visible on the model. Added by right-clicking with a chest (harness required).
- **15 slots per chest**, so 30 in all.
- **Vulture screen**: harness slot, "remove left / right chest" buttons ✅ (the contents drop on the ground), a grid of 15
  or 30 slots, sit/follow button.
- On death it drops its chests, their contents and its harness.

### 6.4 Talons ✅

- In flight, ridden, with the talons key: catches the creature just below (~3 blocks).
- It can catch:
  - **small enough mobs** (hitbox ≤ 1 block wide and ≤ 1 block high ✅: chicken, rabbit, pig, sheep, fox, cat, wolf,
    babies…);
  - **allies** ✅: a player sneaking under the vulture (sneaking means consent), or a tamed animal.
- Carrying someone **weighs it down** ❓: cruising speed −25 %, and it climbs at half the rate.
- The key is a mod key (`key.vulture.talons` in `horizon-mods.txt`, R by default); a sneaking player under it can't be
  caught in single player (the only player is its rider), so allies are tamed animals for now.
- It lets go on the key, when the vulture dies or when the rider gets off. The fall deals normal damage.
- A wild or unridden vulture catches nothing ✅.

## 7. Graphics and audio

- The **model is drawn by code** (`native_mob_submit`) from cubes. Its source is a `.bbmodel` file (Blockbench),
  converted to Rust data by a script in the repository: we edit visually, the game gets cubes, pivots and UVs.
- Animated parts: head, neck, body, 2 wings in three segments (arm, forearm, hand with its primaries), tail, 2 legs with
  talons, harness, 2 chests.
- Code animations: flap, glide (wings spread, still), banking turn, dive (wings swept back), walk, peck/eat, sit,
  attack, closed talons.
- Sounds: call, wing flap, eat, hurt, death. CC0 sounds (freesound.org, licence checked; CC0 requires no credit, the
  sources are listed in `tools/src/sounds.rs`), edited.
- English and French translations.

### 7.1 Model ✅ (redesign)

**More worked and imposing** ✅, after photos of griffon and Rüppell's vultures (landing with wings spread, perched,
wings open on a post), and **lean, not stocky** ✅ (a first version looked like a turkey): the profile of a hunched
vulture.

- **Silhouette** ✅: a long, narrow, low body (8 × 7 × 16 texels) whose back **slopes down to the tail** on the ground,
  a cream **ruff** at the shoulders, the neck **reaching forward** from it at shoulder height (not upright), a small bald
  head held low with a **heavy hooked beak**, short legs with feathered thighs, grey feet and dark talons, a short square
  tail. No breast bulge.
- **Wings spread** ✅, the impressive part: long and broad (span **~4.5 blocks**), three segments (arm, forearm, hand)
  so they fold along the flanks on the ground and open fully in flight. The hand ends in **separate primary feathers,
  the "fingers"** ✅ (5 per wing). Two-tone like the photos: scaly, mottled brown-and-cream **coverts** on top, then a
  pale band, then **dark brown flight feathers**.
- **Folded** on the ground, the wings cover the flanks, coverts on top, and their primaries reach past the tail.
- **Colours** (tawny, v1): mottled brown and cream body, cream ruff, grey-pink bare head and neck, a horn-coloured beak
  with a dark tip, dark eyes.
- Texture **128 × 128** ✅ for the detail.
- **In flight** the neck stays forward, the legs are tucked.

### 7.2 Spawn egg ✅ (redesign)

- In the style of the **latest vanilla spawn eggs** ✅ (since 1.21.5, each egg is drawn for its mob instead of two
  colours with spots): an egg shape with a dark outline and shading, painted with the mob's look.
- The vulture's: a mottled brown egg with dark flight-feather streaks on its sides, a cream ruff collar around its top
  third, and the bald grey-pink head with its hooked beak peeking from the top ❓. 16 × 16, readable at a glance next to
  vanilla eggs.

## 8. Breeding ✅ (v1)

> Shared system: common/SPECS.md §8.5. The vulture gives birth to a live baby (no egg, no incubator) ✅.

- Two tamed vultures fed **rotten flesh** ✅ (scavengers…) go into breeding mode.
- The baby is born tamed, with the same owner, and inherits stats (§2) (and its variant once there are variants, §3).
- The baby is smaller, with a big head and a downy crown. It grows up in 20 min (faster when fed), and can only be ridden
  once grown.
- 5 min cooldown before the parents can breed again.

## 9. v2 (later, kept out of the v1 to-do list) ✅

- Multiplayer: LAN and dedicated server. For a LAN guest it is their own client that moves the mount.
- The skin variants (§3): dark, white, bearded and king, their textures and their spawning by biome; then secondary
  markings.
- Vulture armour.
- User configuration.

## 10. Remaining open questions

- ~~Q26: If phase 0 shows a mod entity cannot be ridden: A, B or C (§1.2)?~~ Moot: phase 0 succeeded without any of the
  three options (2026-11-07).
- ~~Extra stamina cost when carrying someone: +50 %?~~ Moot: no stamina any more (2026-11-09).
- Q27: The flight's numbers (§6.2), the bank and turn limits, and what A / D do in the air.
- Q28: What a passenger in its talons does to its flight (§6.4)?

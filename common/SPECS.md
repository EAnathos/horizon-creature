# Horizon Creature: library specification

The mandatory library of the Horizon Creature series ([docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)): the systems
several creature mods share, and the series' single creative tab. It adds no creature of its own.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Everything below works and was tested in game, single player, unless said otherwise.

- **The library itself**: a content half (`creature.hmod`) and a native half (`creature.hnmod`, crate `mod-creature`)
  that every mod reaches as the `creature` crate (`[dependencies] creature = "^0.1"`).
- **Model drawing** (§3.1), **mob sounds** (§3.2), **the creative tab** (§3.3), **monsters sparing our creatures**
  (§3.4) and **the small fliers' flight** (§3.5).
- **Glass jar and territories** (§4): catching, releasing, settling on a flower patch or in a garden, the jar on
  display (spared by blasts and pistons). The jar left out of the creative tab when no jar mod is installed is built
  but not yet checked in game.
- **Colours and patterns** (§5), used by the butterflies.
- **Mob helpers** (`creature::mob`): `ours` (muted, own box, own goals, spared), `resize`, `Muted`, `despawn_far` (small
  wild creatures far from players, as vanilla's ambient mobs).
- **The item in hand** (`creature::hand`): items whose right-click only the server's hooks handle (the jar, the guide,
  the mods' throwables) skip the off hand's turn, and `use_one` uses one up.
- **Creature guide book and bestiary** (§7).

Limits: single player only (the drawing reads render state from statics, arismoko/horizon#2740); the systems of §6
stay in the vulture until a second mod needs them.

## 1. Role and rules ✅

- Every mod of the series **requires** the library; the library depends on no mod.
- A system belongs here when **two or more mods use it or are specified to use it** (the rule of two), or when it ties
  the series together (the tab). Otherwise it stays in its mod until a second mod needs it.
- The library holds **mechanisms**, the mods hold **species**: no entity type in `creature:`. A mod gives the library its
  species and numbers (registration at start, or parameters).
- A library feature that only makes sense with some mods is **switched on by them** (registration, conditional recipe),
  e.g. the glass jar (§4). The same goes for optional content between mods: a recipe that uses another mod's item when
  it is installed and a vanilla item otherwise is two conditional recipes, never a requirement.
- A system moved here keeps working for the mod it came from, in the same commit.

## 2. Technical context

- **Mod id / namespace**: `creature`. Name: "Horizon Creature".
- **Native half**: a trait mod with an `[api]` crate (`mod-creature`), which the mods reach as the `creature` crate.
  Its `hooks()` runs before theirs (dependency order), so a mod registers its species from its own `hooks()`. It
  publishes functions, types and statics; shared state is behind `Mutex`es.
- **Content half**: the series' tab, shared blocks and items (`creature:glass_jar`, `creature:guide`, the jar blocks), their recipes
  (conditional when a feature depends on a mod), textures and translations (`en_us`, `fr_fr`).
- **Pinned** to the series' Horizon revision (docs/CONVENTIONS.md). Version `0.1.0`; a breaking API change bumps the
  minor version while in `0.x`.

## 3. Systems in place

### 3.1 Model drawing (`creature::model`) ✅

- `Part` / `Cube`: a Blockbench model as data. `cargo creature model <mod> [model]` converts
  `mods/<mod>/assets/models/<model>.bbmodel` into a `pub static <MODEL>: Part` in the mod's crate.
- `draw(host, draw, part, texture, pose)`: draws the tree with the mod's texture; `pose(group)` gives each group's
  animation for the frame. Called from the mod's `mob_submit` hook, which then cancels the template's model.
- Users: vulture; every future mob of the series.

### 3.2 Mob sounds (`creature::sounds`) ✅

- Our mobs copy a vanilla mob whose sounds they must not play, and Horizon prefixes mob sounds with `minecraft:`. The mod
  mutes its mob and the library plays the mod's own sounds with `Host::play_sound`.
- `queue(mob, sound, volume, pitch)` from goals; `tick_post(host, event, voice, source)` from the mod's
  `entity_tick_post`: plays the queue, and the `Voice`'s hurt and death sounds by watching the mob's health.
- Users: vulture; every future mob.

### 3.3 Creative tab ✅

- One tab, "Horizon Creature" (`itemGroup.creature`), generated from the modules' `module.toml`
  (docs/ARCHITECTURE.md §4). The library's items first, then each mod's.
- Icon ✅: the creature guide book (`creature:guide`, §7).
- The glass jar is **hidden when no mod uses it** ✅ (neither Butterflies, Worms nor Fireflies installed): not in the tab,
  not craftable. A tab file can't be conditional today, so how is to prototype ❓: a conditional tab entry if Horizon's
  conditions reach tabs, else a second tab file chosen by the library's native half, else the item registered only when
  a jar user is installed.

### 3.4 Monsters leave our creatures alone (`creature::mob`) ✅

- Our mobs copy the wandering trader, which zombies, husks and illagers hunt. The library's `living_change_target`
  cancels a mob's change of target onto one of our creatures (and the blow its tick dealt with it): `mob::spared`
  (tested with zombies).
- `mob::ours(mob, width, height, eye)` from the mod's `register_goals`: muted, its own box, its template's goals cleared,
  and spared. Users: every mob of the series.

### 3.5 Small fliers (`creature::flutter`) ✅

The flight of butterflies, moths and fireflies: short erratic flights and landing (on flowers), resting, out by day or
by night. A mod gives a `Kind` (day or night, speed, height, flight and rest lengths, drawn to light, landing on
flowers) and calls `register` (size, muted, the flight goal), `tick_post` (territory, despawning far from players)
and `gone`.

## 4. Glass jar and territories ✅

Catching small creatures and settling them in a place they won't leave. Users: **Butterflies**, **Worms**, **Fireflies**
✅. The jar is the library's ✅ and is **active only if one of them is installed** ✅ (hidden otherwise, §3.3).

### 4.1 The jar

- Item `creature:glass_jar`. Recipe ✅ conditional (`neoforge:conditions`, `mod_loaded` butterflies **or** worms **or**
  fireflies): 3 glass in a U + 1 oak slab lid (gives 2 ✅).
- **Catch** ✅: right-click a catchable creature with an empty jar → a **filled jar** holding it (its colours, pattern and
  name are kept).
- **One jar for every creature** ✅: a filled jar is the same `creature:glass_jar`, carrying the creature it holds as
  item data (a registered component, Horizon's `native_component_types`): its kind, colours, pattern and name. No mod
  has a jar item of its own. Its name says what is inside ("Glass Jar of Butterfly" ❓), and its icon shows the
  creature behind the glass: the mod gives a small "in a jar" texture, drawn by the library over the jar's icon ❓
  (`native_item_decorations`, or an item model chosen by the component). Filled jars don't stack ✅.
- Catching and releasing are the library's. A mod registers its catchable kinds:

  ```rust
  creature::jar::register(Catchable { kind: "worms:worm", icon: "worms:item/worm_in_jar", territory: Some(GARDEN) });
  ```

- **Release** ✅: right-click a block with a filled jar → the creature comes out there and the jar is empty again. If the
  creature has a territory rule (§4.2) it **settles** there. A jar whose creature's mod is no longer installed releases
  nothing and stays full.
- **Display** ✅: sneak + right-click places the filled jar as a small block (a creature visibly inside, glowing faintly
  for firefly), broken back into the item. Decorative.

### 4.2 Territories ✅

A settled creature **stays in an area** around one or several blocks, and never despawns ✅. The area is computed from
the block it was released on, by the mod's rule:

| Rule | User | Area |
|---|---|---|
| **Garden** ✅ | Worms | The farmland connected to the release block (4 directions, same level ±1); **at least 10 blocks** ✅, otherwise the worm won't settle |
| **Flower patch** ✅ | Butterflies, Fireflies ❓ | Released on a flower (its anchor): the flowers **within 8 blocks** of the anchor, plus 2 blocks around them and 4 blocks up. The 8-block cap keeps it to at most ~20 × 20 blocks, even in a meadow or a flower forest where every flower touches the next; in a sparse garden it is just the flower beds |

- The creature remembers its anchor (saved on the entity); every few seconds ✅ it checks the area still holds (the
  garden is still ≥ 10 farmland, the flowers are still there). If not, it becomes unsettled (wanders, may despawn ✅).

## 5. Colours and patterns ✅

One species with **several colours and patterns** ✅. Users: **Butterflies**.

- A creature rolls a **base colour**, a **pattern** and a **pattern colour** at spawn (from lists the mod gives, each
  weighted, per biome ✅). Saved on the entity, kept in jars.
- Drawn as two layers: a base texture tinted by the base colour, then the pattern texture tinted by the pattern colour
  (`draw_cube` takes a colour), like vanilla's tropical fish. So 8 colours × 6 patterns × 8 colours need only 7
  textures.

## 6. Systems that could move from the vulture

Each moves here when a second mod needs it. Until then it stays in the vulture.

| # | System | Today |
|---|---|---|
| 6.1 | **Riding and flight**: mount by right-click, **dynamic ridden flight** (steered by the look, banking into arc turns like a plane, vulture/SPECS.md §4.2), dismount, mount hearts (`mount-hud` injection), a saddle | in the vulture |
| 6.3 | **Taming and ownership**: owner on the entity, sit / follow, defend the owner, healing food, only the owner rides | in the vulture |
| 6.5 | **Babies and growth**: smaller model, growth time, faster when fed | in the vulture |
| 6.6 | **Ridden special attack**: a key that makes the mount attack (the vulture's talons) | in the vulture |
| 6.8 | **Hitbox size**: a mod mob's size other than its template's | `creature::mob::resize`, every mob |
| 6.9 | **Sound making tool**: `cargo creature sounds` with a per-mod source list | vulture only |

## 7. Creature guide book ✅

A book about the series' creatures, given to every player.

- **Item** `creature:guide` ("Creature Guide" ❓), unstackable, with **its own texture** ✅ (a worn leather field guide,
  a creature's footprint on the cover ❓).
- **Given at a player's first spawn** in a world ✅ (once per player, remembered on the player; `native_player_logged_in`
  ❓). Not craftable for now ❓ (a book + a feather later, to replace a lost one ❓).
- **For now it does nothing else** ✅: right-click opens nothing (or a cover page ❓).
- **The bestiary** ✅: right-click opens it. On the right, the species met as small
  icons; picking one opens its page on the left: the creature turning in 3D (as Create's Ponder), the biomes it was met
  in, what was found out (how it is tamed, what makes it breed, a jar catches it, it settles on a flower...) and, for
  a species with looks, the morphs met. Nothing about a species not met yet, not even how many there are.
- **Later** (not v1): more pages about each installed mod's creatures, **discovered little by little** ✅ (seeing, catching
  or taming one unlocks its page and then its details): where it lives, what it eats, how it's tamed, and the
  **creature drawn in 3D** in the page ✅ (turning slowly, like Create's
  Ponder scenes). Each mod registers its pages, the library draws the book.

## 8. Out of scope for v1

- Multiplayer (LAN, dedicated server): v2, for the whole series. Server logic is kept apart from rendering.
- A configuration file.
- Compatibility with mods outside the series.

## 9. Open questions

- ~~Q8: Filled jars: one item per creature mod, or one `creature:glass_jar` holding anything?~~ One jar for every
  creature (§4.1).
- ~~Q9: Can a filled jar be placed as a decorative block?~~ Yes, sneak + right-click (§4.1).
- ~~Q10: The jar when no mod uses it?~~ Hidden: not in the tab, not craftable (§3.3).
- Q11: The guide book's name and cover; how it is given (first login hook) and whether it can be crafted again.
- Q5: The tab's order: mods by folder name (now), or a chosen order?

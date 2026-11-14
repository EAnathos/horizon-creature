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
- Users: every mob of the series.

### 3.2 Mob sounds (`creature::sounds`) ✅

- Our mobs copy a vanilla mob whose sounds they must not play, and Horizon prefixes mob sounds with `minecraft:`. The mod
  mutes its mob and the library plays the mod's own sounds with `Host::play_sound`.
- `queue(mob, sound, volume, pitch)` from goals; `tick_post(host, event, voice, source)` from the mod's
  `entity_tick_post`: plays the queue, and the `Voice`'s hurt and death sounds by watching the mob's health.
  `play_queued(host, mob, source)` plays only the queue (a mob without a `Voice`).
- Users: every mob of the series.

### 3.3 Creative tab ✅

- One tab, "Horizon Creature" (`itemGroup.creature`), generated from the modules' `module.toml`
  (docs/ARCHITECTURE.md §4). Order ✅: every spawn egg first, then the library's items, then each mod's other items;
  mods in alphabetical order, each one's items in its own order.
- Icon ✅: the creature guide book (`creature:guide`, §7).
- The glass jar is **hidden when no mod uses it** ✅ (neither Butterflies, Worms nor Fireflies installed): not in the tab,
  not craftable. A tab file can't be conditional, so the library's native half takes it out of the tab
  (`common/native/jar_tab.rs`, an injection into `creative_tabs`); built, still to check in game (§0).

### 3.4 Monsters leave our creatures alone (`creature::mob`) ✅

- Our mobs copy the wandering trader, which zombies, husks and illagers hunt. The library's `living_change_target`
  cancels a mob's change of target onto one of our creatures (and the blow its tick dealt with it): `mob::spared`
  (tested with zombies).
- `mob::ours(mob, width, height, eye)` from the mod's `register_goals`: muted, its own box, its template's goals cleared,
  and spared. Users: every mob of the series.
- `mob::resize(mob, width, height, eye)`: a mod mob's size other than its template's (`ours` calls it), from tiny
  (butterflies, worms) to big (the cyclops).

### 3.5 Small fliers (`creature::flutter`) ✅

The flight of butterflies, moths and fireflies: short erratic flights and landing (on flowers), resting, out by day or
by night. A mod gives a `Kind` (day or night, speed, height, flight and rest lengths, drawn to light, landing on
flowers, an optional take-off sound played at each take-off) and calls `register` (size, muted, the flight goal), `tick_post` (territory, despawning far from players)
and `gone`.

### 3.6 Small crawlers (`creature::crawl`), planned ✅

The slow crawl on the ground of worms and dung beetles: the worm's `Crawl` goal (`worms::Crawl` today), moved here when
the Dung Beetles are built, with what a beetle adds (walking toward a target, backwards). Users: **Worms**, **Dung
Beetles**.

### 3.7 Vanilla hunters eat small creatures (`creature::prey`), planned ✅

A vanilla animal that sees one of its prey within its range goes over and eats it: the creature is gone (no drop),
wild or settled. A mod registers its kinds as prey of a hunter. Not on creatures in a jar, on display or in a bucket.

- **Chickens** peck small creatures within 6 blocks, so a player keeps chickens out of a garden or a pasture. Users:
  **Worms**, **Dung Beetles** (mods/dung_beetles/SPECS.md §2).
- **Sea turtles** swim to jellyfish within 8 blocks and eat them. User: **Jellyfish** (mods/jellyfish/SPECS.md §3).

### 3.8 Feathers (`creature:feathers`) ✅

A mod's own feather works as a feather: the item tag `creature:feathers` lists vanilla's feather and each mod's (as
`required: false` entries, so a missing mod's is left out), and the guide's recipe (§7) takes any of it ✅. Planned:
the recipes for arrows, the brush and the book and quill move here from the Vulture (its `vulture:feather`) when the
**Seagulls** come (mods/seagulls/SPECS.md §5).

### 3.9 Lobbed projectiles (`creature::lob`), planned ✅

A big mob lifts something over its head and hurls it on an arc at a target: the throw goal (range, wind-up, aim on
an arc) and the projectile (damage, a push, what happens where it lands: blocks torn up in a radius, sparing liquids,
containers, placed jars and unbreakable blocks, or none, or a burst of fire). A mod gives its projectile entity and
these values. Users: **Cyclops** (its boulder, `cyclops:boulder`, whose goal and projectile move here) and **Magma
Golem** (planned, its magma glob, mods/magma_golem/SPECS.md §2).

## 4. Glass jar and territories ✅

Catching small creatures and settling them in a place they won't leave. Users: **Butterflies**, **Worms**, **Fireflies**
✅, and **Dung Beetles** and **Fairies** once built (it then joins the recipe's condition and the tab injection). The jar is the library's ✅ and is **active only if one of them is installed** ✅ (hidden otherwise, §3.3).

### 4.1 The jar

- Item `creature:glass_jar`. Recipe ✅ conditional (`neoforge:conditions`, `mod_loaded` butterflies **or** worms **or**
  fireflies): 3 glass in a U + 1 oak slab lid (gives 2 ✅).
- **Catch** ✅: right-click a catchable creature with an empty jar → a **filled jar** holding it (its colours, pattern and
  name are kept).
- **One jar for every creature** ✅: a filled jar is the same `creature:glass_jar`, carrying the creature it holds as
  item data (a registered component, Horizon's `native_component_types`): its kind, colours, pattern and name. No mod
  has a jar item of its own. Its name says what is inside ("Glass Jar of Butterfly" ✅), and its icon shows the
  creature behind the glass: the mod registers the icon of a jar holding it ✅. Filled jars don't stack ✅.
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
| **Flower patch** ✅ | Butterflies, Fireflies ✅ | Released on a flower (its anchor): the flowers **within 8 blocks** of the anchor, plus 2 blocks around them and 4 blocks up. The 8-block cap keeps it to at most ~20 × 20 blocks, even in a meadow or a flower forest where every flower touches the next; in a sparse garden it is just the flower beds |
| **Pasture** ✅ (planned) | Dung Beetles | Released with **at least 3 farm animals** (cows, sheep, horses, donkeys, goats, camels, llamas) **within 8 blocks**: 8 blocks around the release block; fewer than 3 animals left around and the creature is unsettled (mods/dung_beetles/SPECS.md §4) |

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
| 6.1 | **Riding and flight**: mount by right-click, **dynamic ridden flight** (steered by the look, banking into arc turns like a plane, vulture/SPECS.md §6.2), dismount, mount hearts (`mount-hud` injection), a saddle | in the vulture; the flight itself (glide, soaring, landing, the wing poses) moves here with the Seagulls (planned, mods/seagulls/SPECS.md §1), riding stays |
| 6.2 | **Taming and ownership**: owner on the entity, sit / follow, defend the owner, healing food, only the owner rides | in the vulture; moves here with the Sporelins, the Seagulls and the Living Flowers (planned) |
| 6.3 | **Babies and growth**: smaller model, growth time, faster when fed | in the vulture |
| 6.4 | **Ridden special attack**: a key that makes the mount attack (the vulture's talons) | in the vulture |

## 7. Creature guide book ✅

A book about the series' creatures, given to every player.

- **Item** `creature:guide` ("Creature Guide" ✅), unstackable, with **its own texture** ✅ (a worn leather field guide,
  a creature's footprint on the cover ✅).
- **Given at a player's first spawn** in a world ✅ (once per player, remembered on the player; `native_player_logged_in`
  ✅). Craftable again ✅ to replace a lost one: a book, a feather and leather (shapeless).
- **The bestiary** ✅: right-click opens it. On the right, the species met as small
  icons; picking one opens its page on the left: the creature turning in 3D (as Create's Ponder), the biomes it was met
  in, what was found out (how it is tamed, what makes it breed, a jar catches it, it settles on a flower...) and, for
  a species with looks, the morphs met. Nothing about a species not met yet, not even how many there are.
- Each mod registers its species and the facts a player can learn about them (`creature::bestiary::register`); the
  library draws the book. A page opens when a player first meets a wild one, and each fact when they find it out.

- **Morphs**, planned ✅: today the morphs are the colours and patterns of the library's looks (§5). They become any look
  a species gives: a colour and pattern, or a **kind with a model of its own** (the Living Flowers' poppy, dandelion
  and cornflower, the Sporelins' red and brown caps), each shown at the bottom of the page by an icon (a swatch, or the
  kind's item) and drawn by the species when picked.

### 7.1 Tabs, planned ✅

The book gets **tabs**, as ribbons on its edge ✅: **Creatures** (the bestiary above) and **Items**. The book opens on the
tab last read.

- **Creatures**: the series' creatures, then, below a line, the vanilla ones when the Vanilla Bestiary is installed
  (mods/vanilla_bestiary/SPECS.md).
- **Items** (§7.2).

### 7.2 The Items tab, planned ✅

What each item of the series is for, so a player knows how to make it and what to do with it.

- **One page per item** of the series (spawn eggs left out): on the right page the items, as icons; on the left
  the one picked: the item, large, with its name; **how to get it**: its recipe on a 3 × 3 grid of item icons
  (crafting, smithing or smelting, the station drawn beside it), or what drops it (a creature's icon: "Dropped by a
  cyclops"), or what gives it (the dung beetle's ball, the sporelin's stew); and **its uses**, a few lines each with an
  item or creature icon ("Hold it: the farm animals within 12 blocks follow you").
- **Every item shown from the start** ✅ (unlike the creatures): the tab is a reference to plan a craft, so a player can
  look up the shepherd's crook before meeting a cyclops. Only the items of installed mods are listed.
- Each mod registers its items' pages (`creature::bestiary::register_item`): the item, how it is got, its uses as
  language keys with icons. The recipe is read from the game's recipes (`native_recipes`, to check that a trait mod
  reaches it at our pin), else given by the mod and checked against its recipe file by `cargo creature check`.
- Users: the library (the glass jar, the guide), Vulture (feather, meat, saddle use), Butterflies (dust, confetti),
  Cyclops (eye, club, shepherd's crook), and the planned Dung Beetles (dung ball).

## 8. Out of scope for v1

- Multiplayer (LAN, dedicated server): v2, for the whole series. Server logic is kept apart from rendering.
- A configuration file.
- Compatibility with mods outside the series.

## 9. Open questions

- ~~Q12: The Items tab's pages: found little by little, or all shown?~~ All shown from the start (§7.2).

- ~~Q8: Filled jars: one item per creature mod, or one `creature:glass_jar` holding anything?~~ One jar for every
  creature (§4.1).
- ~~Q9: Can a filled jar be placed as a decorative block?~~ Yes, sneak + right-click (§4.1).
- ~~Q10: The jar when no mod uses it?~~ Hidden: not in the tab, not craftable (§3.3).
- ~~Q11: Can a lost guide be crafted again?~~ Yes: a book, a feather and leather (§7).
- ~~Q5: The tab's order?~~ Spawn eggs first, then the library's items, then each mod's, mods in alphabetical order (§3.3).

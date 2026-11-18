# Sunrise Creatures: library specification

The mandatory library of the Sunrise Creatures series ([docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)): the systems
several creature mods share, and the series' single creative tab. It adds no creature of its own.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Everything below works and was tested in game, single player, unless said otherwise.

- **The library itself**: a content half (`creature.hmod`) and a native half (`creature.hnmod`, crate `mod-creature`)
  that every mod reaches as the `creature` crate (`[dependencies] creature = "^0.1"`).
- **Model drawing** (§3.1), **mob sounds** (§3.2), **the creative tab** (§3.3), **monsters sparing our creatures**
  (§3.4), **the small fliers' flight and spawning** (§3.5, by day, at night or at any hour) and **friends** (§6.2, the
  Fairies' and the vulture's owner).
- **The small crawlers' crawl** (§3.6, `creature::crawl`): the worm's ground crawl, now a library goal the next ground
  creatures share; Worms runs on it (tested in game, its behaviour unchanged).
- **Glass jar and territories** (§4): catching, releasing, settling on a flower patch or in a garden, the jar on
  display (spared by blasts and pistons). The jar's absence from the creative tab and recipe when no jar mod is
  installed, and the catch/release/display/name/TNT/piston behavior, are confirmed. The **sand** rule (§4.2, for
  Scorpions) is coded but not yet exercised in game.
- **Colours and patterns** (§5), used by the butterflies.
- **Mob helpers** (`creature::mob`): `ours` (muted, own box, own goals, spared), `resize`, `Muted`, `despawn_far` (small
  wild creatures far from players, as vanilla's ambient mobs).
- **The item in hand** (`creature::hand`): items whose right-click only the server's hooks handle (the jar, the guide,
  the mods' throwables) skip the off hand's turn, and `use_one` uses one up.
- **Creature guide book and bestiary** (§7).

Limits: single player only (the drawing reads render state from statics, kept on the server's side); the systems of §6
stay in the vulture until a second mod needs them, except friends (§6.2), shared with the Fairies.

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
- **Each mod has its row in Options > Mods** ✅ (`creature::list_mod`, from its `mods` hook): without it Horizon lists
  only the mod's content half, which a world must be left to switch (it adds items and creatures), and whose switch
  leaves the mod's code running. The row stands for both halves: switched off, the mod's hooks stop at once, and the
  library hides its pages (`creature::on`: a species' or an item's mod, Minecraft's creatures the Vanilla Bestiary's;
  they come back, as saved, when it is on again); its content follows on the next start. The library itself has no
  such row: every mod needs it.

## 2. Technical context

- **Mod id / namespace**: `creature`. Name: "Sunrise Creatures".
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

- One tab, "Sunrise Creatures" (`itemGroup.creature`), generated from the modules' `module.toml`
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

The flight of butterflies, moths, fireflies and fairies: short erratic flights and landing (on flowers), resting, out
by day, by night or at any hour but in the rain. A mod gives a `Kind` (its `Hours`, speed, height, flight and rest lengths, drawn to light, landing on
flowers, an optional take-off sound played at each take-off) and calls `register` (size, muted, the flight goal), `tick_post` (territory, despawning far from players)
and `gone`; `steer` flies one toward a point (a fairy following its friend).

**Spawning** ✅ (`spawns`): Horizon's natural spawning adds creatures only every 400 ticks, under a cap the farm animals
fill, so small fliers would only appear with new chunks and be gone once their players left. A mod registers a
`Spawning` (kind, biomes, group size, cap near a player, chance); every 10 s, for each player, one chance in its
`chance` while fewer than its `cap` are within 48 blocks, a group comes out 16 to 40 blocks away over the ground, in one
of its biomes, at its hours. Wild ones far from players despawn, so they come and go as bats do. Users: Butterflies
(butterflies, moths), Fireflies, Fairies; planned: **Phasmids**, whose groups are placed **on the top of leaves**
instead of over the ground (mods/phasmids/SPECS.md §3).

### 3.6 Small crawlers (`creature::crawl`) ✅

The slow crawl on the ground of worms, scorpions and the other ground creatures: the worm's `Crawl` goal moved here as
`creature::crawl::Crawl`, driven by a `Cfg` (speed, how long a crawl and a pause last, how far a wild one wanders, how
its settled area is found, a pause hook for the worm's dig). A wild one crawls around where it first crawled; a settled
one crawls only over its area (its mod's territory rule). Users: **Worms** (in place), **Dung Beetles**, **Scorpions**
(mods/scorpions/SPECS.md §3) and **Phasmids** (mods/phasmids/SPECS.md §3). Still to add when their mods are built: a
beetle's walking toward a target (backwards), and **climbing** for phasmids (up the sides of logs and leaves and on
their top, never coming down on its own).

### 3.7 Hunters eat small creatures (`creature::prey`), planned ✅

A hunter that sees one of its prey within its range goes over and eats it: the creature is gone (no drop), wild or
settled. A mod registers its kinds as prey of a hunter, a vanilla mob or **another mod's** (by its entity id): a hunter
whose mod is not installed is skipped, so neither mod requires the other. Not on creatures in a jar, on display or in a
bucket.

- **Chickens** peck small creatures within 6 blocks, so a player keeps chickens out of a garden or a pasture. Users:
  **Worms**, **Dung Beetles** (mods/dung_beetles/SPECS.md §2).
- **Sea turtles** swim to jellyfish within 8 blocks and eat them. User: **Jellyfish** (mods/jellyfish/SPECS.md §3).
- **Dolphins** chase flying fish within 12 blocks and eat them; the fish flee by gliding. User: **Flying Fish**
  (mods/flying_fish/SPECS.md §3).
- **Seagulls** (a mod's hunter) swoop on gliding flying fish within 16 blocks and snatch them in the air, when both
  mods are installed. Users: **Flying Fish** and **Seagulls** (mods/flying_fish/SPECS.md §3).

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

### 3.10 Water creatures (`creature::aquatic`), planned ✅

The life in water of jellyfish and flying fish, which copy the land-living wandering trader: breathing in water and not
in air (Horizon's `native_living_breathe`), no gravity in water, **beached** out of it (it can't swim, flops now and
then, dries out after the mod's delay and dies, dropping as usual), and **the bucket**: a water bucket used on one
catches it as vanilla's fish, into the mod's bucket item, keeping its kind and name, and lets it out where the bucket is
emptied; a caught one never despawns. A mod gives its kinds, its bucket item and its drying delay. Users: **Jellyfish**
(mods/jellyfish/SPECS.md §3, §5) and **Flying Fish** (mods/flying_fish/SPECS.md §3, §5).

## 4. Glass jar and territories ✅

Catching small creatures and settling them in a place they won't leave. Users: **Butterflies**, **Worms**, **Fireflies**
✅, **Fairies** ✅, and **Dung Beetles**, **Phasmids** and **Scorpions** once built (they then join the recipe's
condition and the tab injection). The jar is the library's ✅ and is **active only if one of them is installed** ✅
(hidden otherwise, §3.3).

### 4.1 The jar

- Item `creature:glass_jar`. Recipe ✅ conditional (`neoforge:conditions`, `mod_loaded` butterflies **or** worms **or**
  fireflies **or** fairies): 3 glass in a U + 1 oak slab lid (gives 2 ✅).
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
  for a firefly or a fairy), broken back into the item. Decorative.
- **Befriended and named creatures** ✅: a befriended one (§6.2) is only caught by its friend, and the jar keeps its
  friend ("Befriended" under its name): let out, it is still theirs. A named one keeps its name ("Name: ..." under the
  jar's), restored when released.

### 4.2 Territories ✅

A settled creature **stays in an area** around one or several blocks, and never despawns ✅. The area is computed from
the block it was released on, by the mod's rule:

| Rule | User | Area |
|---|---|---|
| **Garden** ✅ | Worms | The farmland connected to the release block (4 directions, same level ±1); **at least 10 blocks** ✅, otherwise the worm won't settle |
| **Flower patch** ✅ | Butterflies, Fireflies ✅ | Released on a flower (its anchor): the flowers **within 8 blocks** of the anchor, plus 2 blocks around them and 4 blocks up. The 8-block cap keeps it to at most ~20 × 20 blocks, even in a meadow or a flower forest where every flower touches the next; in a sparse garden it is just the flower beds |
| **Tree** ✅ (planned) | Phasmids | Released on leaves or a log: the leaves and logs connected to it within 8 blocks, plus a block around them; **at least 10 leaves** ✅, otherwise it won't settle (mods/phasmids/SPECS.md §5) |
| **Sand** ✅ (rule in place, its mod to come) | Scorpions | Released on sand, red sand, sandstone or terracotta: those blocks connected to it within 8 blocks; **at least 10** ✅, otherwise it won't settle (mods/scorpions/SPECS.md §6) |
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
| 6.1 | **Riding and flight**: mount by right-click, **dynamic ridden flight** (steered by the look, banking into arc turns like a plane, vulture/SPECS.md §6.2), dismount, mount hearts (vanilla's on Sunrise; the `mount-hud` injection on Horizon), a saddle | in the vulture; the flight itself (glide, soaring, landing, the wing poses) moves here with the Seagulls (planned, mods/seagulls/SPECS.md §1), riding stays |
| 6.2 | **Taming and ownership**: owner on the entity, sit / follow, defend the owner, healing food, only the owner rides | **friends** ✅ in the library (`creature::friend`: the friend's UUID and staying, saved on the creature as `creature:friend`, kept in the glass jar), used by the **Fairies** and the **Vulture** (its owner and sitting; a save from before, which held them in `vulture:state`, is moved over once, tested in game); defending, healing food and riding come with the Sporelins, the Seagulls and the Living Flowers (planned) |
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
- **For Minecraft's own creatures** ✅ (the Vanilla Bestiary, mods/vanilla_bestiary/SPECS.md): a species with no model
  of ours draws its icon, large and turning (`bestiary::draw_icon`); a species' biomes may be a whole dimension
  (`@minecraft:the_nether`); a fact with no word is a drop, its item alone, on its own lines below the other facts under
  the heading "Drops", and the facts wrap onto as many lines as they need; `bestiary::players_near` gives the players
  who saw something happen. The right page lists the series' species, then, below a line, Minecraft's own
  (`minecraft:` ids), and the wheel scrolls it when they don't fit.
- **Operators' commands** ✅ (level 2), to check the pages: `/bestiary unlock <species>` (its id, or the part after the
  colon when one species has it: `cow`) writes down everything about it for the player who runs it: every biome it
  spawns in, every fact, every look its varieties can roll; `/bestiary unlock all` does it for every species;
  `/bestiary reset` forgets everything. The chat completes the species' ids.
- **Fixed** (was: after `/bestiary unlock all` the Creature Guide no longer opened): the book's data (every species,
  biome, fact and morph the player has met) is one custom payload, which overflowed Horizon's `u16`/32767-byte limit
  once all 92 species were unlocked and was silently dropped, so the book never opened. It is now sent in chunks under
  the limit, split on species boundaries, and the client opens on the last chunk (`book.rs`, `send_chunked`). Tested in
  game: `/bestiary unlock all` then the guide opens, and `/bestiary reset` then it opens.

- **Morphs**, planned ✅: today the morphs are the colours and patterns of the library's looks (§5). They become any look
  a species gives: a colour and pattern, or a **kind with a model of its own** (the Living Flowers' poppy, dandelion
  and cornflower, the Sporelins' red and brown caps), each shown at the bottom of the page by an icon (a swatch, or the
  kind's item) and drawn by the species when picked.

### 7.1 Tabs ✅

The book has **tabs**, as ribbons above its right page ✅ (the open one in the paper's colour): **Creatures** (the
bestiary above) and **Items**. The book opens on the tab last read (while the game runs).

- **Creatures**: the series' creatures, then, below a line, the vanilla ones when the Vanilla Bestiary is installed
  (mods/vanilla_bestiary/SPECS.md).
- **Items** (§7.2).
- **Scrolling** ✅: the wheel scrolls the list it is over, a row at a time, with a thin scrollbar when there is more than
  shown: the right page's icons, a species' morph swatches, and the left page's details (a species' biomes, facts and
  drops; an item's recipes and uses), so a long page (a butterfly met in many biomes) is read whole. A species with
  looks has a shorter 3D view, for its details to have room above its swatches.

### 7.2 The Items tab ✅

What each item of the series is for, so a player knows how to make it and what to do with it.

- **One page per item** of the series (spawn eggs left out) ✅: on the right page the items, as icons; on the left
  the one picked: the item turning in the 3D view, with its name; **how to get it**: its recipes on a 3 × 3 grid of
  item icons (crafting, smelting, smoking, campfire cooking, brewing; the stations drawn beside it, one grid for the
  same recipe in several stations, a slot that takes several items, a tag's, showing each in turn), what drops it (a
  creature's icon: "Dropped by a cyclops"), or what gives it (a befriended fairy's dust); **its uses**, a few lines
  each with an icon ("Held: the farm animals within 12 blocks follow you"); and what it is **used to make** (the
  results of the recipes it goes into: a vulture feather makes arrows, the brush, the book and quill, the guide).
- **Every item shown from the start** ✅ (unlike the creatures): the tab is a reference to plan a craft, so a player can
  look up the shepherd's crook before meeting a cyclops. Only the items of installed mods are listed.
- Each mod registers its items' pages (`creature::bestiary::register_item`, `creature::items::ItemPage`) ✅: the item,
  the creatures that drop it, what else gives it and its uses, as language keys with icons. The glass jar's page shows
  only with a mod that uses the jar.
- **The recipes are the modules' own files** ✅: Horizon gives a trait mod only custom recipe types at our pin, so
  `cargo creature tab` reads every module's `data/<id>/recipe/` into the library's `recipes.rs` (the series' tags
  expanded, Minecraft's drawn by an item of theirs), and `cargo creature check` fails when it is out of date. A page
  can't show a recipe the game doesn't have.
- Users: the library (the glass jar, the guide), Vulture (feather, raw and cooked meat), Butterflies (dust, confetti),
  Cyclops (eye, club, shepherd's crook), Fairies (fairy dust), and the planned Dung Beetles (dung ball).

## 8. Out of scope for v1

- Multiplayer (LAN, dedicated server): v2, for the whole series. Server logic is kept apart from rendering.
- A configuration file.
- Compatibility with mods outside the series.

## 9. Testing in game

Creative, a test world; `cargo creature install` first.

- The creative tab "Sunrise Creatures": every installed mod's items, spawn eggs first; the glass jar only with a jar mod.
- `/bestiary unlock all`: every installed species' page, full; then open the Creature Guide (regression for the known
  bug in §7). `/bestiary reset` empties the book and it still opens. The wheel scrolls a long page (a butterfly's) and
  the icons; the Items tab lists every installed mod's items, each with its recipe.
- A new world: the creature guide given once; its bestiary pages turn in 3D, list the biomes met, the facts and the
  rarity; a craft of the guide with a vulture feather.
- Monsters (a zombie, a skeleton) ignore a butterfly, a worm, a vulture.
- The glass jar, confirmed: it is hidden when no jar mod is installed; catch, release, display and break it; names are
  kept; TNT and pistons leave placed jars alone; flower settling works. Scorpions' sand settling rule still needs an
  in-game check.
- The recipe book lists the series' recipes once one ingredient is held.
- Small fliers come out near a player within a minute or two, in their biomes and hours.

## 10. Open questions

- ~~Q12: The Items tab's pages: found little by little, or all shown?~~ All shown from the start (§7.2).

- ~~Q8: Filled jars: one item per creature mod, or one `creature:glass_jar` holding anything?~~ One jar for every
  creature (§4.1).
- ~~Q9: Can a filled jar be placed as a decorative block?~~ Yes, sneak + right-click (§4.1).
- ~~Q10: The jar when no mod uses it?~~ Hidden: not in the tab, not craftable (§3.3).
- ~~Q11: Can a lost guide be crafted again?~~ Yes: a book, a feather and leather (§7).
- ~~Q5: The tab's order?~~ Spawn eggs first, then the library's items, then each mod's, mods in alphabetical order (§3.3).

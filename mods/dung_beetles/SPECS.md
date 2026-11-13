# Dung Beetles: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): small beetles that follow
grazing herds, roll balls of dung in straight lines, steered by the stars, and bury them, turning worn ground back into
grass. Simple, like Worms ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the model, no code yet.

- **The model** (`assets/models/dung_beetle.bbmodel`, pictures in docs/MODELS.md), after a photo of a dung beetle at
  work: a domed, matte black-brown beetle (the wing cases and the pronotum rounded from stacked cubes, faint straw
  specks, reddish hairs underneath), standing high and head down (the body tilted 12°), a flat semicircular head shield
  and short antennae. Six thin two-part legs: the front ones short and planted under the head, the middle ones long and
  swept back, the hind ones reaching back onto the ball. The ball behind it, about as big as the beetle (7 texels,
  three crossed cubes, `ball`), and a small one for while it shapes it (`ball_small`); the game shows one ball or none
  (`draw_shown`) and poses it all.

Next: the code once Horizon's launcher builds the pinned revision again (docs/HORIZON-ISSUES.md, #15764). The library's side comes with it: the small
crawlers and the chickens' pecking (common/SPECS.md §3.6, §3.7), and the pasture rule (§4.2).

## 1. Technical context

- **Mod id**: `dung_beetles` ✅. Entity `dung_beetles:dung_beetle`, spawn egg `dung_beetles:dung_beetle_spawn_egg`, item
  `dung_beetles:dung_ball` ✅ (§5). No jar item of its own: a caught beetle is in the library's `creature:glass_jar` ✅
  (library §4.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4, tiny), **small crawlers** (§3.6),
  **chickens' pecking** (§3.7), the **glass jar and territories** (§4, which this mod switches on; rule "pasture"),
  the creature guide's pages (§7).
- The beetle copies the wandering trader (goals only) with a tiny hitbox, like the worm.
- **The ball is drawn with the beetle** ✅: a part of its model, growing as it is shaped and turning as it rolls, not an
  entity of its own (no physics, nothing to save apart, no stray balls left in the world).
- The dung ball in a composter: through Horizon's compostables data map if a content mod can add to it, else the
  library's block interaction hook (to check when implementing).

## 2. The dung beetle ✅

- A small beetle, about 5 texels wide and 10 long: a domed, matte black-brown shell, a flat semicircular head shield,
  long thin legs, the hind ones the longest. Ambient and harmless, 1 health, **no drop** ✅.
- **Spawning** ✅: savannas, savanna plateaus, plains, sunflower plains, meadows and the edges of deserts (on grass,
  coarse dirt or sand next to grass), 1 to 3 together, by day and at night. Wild ones despawn far from players.
- **Follows the herds** ✅: it is drawn to grazing animals (cows, sheep, horses, donkeys, goats, camels, llamas) within
  16 blocks; with none around it just crawls about (the library's small crawlers).
- **Chickens peck it** ✅ (library §3.7): a chicken that sees one walks over and eats it. Keep chickens out of the
  pasture.

### 2.1 Rolling the ball ✅

The beetle's signature, the thing a player stops to watch.

- Near a grazing animal, now and then (about once every 2 minutes per beetle), it **shapes a ball** where the animal
  stood: a few seconds of digging, the ball growing in front of it.
- Then it **rolls it away in a straight line**, walking backwards, pushing with its hind legs, the ball turning, for 8 to
  16 blocks. It climbs a one-block step and turns aside only when stuck.
- **Steered by the stars** ✅ (real dung beetles follow the Milky Way): on a clear night its lines are perfectly
  straight; under clouds, rain or a roof they wander.
- At the end it **buries the ball** in the block below (dirt, grass, coarse dirt, podzol or rooted dirt) and
  fertilizes it (§3).
- **Taking its ball** ✅: right-click a beetle with an empty hand while it rolls one, and you get a dung ball (§5); the
  beetle stops, looks around, and starts a new one later. (A glass jar in hand catches the beetle instead, §4.)

## 3. What the buried ball does ✅

Grazing land kept alive: the beetles slowly repair what the herds wear down.

- **One block per ball** ✅, the block it is buried in: **coarse dirt** or rooted dirt becomes dirt; **dirt** becomes
  grass; on **grass**, short grass and a flower or two grow around it, as bone meal does (the biome's flowers).
- A ball every couple of minutes per beetle: slow, so it is a quiet effect on a pasture, not a farm machine.

## 4. Catching and settling ✅

- **Caught in the glass jar** ✅ (library §4.1), like butterflies, fireflies and worms. Dung Beetles is a jar user: the
  jar's recipe condition and the creative tab injection list it too (library §3.3, §4.1).
- **Released in a pasture**, it settles there ✅ (library §4.2, rule **"pasture"**): released with at least **3 farm
  animals within 8 blocks**, it roams 8 blocks around the release block, follows the herd there, rolls its balls there
  and never despawns. With fewer than 3 animals left around, it becomes unsettled (wanders, may despawn).

## 5. The dung ball ✅

`dung_beetles:dung_ball`, taken from a rolling beetle (§2.1). Stacks to 64.

- **A fertilizer for the ground** ✅: used on dirt, coarse dirt, rooted dirt or grass, it does what a buried ball does
  (§3) to that block, with bone meal's green sparkles. Nothing on crops or saplings (that is bone meal's job, and the
  worms'): a way to turn worn, coarse ground back into grass by hand.
- **In a composter** ✅, it always adds a level.

## 6. Bestiary page ✅

The biomes met in, and the facts: "Catch", "Ball" (seen shaping or rolling one), "Buried" (a ball buried and the ground
turned greener), "Stars" (a perfectly straight roll on a clear night), "Pasture" (settled).

## 7. Out of scope for v1

- Other beetle species, flying, breeding, taming, riding ✅. No drop when killed, no shell or charm ✅.

## 8. Open questions

- ~~Q1: The mod's name?~~ Dung Beetles (`dung_beetles`).
- ~~Q2: A jar user, and how it settles?~~ Yes, in a pasture: at least 3 farm animals within 8 blocks (§4).
- ~~Q3: The ball, drawn or an entity?~~ Drawn with the beetle (§1).
- ~~Q4: Which items?~~ The dung ball only (§5); chickens peck beetles and worms (§2).
- ~~Q5: How strong the buried ball is?~~ One block per ball (§3), to balance in game.

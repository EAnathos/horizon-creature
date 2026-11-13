# Worms: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): small earthworms you find on
the dirt, carry in a glass jar and settle in your garden, where they stay and slowly make crops grow faster. Simple,
like Butterflies ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player).

- **The worm** (`worms:worm`, spawn egg): six segments (`assets/models/worm.bbmodel`), a wave running along it as it
  crawls, slowly, a few blocks around where it first crawled, with pauses.
- **Spawning**: plains, meadows, forests and taigas, 1 or 2 together; while it rains, more come up on grass and dirt
  near players (a chance every 5 s, up to 6 within 24 blocks). Wild ones despawn far from players.
- **Gardens** (§4): caught in the glass jar and released on farmland (or a crop on it) of at least 10 connected
  farmland blocks, it settles and crawls only there; every 20 s it makes one crop of its garden grow a stage, with
  green sparkles; more than 3 worms in one garden count as 3. A garden shrunk below 10 frees it.
- **Bestiary page**: the biomes met in, "Catch", "Garden".

Left: its own sounds (picked from freesound, not made yet).

## 1. Technical context

- **Mod id**: `worms`. Entity `worms:worm`. No jar item of its own: a caught worm is in the library's
  `creature:glass_jar`, the same jar as the butterflies' ✅ (library §4.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **glass jar and territories** (§4, which this mod
  switches on; rule "garden"), hitbox size (§3.4).
- The worm copies the wandering trader (goals only) with a tiny hitbox. Crop growth: Horizon's random tick hook
  (`native_random_tick`) or a bonus growth tick from the worm itself ✅ (the worm's own tick).

## 2. The worm ✅

- Small ambient earthworms: harmless, slow, crawling.
- **Found in nature, on the dirt** ✅: on dirt, grass, podzol, coarse dirt and rooted dirt ✅, in groups of 1–2, more after rain
  ✅ (while it rains, one chance in 3 every 5 s for each player, 4 to 16 blocks away, up to 6 worms within 24 blocks); wild ones despawn far from players ✅.
- Pink-brown, segmented model, crawling animation; sounds to cut from freesound (§0).

## 3. Jar and garden ✅

- **Picked up in a glass jar** ✅ (library §4.1), the same one butterflies and fireflies go in.
- **Released in a garden** ✅, it **stays there** ✅ (library §4.2, rule "garden"): the farmland connected to the release
  block must count **at least 10 blocks** ✅, otherwise it won't settle ✅ (it stays unsettled and may despawn).
- Settled worms wander only on that farmland (and the crops on it), and never despawn.

## 4. Faster crop growth ✅

- A settled worm makes the crops of its garden grow **faster, with a slow bonus** ✅, as built: every 20 s it makes one
  crop of its garden grow a stage; more than 3 worms in one garden count as 3 ✅ (so a big farm needs worms in several
  gardens).
- Faint particles on a crop that got a worm's bonus ✅ (green sparkles, as bone meal).

## 5. Out of scope for v1

- Giant worms ✅, taming, combat, riding, fishing bait and animal food ✅, breeding ✅.

## 6. Open questions

- ~~Q1: What a worm does?~~ Faster crop growth, a slow bonus.
- ~~Q2: The garden?~~ At least 10 connected farmland.
- ~~Q3 / Q4: Finding and catching?~~ In nature on the dirt; in a glass jar.
- ~~Q5: Bait / chicken food?~~ No.
- ~~Q6: Multiplying?~~ No: worms don't multiply.
- ~~Q7: The bonus?~~ One crop a stage every 20 s per worm, at most 3 worms counting per garden.

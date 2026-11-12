# Worms: specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): small earthworms you find on
the dirt, carry in a glass jar and settle in your garden, where they stay and slowly make crops grow faster. Simple,
like Butterflies ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-09)

- Folder and spec created.
- **In game** (2026-11-12, tested): `worms:worm` and its spawn egg; model `assets/models/worm.bbmodel` (six segments, a
  lighter band, each segment a group: a wave runs along it as it crawls). It crawls slowly (0.015 block a tick) a few
  blocks around where it first crawled, with pauses. Caught in the glass jar (its own jar icon); released on farmland
  or a crop on it, in a garden of at least 10 connected farmland blocks (the library's garden rule), it settles: it
  crawls only on that farmland (tested) and, every 20 s, makes one crop of its garden grow a stage (tested: 4 wheat
  stages in about a minute with random ticks off); more than 3 worms in one garden count as 3. A garden shrunk below
  10 frees it. Wild ones despawn far from players, settled ones never. Spawns in plains, meadows, forests and taigas,
  1 or 2 together. Bestiary page (Catch, Garden). Not yet: more worms after rain, the farmland kept moist, particles
  on a crop it helped.

## 1. Technical context

- **Mod id**: `worms`. Entity `worms:worm`. No jar item of its own: a caught worm is in the library's
  `creature:glass_jar`, the same jar as the butterflies' ✅ (library §6.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **glass jar and territories** (§6, which this mod
  switches on; rule "garden"), hitbox size (§8.8).
- The worm copies the wandering trader (goals only) with a tiny hitbox. Crop growth: Horizon's random tick hook
  (`native_random_tick`) or a bonus growth tick from the worm itself ❓.

## 2. The worm ✅

- Small ambient earthworms: harmless, slow, crawling.
- **Found in nature, on the dirt** ✅: on dirt, grass, coarse dirt and rooted dirt ❓, in groups of 1–2, more after rain
  ❓; they don't despawn while in sight ❓.
- Pink-brown, segmented model, crawling animation; no sound ❓.

## 3. Jar and garden ✅

- **Picked up in a glass jar** ✅ (library §6.1), the same one butterflies and fireflies go in.
- **Released in a garden** ✅, it **stays there** ✅ (library §6.2, rule "garden"): the farmland connected to the release
  block must count **at least 10 blocks** ✅, otherwise it won't settle ❓ (it burrows away / stays unsettled).
- Settled worms wander only on that farmland (and the crops on it), and never despawn.

## 4. Faster crop growth ✅

- A settled worm makes the crops of its garden grow **faster, with a slow bonus** ✅. Proposal ❓:
  - each worm gives the crops of its garden an extra growth chance equal to **+10 %** of the normal rate ✅;
  - bonuses add up to **+30 %** per garden ✅ (3 worms), so a big farm needs worms in several gardens;
  - the farmland it crawls on stays moist ❓.
- Faint particles on a crop that got a worm's bonus ❓.

## 5. Out of scope for v1

- Giant worms ✅, taming, combat, riding, fishing bait and animal food ✅, breeding ✅.

## 6. Open questions

- ~~Q1: What a worm does?~~ Faster crop growth, a slow bonus.
- ~~Q2: The garden?~~ At least 10 connected farmland.
- ~~Q3 / Q4: Finding and catching?~~ In nature on the dirt; in a glass jar.
- ~~Q5: Bait / chicken food?~~ No.
- ~~Q6: Multiplying?~~ No: worms don't multiply.
- ~~Q7: The bonus?~~ +10 % per worm, capped at +30 % per garden.

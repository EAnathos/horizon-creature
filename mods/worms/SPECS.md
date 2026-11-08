# Worms — specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): small earthworms you find on
the dirt, carry in a glass jar and settle in your garden, where they stay and slowly make crops grow faster. Simple,
like Butterflies ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-08)

- Folder and spec created. **Not started**.

## 1. Technical context

- **Mod id**: `worms`. Entity `worms:worm`, item `worms:worm_jar` ❓ (a jar holding one).
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

- **Picked up in a glass jar** ✅ (library §6.1).
- **Released in a garden** ✅, it **stays there** ✅ (library §6.2, rule "garden"): the farmland connected to the release
  block must count **at least 10 blocks** ✅, otherwise it won't settle ❓ (it burrows away / stays unsettled).
- Settled worms wander only on that farmland (and the crops on it), and never despawn.

## 4. Faster crop growth ✅

- A settled worm makes the crops of its garden grow **faster, with a slow bonus** ✅. Proposal ❓:
  - each worm gives the crops of its garden an extra growth chance equal to **+10 %** of the normal rate;
  - bonuses add up to **+30 %** per garden ❓ (3 worms), so a big farm needs worms in several gardens;
  - the farmland it crawls on stays moist ❓.
- Faint particles on a crop that got a worm's bonus ❓.

## 5. Out of scope for v1

- Giant worms ✅, taming, combat, riding.

## 6. Open questions

- ~~Q1 — What a worm does?~~ Faster crop growth, a slow bonus.
- ~~Q2 — The garden?~~ At least 10 connected farmland.
- ~~Q3 / Q4 — Finding and catching?~~ In nature on the dirt; in a glass jar.
- Q5 — Fishing bait / chicken food / compost: wanted?
- Q6 — Can worms multiply in a garden?
- Q7 — The bonus: +10 % per worm, capped at +30 % per garden?

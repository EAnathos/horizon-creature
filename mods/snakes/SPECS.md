# Snakes — specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): an aggressive snake in many
colours and patterns, whose eggs you hatch in the library's incubator to raise a named pet snake ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-08)

- Folder and spec created. **Not started**.

## 1. Technical context

- **Mod id**: `snakes`. Entity `snakes:snake`, item `snakes:snake_egg` ✅.
- Uses from the library: model drawing and sounds (§3.1, §3.2), **incubator** (§4), **naming on hatch** (§5), **colours
  and patterns** (§7), taming (§8.3), babies and growth (§8.5), hitbox size (§8.8).
- A wild snake is hostile: it copies the zombie ❓ (hostile goals) or the trader with our own target goals ❓. A long,
  low body: a segmented model animated in a wave (one group per segment).

## 2. The snake ✅

- **One species** ✅ with **several colours and patterns** ✅ (library §7). Proposal ❓:
  - base colours: green, olive, brown, sand, grey, black, red, yellow;
  - patterns: plain, stripes, bands, diamonds (viper zig-zag), spots, rings;
  - biome weights: sand/bands in deserts, green in jungles and swamps, grey/diamonds in mountains ❓.
- **Aggressive** ✅: attacks players and small animals on sight within ~8 blocks ❓. Bite: 3 damage + **poison** (5 s ❓).
  Hisses and rears up before striking ❓. Health 12 ❓. Swims.
- Spawning ❓: deserts, badlands, jungles, swamps, savannas; 1–2 together; also in caves ❓.
- Drops ❓: 0–1 snake skin (leather substitute ❓).

## 3. Eggs and hatching ✅

- Snakes **drop eggs** ✅: proposal 25 % on death ❓. The egg keeps the snake's colours and pattern (library §4.3) ❓.
- The egg hatches only in the **incubator** ✅ at **Warm** ❓ (level 2), in ~10 min ❓; the wrong temperature cracks it.
- When it's ready, **its owner has to name it, then it hatches** ✅ (library §5).
- The hatchling is a **pet** ✅, tamed to the player who put the egg in ✅, bearing its name.

## 4. The pet snake ❓

- A baby that grows up in 20 min ❓.
- Follows its owner, sits on command (right-click) ❓, never attacks its owner; attacks what attacks its owner, like a
  wolf ❓ (with its poison bite).
- Can ride on its owner's shoulder ❓ (like a parrot).
- Heals with raw meat / eggs ❓.

## 5. Out of scope for v1

- Riding, breeding pets together ❓.

## 6. Open questions

- ~~Q1 / Q2 — Species and attitude?~~ One species, several colours and patterns, aggressive.
- ~~Q3 / Q4 — Eggs and hatched snakes?~~ Dropped by snakes, hatched (named first) into a pet.
- Q5 — Egg drop chance; incubation temperature and time.
- Q6 — What the pet does: follow, sit, defend, shoulder?
- Q7 — Can two pets breed (laying an egg)?

# Fireflies: specification (v0.1, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): **fireflies** that light up
the night, that you catch in the glass jar and settle in your garden, or keep in a jar as a soft light ✅. Small in
scope, like Butterflies ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-09)

- Folder and spec created. **Not started**.

## 1. Technical context

- **Mod id**: `fireflies`. Entity `fireflies:firefly`, item `fireflies:firefly_spawn_egg`. No jar item of its own: a
  caught firefly is in the library's `creature:glass_jar` ✅ (library §6.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **glass jar and territories** (§6, which this mod
  switches on), hitbox size (§8.8, tiny), the creature guide's pages later (§9).
- The firefly copies the wandering trader (goals only) with a tiny hitbox. Its glow: drawn full-bright (an emissive
  layer ❓); it gives no light to the world while flying ❓.

## 2. The firefly ❓

- A tiny beetle (a few pixels) whose abdomen **glows yellow-green**, pulsing: lit ~1 s, dark ~2 s, each one on its own
  rhythm ❓; groups slowly fall into step ❓.
- **Night only** ✅ ❓: they come out at dusk, drift slowly and wander in loose swarms of 4 to 10 ❓, low over the grass;
  at dawn they settle on the underside of leaves and grass (dark, no flight), and wild ones may despawn.
- **Spawning** ❓: warm nights, no rain, near water and tall grass: swamps, mangroves, rivers' banks, plains, meadows,
  forests' edges. Not in deserts or cold biomes.
- Ambient and harmless, 1 HP, no drop ❓. Flees a sprinting player ❓.
- Sound: none, or a very faint buzz ❓.

## 3. Jar and settling ✅

- **Caught in the glass jar** ✅ (library §6.1), the same one as butterflies and worms.
- **Released** on a flower, it **settles** there ❓ (library §6.2, rule "flower patch", like butterflies), and like a
  moth a settled firefly only moves **at night** ❓.
- **A jar of fireflies as a light** ❓: placed as a decorative jar (library §6.1, "display"), it glows (light level 7 ❓,
  pulsing ❓); a lantern alternative for gardens and paths.

## 4. Out of scope for v1

- Larvae, glowworms in caves, colours other than yellow-green.

## 5. Open questions

- Q1: Settled fireflies: the "flower patch" rule, or a rule of their own (tall grass and water ❓)?
- Q2: The jar's light level, and whether it pulses.
- Q3: Do fireflies give any light while flying (dynamic light is not in Horizon ❓)?

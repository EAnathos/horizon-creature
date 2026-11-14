# Fireflies: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): **fireflies** that light up
the night, that you catch in the glass jar and settle in your garden, or keep in a jar as a soft light ✅. Small in
scope, like Butterflies ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player).

- **The firefly** (`fireflies:firefly`, spawn egg): a small beetle drawn at half size from
  `assets/models/firefly.bbmodel`. It flies with the library's small fliers (common/SPECS.md §3.5): at night, slow,
  low, long drifts; by day it rests on the ground.
- **Its lantern** pulses on its own rhythm (lit about a second out of three): a full-bright texture (`emissive`) and a
  soft yellow-green light around up to 8 lit fireflies (`visuals`, lighting nothing in the world).
- **Spawning**: swamps, mangroves, rivers, plains, meadows, forests, birch and flower forests, 3 to 6 together.
- **Glass jar** (common/SPECS.md §4): caught and released; released on a flower it settles there; a jar of fireflies on
  display gives light 7.
- **Bestiary page**: the biomes met in, "Catch", "Settles".

- **Sounds**: a faint buzz when one takes off, from CC0 freesound recordings (`assets/sounds.toml`).

## 1. Technical context

- **Mod id**: `fireflies`. Entity `fireflies:firefly`, item `fireflies:firefly_spawn_egg`. No jar item of its own: a
  caught firefly is in the library's `creature:glass_jar` ✅ (library §4.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **glass jar and territories** (§4, which this mod
  switches on), hitbox size (§3.4, tiny), the creature guide's pages (§7).
- The firefly copies the wandering trader (goals only) with a tiny hitbox. Its glow: drawn full-bright (an emissive
  layer ✅); it gives no light to the world while flying ✅.

## 2. The firefly ✅

- A tiny beetle (a few pixels) whose abdomen **glows yellow-green**, pulsing: lit ~1 s, dark ~2 s, each one on its own
  rhythm ✅.
- **Night only** ✅: they drift slowly, low over the grass, in groups of 3 to 6 ✅; by day they rest on the ground, and
  wild ones despawn far from players.
- **Spawning** ✅: swamps, mangroves, rivers, plains, meadows, forests, birch and flower forests. Not in deserts or cold
  biomes.
- Ambient and harmless, 1 HP, no drop ✅. Flees a sprinting player ✅.
- Sound ✅: a very faint wing buzz, still to cut from freesound (§0).

## 3. Jar and settling ✅

- **Caught in the glass jar** ✅ (library §4.1), the same one as butterflies and worms.
- **Released** on a flower, it **settles** there ✅ (library §4.2, rule "flower patch", like butterflies), and like a
  moth a settled firefly only moves **at night** ✅.
- **A jar of fireflies as a light** ✅: placed as a decorative jar (library §4.1, "display"), it glows (light level 7,
  steady); a lantern alternative for gardens and paths.

## 4. Out of scope for v1

- Larvae, glowworms in caves, colours other than yellow-green.

## 5. Open questions

- ~~Q1: Settled fireflies?~~ The flower patch rule, as butterflies.
- ~~Q2: The jar's light?~~ Light 7, steady.
- ~~Q3: Light while flying?~~ No world light: a full-bright lantern and Horizon's visual light (seen, lights nothing).

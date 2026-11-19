# Storm Spirits: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): spirits of living
electricity, about a player's height, born now and then where a storm's lightning strikes, in any biome. They crackle, zip about and shock what
hits them; what they leave behind calls lightning down.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable, tested in game (single player): everything below works, unless marked ❓.

- **The storm spirit** (`storm_spirits:storm_spirit`, spawn egg): drawn with its own model
  (`sources/models/storm_spirit.bbmodel`, pictures in docs/models/), full-bright, its arms swaying, its tail of arcs
  flickering, its four sparks circling it and blinking, a soft blue light around it.
- **Born of lightning** (§2): one weather strike in 10; tested with the spark's bolts and `/summon`, which never give
  birth. **Lives with the storm**: it stays through a thunderstorm and fades 1 to 3 minutes after it.
- **Flight**, the **shock** (its dash, the zap, keeping away) and the zap of a touch (§2); immune to lightning and fire.
- **The spark** (§4): the bolt where you aim, used up, its cooldown.
- **Sounds** ❓: vanilla's for now (a breeze's hurt and death, a bee's buzz, a firework's twinkle, a lightning bolt's
  impact for the zap), until its own from freesound.

## 1. Technical context

- **Mod id**: `storm_spirits` ✅. Entity `storm_spirits:storm_spirit`, spawn egg `storm_spirits:storm_spirit_spawn_egg`,
  item `storm_spirits:spark` ✅ (§4).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4),
  the behaviours moved by hand (`creature::body`), fighting back (`creature::anger`, §3.12, shared with the Scorpions),
  the lights of glowing creatures (§3.11), the creature guide's pages and Items tab (§7).
- Sunrise: the `lightning_strike` hook (its `cause` tells a weather bolt from a trident's, a command's or a mod's) for
  its birth, `strike_lightning` for the spark's bolts, `entity_struck_by_lightning` and the damage hook for its
  immunities.
- It copies the wandering trader (goals only), with a box of 0.8 × 1.8 blocks ✅. Its own sounds (a crackle, a buzz, a
  zap) from freesound, CC0, cut by `cargo creature sounds` ❓ (vanilla's for now, §0).

## 2. The storm spirit ✅

- **Its look**: about a player's height, a figure of living lightning that only hints at a body: a glowing chest of
  blue-white light with a brighter core, a head that is a crackling crest with two bright eyes, long jagged arms of
  lightning, and no legs (its body thins into a flickering tail of arcs). Crackling arcs leap around it. It glows: drawn
  at full brightness, with a soft blue light around it, seen only (the library's lights, common/SPECS.md §3.11): Sunrise
  gives a mob no world light.
- 20 health. Immune to lightning and fire. Water does not hurt it: it flies over it.
- **Born of lightning**: when a **weather** lightning bolt strikes, in any biome, one chance in 10 a spirit appears at
  the strike, with a crackle. Not from a trident's Channeling or `/summon` (no farming with a trident). It never
  spawns otherwise (no natural spawning list).
- **Lives with the storm**: while it thunders it is lively. When the storm ends it fades within 1 to 3 minutes (a last
  spark, no drops).
- **Flight**: it always flies, 1 to 4 blocks above the ground or water, in short, jerky dashes (a flicker, then a quick
  zip of a few blocks), buzzing.
- **Neutral**: it ignores players. Hit, it **shocks** back: it dashes at the attacker and zaps (3 damage and a moment
  of Slowness), then keeps away and zaps again every few seconds until it loses them. Anything touching it is zapped
  (1 damage).
- **Drops**: 1 to 3 sparks (§4), plus one per Looting level.

## 3. Bestiary page ✅

The facts: "Born" (saw one appear at a strike), "Shock" (zapped by one), "Bolt" (called lightning with a spark).

## 4. Items ✅

- **Spark** (`storm_spirits:spark`): a crackling shard of light, stacks to 16. **Right-click** to call a lightning bolt
  on the block you look at, up to 32 blocks away (in the air with no block in range, it fails and is not used). Used
  up on use, 2 seconds of cooldown. Works in any weather and dimension.

## 5. Out of scope for v1

- Befriending and riding, spirits of other elements, the glass jar (too big for it), lightning rods, copper and
  creepers, a reusable lightning item, powering redstone, brewing with sparks.

## 6. Open questions

- ~~Q1: The name?~~ Storm Spirits, `storm_spirits` (§1).
- ~~Q2: How often it is born?~~ One weather strike in 10, never from a trident or `/summon` (§2).
- ~~Q3: After the storm?~~ It fades within minutes, no drops (§2).
- ~~Q4: Its temper?~~ Neutral, shocks back when hit (§2).
- ~~Q5: Lightning rods, copper, creepers?~~ Left out (§5).
- ~~Q6: The items?~~ The spark only, used up (§4).
- ~~Q7: The glass jar?~~ No, too big (§5).

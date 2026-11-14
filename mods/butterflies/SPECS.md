# Butterflies: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): butterflies that bring life to
flowery places, that you catch in a glass jar and settle where you like. Deliberately small in scope ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player).

- **Butterflies and moths** (`butterflies:butterfly`, `butterflies:moth`, spawn eggs): tiny wandering trader copies, 1
  health, muted, drawn from `assets/models/butterfly.bbmodel` and `moth.bbmodel` (pictures in docs/MODELS.md). A
  butterfly gets a look the first time it is seen, from the library's colours and patterns (common/SPECS.md §5): 8
  colours, 6 patterns (plain, eye spots, wing tips, veins, dots, tails), weighted per biome (§2); the moth is painted
  (§2.1).
- **Flight** (the library's small fliers, common/SPECS.md §3.5): butterflies by day in fair weather, moths by night;
  short erratic flights, landing on a flower or the ground; a sprinting player close by makes them fly off. At night a
  moth flutters around the nearest bright light. Wild ones despawn far from players.
- **Spawning**: butterflies in flower forests, meadows, sunflower plains, cherry groves, plains, forests, birch forests
  and jungles; moths in forests, birch and dark forests, taigas, plains and meadows.
- **Glass jar** (common/SPECS.md §4): caught and released keeping its look; released on a flower it settles there; a
  filled jar placed as a block shows it inside.
- **Butterfly dust** (§4): dropped by both; thrown, a shimmering cloud; with paper, 8 confetti, a party popper: right-click pops it
  with a bang and a spark, and a cone of coloured bits flies out ahead and falls.
- **Bestiary pages**: the biomes met in, "Catch", "Settles", the butterfly's morphs.
- **Sounds**: a soft flutter when one takes off (a moth's is duller), from CC0 freesound recordings
  (`assets/sounds.toml`).

Left: their shadow is too big (Horizon gives a mod mob 0.5;
arismoko/horizon#3896).

## 1. Technical context

- **Mod id**: `butterflies`. Entities `butterflies:butterfly`, `butterflies:moth` ✅; item `butterflies:butterfly_dust`
  ✅. No jar item of its own: a caught butterfly or moth is in the library's `creature:glass_jar` ✅ (library §4.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **colours and patterns** (§5, the butterfly only), **glass jar and
  territories** (§4, which this mod switches on), hitbox size (§3.4).
- The butterfly and the moth copy the wandering trader (goals only) with a **tiny hitbox**.

## 2. The butterfly ✅

- **One species** ✅ with **several colours and patterns** ✅ (library §5):
  - base colours: white, lemon, orange, red, sky blue, violet, brown, black;
  - patterns: plain, eye spots, wing-tip bands, veins, dots, tails (swallowtail shape ✅);
  - biome weights: bright colours in flower forests and meadows, whites and lemons in plains, browns in forests, blues
    and violets in jungles ✅.
- Wings beat in short erratic flights; it lands on flowers and folds its wings.
- **Ambient** and harmless: 1 HP ✅, no attack. Flees a sprinting player ✅.
- **Spawning** ✅: daytime, fair weather, near flowers, 1–3 together; at night they rest on leaves and flowers (no flight),
  and wild ones may despawn; settled ones never ✅.
- Sound: a faint flutter ✅.

## 2.1 The moth ✅ (night)

The butterflies' night counterpart ✅: a **moth** (`butterflies:moth`), flying **only at night** ✅.

- Stouter and furrier than a butterfly ✅. At rest its forewings sweep back into a wide triangle over its body, with
  a gently curved, scalloped and fringed rear edge, and a narrow V between them at the back where the segmented
  abdomen and the paler hindwings show; thin plain antennae angled forward ✅ (its model is
  built in this rest shape; flight spreads the wings from it).
- **One look** ✅: a speckled grey-brown, two faint cross lines and a fringe on the forewing, the
  hindwing paler, a fuzzy taupe body. Its colours are painted in its texture (`assets/models/moth.bbmodel`): it doesn't use the library's
  colours and patterns, which only the butterfly does.
- **Spawning** ✅: in forests, birch and dark forests, taigas, plains and meadows, at night; drawn to light ✅: it
  flutters around the nearest bright light. By day it rests (no flight), and wild ones despawn far from players.
- **Caught and settled like a butterfly** ✅ (library §4, rule "flower patch"), but a **settled moth only moves at night**
  ✅: by day it stays still on a block of its area.
- Ambient and harmless, 1 HP, flees a sprinting player; drops butterfly dust like a butterfly ✅.

## 3. Catching and settling ✅

- **Caught with a glass jar** ✅ (library §4.1): right-click a butterfly with an empty jar.
- **Released** from the jar on a block, it **won't leave a given area** around one or several blocks ✅.
  Rule "flower patch" (library §4.2):
  - released **on a flower**, that flower is its anchor: it roams over the **flowers within 8 blocks** of it, plus 2
    blocks around them and 4 blocks up. The cap keeps the area small (at most ~20 × 20 blocks) even in a meadow or a
    flower forest where flowers are everywhere; in a garden, the flower beds are its area;
  - released **anywhere else**, it doesn't stays (and then can dispawn).
  - if the anchor flower is broken, it moves its anchor to the nearest flower within 8 blocks, or flies off if there is
    none ✅.
- A filled jar can also be **placed as a decorative jar** ✅ (library §4.1, "display").

## 4. Butterfly dust ✅

- **Dropped when a butterfly is killed** ✅ (1 dust ✅; Looting adds 1 ✅).
- **One dust item for every colour** ✅ (`butterflies:butterfly_dust`).
- **Thrown** ✅: right-click throws a shimmering cloud of it ahead (pale dust, glints, a chime); one used
  up (not in creative).
- **Confetti** ✅: `butterflies:confetti`, 8 from paper and butterfly dust (shapeless). Right-click throws a
  burst of bright colours a little ahead (a firework's twinkle); one used up (not in creative). Particles only.

## 5. Out of scope for v1

- caterpillars and cocoons.

## 6. Open questions

- ~~Q1: Species?~~ One species, several colours and patterns.
- ~~Q2: Catching?~~ With a glass jar; released, it keeps to an area.
- ~~Q3: Butterfly dust?~~ Dropped when a butterfly is killed; one item for all colours.
- ~~Q4: The settled area?~~ The flowers within 8 blocks of the anchor flower (§3).
- ~~Q5: Day only, or moths at night too?~~ Moths at night (§2.1); a settled moth only moves at night.
- ~~Q6: The moth's look, colours and attraction to light?~~ One look, painted in its texture; it flutters around bright lights (§2.1).

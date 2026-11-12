# Butterflies: specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): butterflies that bring life to
flowery places, that you catch in a glass jar and settle where you like. Deliberately small in scope ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).
**Used by** Wyverns when both are installed ✅: its staff is then crafted with butterfly dust (§4), and with gunpowder
without Butterflies. Neither mod requires the other.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-09)

- Folder and spec created.
- **In game** (2026-11-11, tested): `butterflies:butterfly` and `butterflies:moth`, with spawn eggs (in the library's
  tab), copies of the wandering trader with their goals cleared, a tiny box, 1 health, muted. A butterfly gets a look
  the first time it is seen, from the library's colours and patterns (common/SPECS.md §7): 8 colours, 6 patterns
  (plain, eye spots, wing tips, veins, dots, tails), weighted per biome as in §2; the look is saved and comes back
  after a reload (tested). On the ground it opens and closes its wings slowly. The moth is drawn with its painted
  texture. Right-click does nothing (no trading screen); both are caught and released with the library's glass jar,
  the butterfly keeping its look (tested), each with its own jar icon. Their wings are thin slabs a little apart (flat
  wings flickered). Their shadow is too big (Horizon gives a mod mob 0.5; docs/HORIZON-ISSUES.md, "To open").
- **Flight, spawning, dust, bestiary** (2026-11-12, flight and settling tested): butterflies fly by day in fair
  weather, moths by night; short erratic flights 1 to 3.5 blocks up around where they first fluttered, then they land
  on a flower within 6 blocks (or the ground) for 5 to 30 seconds. A sprinting player within 4 blocks makes them fly
  off, away from them. No fall damage (with 1 health, a drop off a ledge killed one). Released from a jar on a flower
  they settle (the library's flower patch) and only fly over and land on its flowers. They spawn: butterflies in flower
  forests, meadows, sunflower plains, cherry groves, plains, forests, birch forests and jungles; moths in forests,
  birch and dark forests, taigas, plains and meadows (a biome modifier can't ask for night or flowers: they spawn
  anywhere in those biomes and rest when it isn't their time). Butterfly dust drops from both (1, Looting adds up to
  1). Bestiary pages: met wild (not settled) in those biomes; the fact "Catch" once one is caught in a jar. Not yet:
  moths drawn to light, despawning, the jar placed as a block, the dust in the Wyvern Staff (Wyverns isn't started).
- **Models** (2026-11-10): `assets/models/butterfly.bbmodel` and `moth.bbmodel` (pictures in docs/MODELS.md), one group
  tree for both (body, head and antennae, a forewing and a hindwing per side, flat wings cut out by the texture's
  alpha). The butterfly's texture is greyscale, for the library to tint (§2); the moth's is painted (§2.1). Not in
  game yet.

## 1. Technical context

- **Mod id**: `butterflies`. Entities `butterflies:butterfly`, `butterflies:moth` ✅; item `butterflies:butterfly_dust`
  ✅. No jar item of its own: a caught butterfly or moth is in the library's `creature:glass_jar` ✅ (library §6.1).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **colours and patterns** (§7, the butterfly only), **glass jar and
  territories** (§6, which this mod switches on), hitbox size (§8.8).
- The butterfly and the moth copy the wandering trader (goals only) with a **tiny hitbox**.

## 2. The butterfly ✅

- **One species** ✅ with **several colours and patterns** ✅ (library §7) :
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
  abdomen and the paler hindwings show; thin plain antennae angled forward ✅ (2026-11-10, its model is
  built in this rest shape; flight spreads the wings from it).
- **One look** ✅ (2026-11-10): a speckled grey-brown, two faint cross lines and a fringe on the forewing, the
  hindwing paler, a fuzzy taupe body. Its colours are painted in its texture (`assets/models/moth.bbmodel`): it doesn't use the library's
  colours and patterns, which only the butterfly does.
- **Spawning** ❓: at night, near light sources and flowers, 1–2 together; drawn to light: it flutters around lanterns,
  torches and campfires within ~10 blocks ❓. At dawn it rests on a log, a wall or the underside of leaves (no flight)
  and wild ones may despawn.
- **Caught and settled like a butterfly** ✅ (library §6, rule "flower patch"), but a **settled moth only moves at night**
  ✅: by day it stays still on a block of its area.
- Ambient and harmless, 1 HP, flees a sprinting player; drops butterfly dust like a butterfly ❓.

## 3. Catching and settling ✅

- **Caught with a glass jar** ✅ (library §6.1): right-click a butterfly with an empty jar.
- **Released** from the jar on a block, it **won't leave a given area** around one or several blocks ✅. Proposal
  (library §6.2, rule "flower patch"):
  - released **on a flower**, that flower is its anchor: it roams over the **flowers within 8 blocks** of it, plus 2
    blocks around them and 4 blocks up. The cap keeps the area small (at most ~20 × 20 blocks) even in a meadow or a
    flower forest where flowers are everywhere; in a garden, the flower beds are its area;
  - released **anywhere else**, it doesn't stays (and then can dispawn).
  - if the anchor flower is broken, it moves its anchor to the nearest flower within 8 blocks, or flies off if there is
    none ✅.
- A filled jar can also be **placed as a decorative jar** ✅ (library §6.1, "display").

## 4. Butterfly dust ✅

- An ingredient of the Wyvern Staff when Wyverns is installed ✅ (Wyverns uses gunpowder in its place without
  Butterflies).
- **Dropped when a butterfly is killed** ✅ (1 dust ✅; Looting adds 1 ✅).
- **One dust item for every colour** ✅ (`butterflies:butterfly_dust`).

## 5. Out of scope for v1

- caterpillars and cocoons.

## 6. Open questions

- ~~Q1: Species?~~ One species, several colours and patterns.
- ~~Q2: Catching?~~ With a glass jar; released, it keeps to an area.
- ~~Q3: Butterfly dust?~~ Dropped when a butterfly is killed; one item for all colours.
- Q4: The settled area: flowers within 8 blocks of the anchor flower (§3)?
- ~~Q5: Day only, or moths at night too?~~ Moths at night (§2.1); a settled moth only moves at night.
- Q6: The moth's look, colours and attraction to light (§2.1).

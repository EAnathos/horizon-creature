# Butterflies — specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): butterflies that bring life to
flowery places, that you catch in a glass jar and settle where you like. Deliberately small in scope ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).
**Required by** Wyverns ✅ (its dimension staff is crafted with butterfly dust, §4).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-08)

- Folder and spec created. **Not started**.

## 1. Technical context

- **Mod id**: `butterflies`. Entity `butterflies:butterfly`, items `butterflies:butterfly_jar` ❓ (a jar holding one),
  `butterflies:butterfly_dust` ✅.
- Uses from the library: model drawing and sounds (§3.1, §3.2), **colours and patterns** (§7), **glass jar and
  territories** (§6, which this mod switches on), hitbox size (§8.8).
- The butterfly copies the wandering trader (goals only) with a **tiny hitbox**.

## 2. The butterfly ✅

- **One species** ✅ with **several colours and patterns** ✅ (library §7). Proposal ❓:
  - base colours: white, lemon, orange, red, sky blue, violet, brown, black;
  - patterns: plain, eye spots, wing-tip bands, veins, dots, tails (swallowtail shape ❓);
  - biome weights: bright colours in flower forests and meadows, whites and lemons in plains, browns in forests, blues
    and violets in jungles ❓.
- Wings beat in short erratic flights; it lands on flowers and folds its wings.
- **Ambient** and harmless: 1 HP ❓, no attack. Flees a sprinting player ❓.
- **Spawning** ❓: daytime, fair weather, near flowers, 1–3 together; at night they rest on leaves and flowers (no flight),
  and wild ones may despawn — settled ones never ✅.
- Sound: a faint flutter, or none ❓.

## 3. Catching and settling ✅

- **Caught with a glass jar** ✅ (library §6.1): right-click a butterfly with an empty jar.
- **Released** from the jar on a block, it **won't leave a given area** around one or several blocks ✅. Proposal
  (library §6.2, rule "flower patch"):
  - released **on a flower**, it settles on the **flower patch**: every flower connected to it (each within 3 blocks of
    another), plus 3 blocks around and 4 blocks up. Plant more flowers next to the patch and its area grows: the player
    designs a butterfly garden;
  - released **anywhere else**, it stays within 4 blocks of the release block ❓ (a lantern, a planter on a balcony);
  - if the patch's flowers are all broken, it flies off (no longer settled) ❓.
- A filled jar can also be **placed as a decorative jar** ✅ ❓ (library §6.1, "display"), the butterfly fluttering
  inside, glowing faintly ❓.

## 4. Butterfly dust ✅

- An item needed by Wyverns' dimension staff ✅ — and a possible dye ❓.
- How to get it ❓. Proposals:
  - **A** (gentle): a butterfly in a placed display jar sheds 1 dust every 10 min ❓, or a settled butterfly drops one now
    and then on its flowers ❓;
  - **B**: brushing a butterfly (vanilla brush) gives 1 dust, then it can't be brushed for 5 min ❓;
  - **C**: a butterfly drops 1 dust when killed.
- Coloured dust by the butterfly's colour (a dye of that colour) ❓.

## 5. Out of scope for v1

- Taming, riding, combat, caterpillars and cocoons.

## 6. Open questions

- ~~Q1 — Species?~~ One species, several colours and patterns.
- ~~Q2 — Catching?~~ With a glass jar; released, it keeps to an area.
- Q3 — How butterfly dust is obtained (A, B or C, §4)? Coloured dust as dyes?
- Q4 — The settled area: the flower-patch rule (§3), or something else?
- Q5 — Day only, or moths at night too?

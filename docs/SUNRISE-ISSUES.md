# Sunrise issues

The issues we raised or follow on [Sunrise](https://github.com/sunrise-org/sunrise), and the workaround each one left in
our code (marked there with its issue). Update this page when an issue moves, and before moving the series' pin
(below). Issues go to [sunrise-org/sunrise](https://github.com/sunrise-org/sunrise/issues): search it first, open
and closed, and file only with a clean repro and the pinned revision.

Our pin: Nightly `nightly-2026-11-17-2d81263`, source `2d8126336e` (every `sunrise-mod.toml`'s `[native]` pins and
CI's `SUNRISE_REV`). Last checked: 2026-11-18.

Status: **to open** (found, not filed), **open**, **fixed upstream** (merged, not yet in our pin), **done** (in our pin,
workaround removed).

## Open

| Issue | What | Status | Our workaround (where) | Drop it when fixed |
|---|---|---|---|---|
| [#2671](https://github.com/sunrise-org/sunrise/issues/2671) (ours; follows [#1286](https://github.com/sunrise-org/sunrise/issues/1286)) | From `menu_clicked`, `ItemPlace` reaches neither the clicking player's inventory (`PlayerSlot` by UUID, `MenuSlot` on an inventory slot) nor the cursor: they read and take 0 | open (filed 2026-11-18) | The vulture's screen moves a whole stack across with a click, written with `set_inventory_slot` and `menu_set_items`, no cursor (`mods/vulture/src/screen.rs`, `clicked`) | Pick up, put down and swap with the cursor, as a chest's slots |
| [#1134](https://github.com/sunrise-org/sunrise/issues/1134) (not ours, we follow it) | `entity_size` isn't asked for a client's copies of entities: there, our creatures keep their template's box (a wandering trader's, 1.95 high) | open | The book centres our creatures by their own box, offset from the template's height (`common/src/bestiary/book.rs`, `TEMPLATE_HEIGHT`) | Drop the offset |

## Limits of our workarounds

| Workaround | Its limits |
|---|---|
| Whole stacks across in the vulture's screen (#2671) | No cursor: no splitting a stack, no placing one item, no drag; a click sends as much as fits |
| The book's offset (#1134) | Reads our box from the library's table (`creature::mob::size_of`), not from the entity |

## To open (candidates)

Found, not filed: check the tracker and the newest Nightly first.

- **A mod mob's spawn egg never says "Disabled in Peaceful"**: the game decides it from the copied vanilla type (a
  mod mob copies a zombie, a villager or a wandering trader), with no way to call it a monster. Ours: the cyclops's
  tooltip adds the line from a static its server tick keeps (`mods/cyclops/src/lib.rs`, `PEACEFUL`), single player
  only.

## Moving the pin

1. Take the newest Nightly the launcher offers (`sunrise-launcher versions`, `update`) and its commit.
2. Check which of the issues above it fixes (`git log <old>..<new>` in the Sunrise sources, the issues' state); update
   the tables.
3. Set the commit in every `sunrise-mod.toml` (`[native]`) and CI's `SUNRISE_REV`; `cargo creature check`,
   `cargo creature native-check`, `cargo creature install`; drop the workarounds it fixes; test in game.

## Horizon

The issues we raised on Horizon (horizon-rust/horizon and the arismoko/horizon fork) and their workarounds ended with
the move to Sunrise: each one is answered or gone (the port's notes: `git show 2c09384:docs/SUNRISE-PORT.md`, §5).
The old page: `git show e542ff7:docs/HORIZON-ISSUES.md`.

# WriftHeart — the Rust port

Porting `Code/Game Dev/Wriftheart` (vanilla JS + Canvas, ~28k lines, 44 modules, v0.6.46)
to Rust. **The JS project is a FROZEN, read-only reference** (Baz, 2026-07-16): it is the
source of truth for every behaviour question, but it receives NO further changes — no
fixes, no back-ports, nothing. At feature parity it is retired and THIS port becomes the
main game and the source of truth.

**Improve, don't transliterate** (Baz, 2026-07-15): the rebuild is the chance to do things
BETTER. What must survive the port is the game's *behavior and feel* (gameplay numbers,
worldgen determinism, the look) — not the JS architecture. Where the original grew pain
(hardcoded tab indexes, copy-pasted chrome, mode booleans), design the right structure
instead. Pitch notable deviations to Baz before or as you build them, and mark deliberate
behavior deviations in comments (see the axe-spin fix in `actors/attacks.rs`).

## Decisions (locked 2026-07-15)

| | |
|---|---|
| **Engine** | Bevy 0.19 |
| **Target** | Native desktop only (no WASM) |
| **Scope** | Game core first; co-op + launcher come after parity |

### What Bevy means for this port

This is a **redesign, not a transliteration.** The JS is a hand-rolled entity list where each
entity owns `update(room, player, blocked, spawn)` and `draw(ctx, ox, oy)` closures. Bevy is an
ECS: data lives in **components**, behaviour lives in **systems** that query for components, and
nothing owns a draw call — you spawn an entity with a `Sprite` and the renderer handles it.

So a JS mob like:

```js
mob({ type: 'wolf', health: 6, ai(e, room, player, blocked) { ...chase... } })
```

becomes roughly: an entity with `Enemy`, `Health(6)`, `Sprite`, `Transform`, and a marker
`Wolf` — plus a `wolf_ai` system that queries `(&mut Transform, &Wolf)` and the player's
position. **Resist the urge to rebuild the entity-list in Rust.** Where the JS has one big
`update()` doing everything in order, we get many small systems; where it has `if (e.type === x)`
chains, we get separate systems per marker component.

The trade we accepted: more upfront restructuring, in exchange for parallel systems, real
types, and room to grow. Compile times are the tax (see the dev profile in `Cargo.toml`).

## Anti-monolith rule (enforced)

The JS project's one architectural regret is `game.js` at ~7,000 lines. **This port must never
grow a file like that.** The rule:

- **No source file past ~500 lines.** When one approaches it, split along a seam BEFORE adding
  more — a new sibling module, or promote the file to a directory (`enemies.rs` -> `enemies/`
  with one file per behaviour family).
- **crate is lib + bin.** `main.rs` is only app wiring; every subsystem is a library module so
  it can be tested in isolation. Logic never lives in `main.rs`.
- **One concern per module.** `gfx/palette.rs` is the palette and nothing else; `worldgen/rng.rs`
  is the determinism core and nothing else. A module that starts doing two things is two modules.
- **`mod.rs` is glue only** — declarations + a few `pub use` re-exports, no logic.

Quick check: `find src -name '*.rs' | xargs wc -l | sort -rn | head` — nothing near 500.

## Reuse rule (enforced) — "a window is a function you call"

The JS version's other pain (user's words): the same code pasted over and over — every dialog
hand-drew its own panel chrome, every picker reimplemented list scrolling, and a change meant
hunting every copy. In this port, **the second time a pattern appears, it becomes a shared
function/type — before the third copy is written.** Concretely:

- **UI chrome is primitives, not paste.** ONE `ui/` module owns `panel()`, list/scroll state,
  progress bars, prompts, text helpers. Screens compose primitives; they never re-draw chrome.
  If two screens differ only by data, they are one function with parameters.
- **Per-direction / per-edge / per-facing logic is table-driven** — one loop over a table of
  (data, coords), never four pasted blocks (see the connect/footing tables in
  `worldgen/generate.rs` for the pattern).
- **One source of truth per fact.** Palette, tile solidity, layout constants, damage formulas:
  each lives in exactly one place and is imported everywhere else. Duplicating a constant or a
  formula "just for now" is how the JS got here.
- **Input prompts are DERIVED, never typed** (the user's canonical example of the disease:
  on-screen "press X" text was hardcoded in the JS, so a binding change meant hunting every
  string). Rule: gameplay code speaks in semantic actions (`Action::Confirm`, `Action::Cancel`,
  `Action::Menu`, …); a `Bindings` table maps actions to keys/pad buttons; every on-screen
  prompt renders through ONE `prompt(action) -> &str` lookup at draw time. No UI string may
  ever name a key or button literally — rebind once, every prompt updates itself.
- Rust makes the right thing cheap: if you're copying a block to tweak two values, stop —
  it's a `fn` with two parameters, or a table row.

## Layout

```
src/
  lib.rs             crate consts + module declarations (thin)
  main.rs            Bevy app wiring + the canary scene (thin)
  gfx/
    mod.rs           re-exports
    palette.rs       PALETTE            — port of js/assets.js
    bake.rs          bake() + flip_h()  — port of js/assets.js
    canvas.rs        low-res render target + `at()` coordinate bridge
  worldgen/
    mod.rs
    rng.rs           hash + Mulberry32 + value_noise  — the determinism CORE
  actors/
    mod.rs
    hero.rs          hero sprite grids + recolor  — top of js/player.js
tests/
  worldgen_parity.rs        Rust rng == JS, bit-for-bit
  data/worldgen_golden.rs   GENERATED golden vectors (do not hand-edit)
```

## Module map (JS -> Rust)

Rough plan; not gospel. `game.js` is 7k lines and does not survive as one file — it shreds.

| JS | Rust | Notes |
|---|---|---|
| `assets.js` | `gfx/` | **done** — palette + `bake()` + canvas |
| `font.js` | `font.rs` | 3x5 bitmap font; bake glyphs, draw as sprites |
| `room.js` / `tiles.js` | `room.rs` / `tiles.rs` | tile grid + solidity |
| `world.js` | `world.rs` | **determinism is the hard constraint** — see below |
| `player.js` | `player.rs` | `Player` component + movement/attack systems |
| `enemies.js` | `enemies/` (module dir) | one file per mob family; `mob()` -> a bundle |
| `entities.js` | `entities/` | projectiles, FX, gatherables, chests, props |
| `items.js` | `items.rs` | registry -> a `HashMap<&str, ItemDef>` resource |
| `inventory.js` | `ui/inventory.rs` | |
| `codex.js` | `app/codex/` (mod + one file per tab) | tab REGISTRY, not index chains |
| `dungeon.js` | `dungeon.rs` | |
| `lighting.js` | `lighting.rs` | JS uses radial gradients; likely a shader or a baked mask |
| `audio.js` | `audio.rs` | **no drop-in** — procedural WebAudio synth, hand-rolled vs `kira`/`cpal` |
| `net.js` + `relay/` | *(later)* | co-op, after parity |
| game.js | shredded across the above + `app/` | |

## Port order (milestones)

1. **Canvas + bake** — pixel-perfect 384x216, char-grid sprites. *(done — the canary)*
2. **World-gen determinism core** — hash + PRNG + value noise, pinned to JS bit-for-bit.
   *(done — `worldgen/rng.rs`, `tests/worldgen_parity.rs` green)*
3. **World terrain generation** — biomes, towns/districts, roads+gates, door spans, water/
   rivers, connectivity Dijkstra, bridges, footings, streets, castle/shard paths.
   *(done — `worldgen/{biomes,towns,world,doors,edges,generate}.rs`;
   `tests/worldmap_parity.rs` reproduces 299 JS room maps byte-for-byte across 3 seeds)*
4. **Font** — 3x5 bitmap text. *(done — `gfx/font.rs` bakes whole strings to one texture;
   `measure` pinned to JS by `tests/font_parity.rs`)*
5. **Tiles + room render** — *(done — REAL tileset: art grids machine-extracted from
   js/tiles.js into `gfx/tiles_art.rs` (generated), procedural generators ported in
   `gfx/tile_textures.rs` (thin-ground variants, 4-phase water, bridge decks) and pinned by
   `tests/tilevar_parity.rs`; rooms spawn under a slidable root (`app/room_render.rs`) with
   animated water + oriented bridge decks; edge dressing (`gfx/edge_dressing.rs`): rounded
   terrain corners + scalloped hedges as ONE static per-room overlay image at z=2, pinned
   rect-for-rect to room.js by `tests/dressing_parity.rs` (harness: `tools/extract_dressing.mjs`
   — evals tiles+room+world verbatim with a fillRect-recording ctx))*
6. **Player** — *(done, v1 — `input.rs` semantic Actions + DERIVED prompts; `app/play.rs`:
   JS-exact movement @60Hz fixed (1.25 spd, feet box, axis-slide, sqrt-half diagonals),
   4-frame gait + bob, facing rules; the Zelda room-SLIDE transition (both rooms scroll
   PX/8 frames, player lerps, gait at 6 ticks) + `safe_entry` edge nudge.
   TODO: `nudgeToOpenEntry` once solid entities exist; walk-and-attack with weapons)*
7. **Entities + combat** — *(in progress — LANDED: `combat.rs` resolve pass (teams,
   i-frames, one-shot vs persistent contact +-3px, defense min-1, knockback via per-kind
   HurtProfile, HitLanded messages -> blood FX); goblins (melee axe + slinger stones, JS-exact
   AI numbers) via `actors/goblin.rs` + machine-extracted frames; player sword arc (cooldown
   20 / lock 14 / 55% mid-swing walk), hurt/i-frame blink, HP bar HUD; THE PROP LAYER:
   `worldgen/entities.rs` is the full getEntities port (every salted stream + shared
   `used`-occupancy, authored castle/shard/rift set-pieces; descriptor parity across 1,124
   golden entities in tests/entities_parity.rs) — mob positions are now byte-parity too
   (superseded spawns.rs mob_roster); `actors/props.rs` ports the SEEDED tree generators
   (buildOak/Pine/Cactus/Deadtree, grid parity on 56 sampled seeds) + `props_art.rs`
   (generated: bushes, ore-node boulders, grass sway frames, flowers, 21 clutter kinds);
   `app/room_props.rs` spawns them as room-root children with solid-prop RoomBlockers
   (enter-only: landing inside never traps) and `room_render::actor_z` — the painter's
   y-sort as a z band [4,8] for actors + trees (attacks ride wielder_z ± epsilon).
   GATHERING (`app/gather.rs` + combat.rs gates): nodes carry Health + GatherTool; the
   resolve pass gates by tool (wrong tool = tink spark; js order kept), hits shake the node
   and spray its chip colour; starter loadout = sword/axe/pick on Slots 1-3 (per-tool
   SwingSpecs from items.js: axe 2dmg/0.7pi/cd30, pick 1dmg/0.6pi/cd28); felled nodes drop
   material pickups that magnet into the `Materials` store; gathered nodes respawn NEXT DAY
   (DAY_LEN 36000 frames; GatherState per room), felled trees day-stamp into `TreeGrowth`
   and regrow stump -> sapling -> young(solid) -> full over 3 farm-days (stage sprites ported
   in `actors/props.rs` stage builders, per-kind STAGE_COL tints; WRIFT_SHOT=stages shows the
   gallery). GOTCHA hit: a custom Message must be `add_message`d or every system reading it
   fails validation and the whole chain dies silently.
   BIOME MOBS (base roster DONE — `actors/mobs.rs` + generated `mobs_art.rs` via
   tools/extract_mobs.mjs): the js mob() scaffold as a DATA TABLE — MOB_DEFS rows +
   an `Ai` enum one system interprets (Walker / Chaser-with-lunge / Flyer / Dormant /
   WebSpitter / Burrow / Swoop / Hurl — the js approach/lunge/kite helpers, numbers
   verbatim). Ported kinds: boar (charge + wall-stun cadence), wasp, thornling (bush
   disguise), wolf (per-facing frames, vec-chase + lunge), bear, spider (web bolt +
   kiting), scorpion, burrower (mound/tunnel/surface cycle, unhittable underground),
   vulture (dive + retreat), golem (knock-immune), bat, hurler (dart-and-lob, the
   telegraphed arcRock landing). HP_MUL 1.5, AGGRO_R 80 idle-until-close (struck = awake),
   per-kind deaths (coins/materials/potion chances/xp/bestiary/ledger). spawn_room_mobs
   spawns REAL mobs for ported subs; unported kinds still fall back to goblins.
   WRIFT_SHOT=roster is the art gallery. BATCH 2 (done): the classics + new-biome kinds —
   zombie (downRevive: collapses sideways, flickers, rises at full HP), skeleton, archer
   (kite + rooted bowshot; real rotated arrows, drops 'arrow' items — added to the item
   registry), frog (leaps; its tongue-grab reel is DEFERRED to the player-pull mechanic),
   leech, gnat, lurker, ghoul, wraith (0.8-alpha ghost), revenant, the SLIME family
   (water/frost/ember/spark/toxic — hop cycle + split into two small children with spawn
   i-frames), frostmite, icetroll, frostwyrm, cinderhound, charbrute, pyrewraith,
   sporeling, myconid, sporemother (3-bolt fan), chaoswisp, voidling (blinks behind you),
   riftlord — new Ai archetypes: Shooter, Caster (pre-baked eBolts), Hopper, Blinker,
   FrogHop; Walker gained a range gate. 36 defs total; the bestiary lists all of them.
   Mob REMAINING: the unique-mechanic mob pass (~19 kinds), red goblins, lootgoblin,
   darkknight + dungeon bosses, champion auras, contact/bolt afflictions (slow/poison/
   burn/shock — the status system), the frog tongue, mob ground-shadows.
   REMAINING: townEntities,
   interactive props (shop/wagon/chest/songstone/crackedrock/saltmaze door), set-piece
   structures (castle/dungeon/rift/fences/braziers/wisps/guards), exotic biome tree arts
   (oak fallback), grass tints + Pokémon wading overlap, ore/wood tier ladders by zoneTier +
   tool tiers (reqTier), herb/silk/luck drops, gather sounds, XP/coins/drops, the YOU DIED
   screen, champion auras/affixes, nudgeToOpenEntry.)*
8. **Input + UI foundation** — *(done — `input.rs`: GAMEPAD support (DEF_PAD port: face
   buttons = slots, LB = codex, RB = inventory, Start = pause; d-pad + left-stick with edge
   presses), the poll/consume ActionState model (presses seen by exactly one fixed tick, the
   endFrame contract); `ui/widgets.rs` primitives (label/panel/bar/ListNav — "a window is a
   function you call"); `app/hud.rs` sidebar to the JS layout (name+LVL, HP/MP/XP bars, 4
   ability slots with DERIVED button labels that flip to pad glyphs on connect); `app/menu.rs`
   pause menu — the first window built purely from primitives, controller-drivable.
   TODO: rebinding UI, MP/XP live values, compass minimap.)*
8a. **Slide-out menu** — *(v1 done — `app/slideout.rs`, port of js/inventory.js: the game's
   MAIN MENU slides in from the right over the play area (0.2/frame, never covers the
   sidebar), freezes the world (Screen::SlideOut), tab REGISTRY matching the JS TABS
   1-for-1: CHAR (vitals plate + the 8-col bag grid, gathered materials in the first
   cells), CRAFT / SKILLS / STATUS (each renders as much as its system has ported —
   STATUS's empty state is real). I/RB opens+closes, Q-R / LT-RT switch tabs, heldLatch on
   every exit path. The CHAR page now runs the full carry model (see milestone 9).
   REMAINING: hand recipes (CRAFT), slide-OUT animation on close. NOTE: the HUD sidebar renders at z 13-14, ABOVE the whole play-field stack —
   the JS painted the sidebar last each frame, so sliding rooms (tree canopies at actor-band
   z) must never cross it.)*
8a2. **Passive tree** — *(done — `src/skilltree.rs`: the 8-branch constellation ported
   node-for-node (81 nodes pinned by tests/skilltree_parity.rs: positions, stats, costs,
   the link graph incl. the n1 ring road); `app/slideout/skills_tab.rs`: starfield/nebula/
   halo render, cone-based directional cursor (spatial-first, link fallback), allocate with
   rising costs, leaf-safe refunds, camera lerp, K hotkey jumps to the tab. Tree stats
   apply LIVE where systems exist: melee% on swings, move% on speed, maxhp on Health,
   gather yield bonus, pickup magnet range; the rest bank in the tree until their systems
   port (crit/leech/haste/spell/coin/craft/regen/knock). TEMP: 8 starter points + free
   refunds until XP/leveling + coin port.)*
8b. **Codex** — *(ALL 11 js tabs present in js order (MAP/CALENDAR/PEOPLE/GUILDS/MOBS/
   ITEMS/SONGS/AWARDS/STATS/LORE/WRIFTHEART) — `app/screen.rs` Screen states; `app/codex/`
   TabDef registry (stable TabIds, never index-renumbered), frame + tab strip + per-tab
   derived-prompt hints. LIVE TABS: MAP (visited-room thumbnails, zoom/pan, gold ring);
   MOBS (kill-gated bestiary — `Bestiary` set fed by battle deaths, '?' until slain);
   ITEMS (the item dex over the ported registry, rarity-then-name order, revealed via the
   `Discovered` watcher that marks everything the inventory has ever held); CALENDAR
   (season title + year + DAY n OF 28 + the 7x4 month grid, today ringed in the season
   colour; festival pennants/birthdays/IN-SEASON crops join with their systems — the crop
   column shows the js's own "FIELDS LIE FALLOW"); STATS (THE LEDGER OF DEEDS: the js
   statLines list VERBATIM, two columns + gold banners + 2-line scroll, fed by
   `app/stats.rs` — a Stats map with js-keyed counters and bump sites at kills/coins/
   trees/stones/grass plus one observer for frames/walk/dmg/deaths; unported counters
   print their zeros exactly like a fresh js save). STUB TABS (js true-empty states,
   upgrade in place): PEOPLE ("NO ONE MET YET"), GUILDS/AWARDS/LORE/WRIFTHEART ('? ? ?'
   mysteries under their js headers), SONGS (the illuminated SONGBOOK title + mystery).
   The two-pane dex is a shared module (`codex/dex.rs`: dex_nav/draw_grid/draw_pane/blit —
   js drawDexGrid/drawDexPane/dexBlit verbatim, scrollbar included). WRIFT_TAB=<title>
   opens any tab under WRIFT_SHOT=codex. REMAINING: prop pixels on map thumbs, town/
   dungeon markers + quest pins, per-tab content as villagers/guildhalls/songs/awards/
   lore/shards port. GOTCHA: canvas rgba-alpha overlays read too transparent in Bevy
   (linear-space blending) — use opaque near-black.)*
9. **Items + inventory** — *(increment 1 done — `src/items.rs`: the registry (ItemDef +
   RARITY table/prices/sell-40%, `use_consumable` dispatch replacing the JS use() closures)
   with the STARTER SET (sword/axe/pick, potion, wood/stone/fiber/herb/copper — defs +
   icons verbatim from js/items.js, HERB/COPPER icons hand-embedded in
   `actors/items_art.rs`); `src/inventory.rs`: PlayerInv, the unified uid model from
   js/player.js (entries/bagOrder/slots[4]/gear[6] all store per-instance UIDS;
   add/canAdd/removeOne/removeEntry/removeEntryAll/removeStack/detach/toBag/sortBag ported
   + unit tests). Wired end-to-end: gather drops are real `spawn_pickup` items (icon
   sprites, qty, magnet only while `can_add` — full bags leave drops on the ground),
   ability slots hold item INSTANCES (per-slot cooldowns `p.cooldowns[4]`, weapons
   auto-repeat one-per-tick, consumables on press; potion heals ceil(30%) min 3, vetoes at
   full HP), HUD slots live-draw icon/rarity border/qty/cooldown-dim, and the CHAR page
   carries the FULL updateGear model: A pick/place/swap (cellAccepts + swap-back
   validation), X use, Y drop tap-one/hold-40f-stack with the amber/red progress bar,
   T/R3 trash, H/L3 sort, SHIFT instant-stack, B cancels a carry before it closes, cyan
   #7fe0ff held border, 16x16 icons, wrapped desc + rarity/kind detail pane, the JS hint
   bar. TEMP deviations (commented): fresh hero carries sword+axe+pick (js: sword+shield —
   shields not ported); pad trash/sort are bare R3/L3 (js chords R3+RIGHT / SEL+LEFT).
   REMAINING for M9: armor/trinket defs + gear-stat refresh + worn-armor sprites, shields,
   satchel bag rows, item-get fanfare, shops, blueprints.
   CRAFTING v1 (done): `items.rs` Recipe table (the js RECIPES hand rows verbatim; a row
   only SHOWS once its output def exists — same fill-in rule as the loot pools; today:
   axe, pickaxe, bandage — the bandage item def ported with it) + `slideout/craft_tab.rs`,
   the js drawCraftWindow 1:1: recipe list left (craftable grey / not-craftable dark,
   gold selection), detail right (24px icon, rarity-tinted name, desc, MATERIALS rows
   with green/red have-need counts, the A CRAFT button), the rising "+1 NAME" / "BAG
   FULL" banner, the tree's CRAFT stat sparing materials, the crafts ledger bump.
   WRIFT_SHOT=craft stages it. CRAFT REMAINING: recipe pins, home-chest pooling,
   blueprint locks, station windows (workbench placement -> forge/alchemy/...),
   '@KIND' generic costs, batch outputs (outN).)*
9a. **Loot, XP & coins** — *(done — the reward loop. `app/rewards.rs`: Progress
   (level/xp, js xpForLevel = level*10; gainXP grants +1 tree point per level — the TEMP
   8 starter points are GONE, and a leaf-safe tree refund now costs REFUND_COST=40 coin),
   the LOOT FEED (js addLoot/drawLootLog: right-aligned toast pills bottom-right of the
   play field, rapid runs merge, 6 max, slide-in/fade-out, "+N COPPER" / "NAME xN
   (EQUIPPED)"), and the LEVEL-UP flourish (js drawLevelUp: golden flash + two shockwave
   rings + glow + pop-scaled "LEVEL UP!" — additive in js, alpha-blended at ~half
   intensity here per the linear rule, ring thickness fixed as it scales). Goblin deaths
   run the js deathEffect verbatim: 1-5 copper always, melee 15%·luck drops the AXE (the
   woodcutting bootstrap, no magnet), slingers 15%·luck drop 1-2 stone, 0.8%·luck rolls
   Items::roll_loot (TIER_BASE verbatim; pools FILTERED to ported defs — only potion
   today — refill as items port; the procedural weapon/armour substitution joins with the
   generator). Coins are Pickup::Coin: square COIN sprite, gold glow (gi 0.10), always
   magnetised, banked ×(1 + GOLD stat) into PlayerInv.money — the CHAR coin pips are
   live. Items collect through the js autoEquipPickup courtesy (fills an EMPTY matching
   slot, never swaps a choice; we detach the bag ref where the js leaves a dangling one).
   TreeStats grew luck + coin. HUD XP bar + LVL plate live. (A per-drop SPARKLE glint was
   built here and then REMOVED at Baz's call — the ground glow + the night light-pool
   cover it; the js book glint stays book-only.) REMAINING: red goblins/champions/elites
   (their coin/boost multipliers are noted inline), per-mob xp for the biome roster, mana,
   item-get fanfare, saves for level/money.)*
9b2. **Saves v1** — *(done — `app/save.rs`: ONE autosave file (JSON,
   ~/Library/Application Support/wriftheart/save1.json via `dirs`; serde) persisting the
   room + player pos/health, PlayerInv (entries/bag/slots/gear/money/uid counter),
   level/XP, the tree allocation (BY NODE ID — survives table reorders), the stats
   ledger, bestiary, discovered items, gather + tree-growth stamps, visited rooms, and
   the day/night clock. Loaded in PreStartup; play::setup boots straight into the saved
   room/pos and apply_save restores the resources PostStartup. Writes every ~10s of
   active play + on every pause. ROBUSTNESS: all item/mob/node refs save as ID STRINGS;
   unknown ids drop quietly (cross-build safety). WRIFT_SHOT runs never load or write
   saves. Round-trip pinned by a unit test + verified live (boot -> autosave -> reboot
   -> resumed clock). REMAINING: the 4-slot picker + title screen + delete/overwrite
   confirms, save-on-quit hook, per-slot world seeds, hero look/creator fields.)*
9b5. **Character creator + traits + hero identity** — *(done — the NEW GAME flow is now
   title -> slot pick -> CREATOR -> a fresh world, the full js loop. `app/creator.rs`:
   name (on-screen keyboard, one flow for pad + keys, physical Backspace convenience;
   rolled starter name pools M/F/N with gender defaulting to the name's tag), gender,
   hair/style/eyes/skin/outfit pickers (the js colour tables verbatim) with a LIVE
   turntable preview (head-centred per facing via the baked frame's alpha, js
   centerOffset), trait roll + REROLL, START ADVENTURE. `traits.rs`: the js 2-good+2-bad
   roster verbatim, mirror-pair-safe rolls (unit-tested), day/night quirk gating.
   `actors/hero.rs`: HAIR_EDITS row-swap hairstyles (8 styles + bald) applied per facing
   before baking; Look carries hair_style and serializes. `app/identity.rs`: HeroIdent
   (name/gender/look/traits, saved per slot) + the Night flag — day/night flips re-fold
   TreeStats. recompute() now = tree + traits (js player.stat); TreeStats grew
   defense/crit/leech/regen/iframes (banked until their systems port). Saves carry the
   whole identity + a PER-SLOT WORLD SEED (js World.setSeed — a new game rolls its own;
   the loader rebuilds World + HeroArt on the swap; old saves keep 1337). HUD name plate
   is live. THE BAZ EGG: naming him BAZ boots bald — applied at START, invisible in the
   preview, exactly the js. WRIFT_SHOT: `creator` stages the screen; `newgame` drives
   title -> creator -> START -> world (the loader smoke test). VERIFIED: creator + title
   + flyover drift screenshots eyeballed real; the story crawl remains visually
   unverified (see the black-frame note below).)*

BLACK-FRAME THEORY UPDATE (2026-07-16): the episodes look OCCLUSION-CORRELATED — early
frames (90/560) captured fine while later frames (900/1600) of the same build came back
byte-identical black; macOS stops redrawing fully-occluded windows, so a WRIFT_SHOT
capture goes black whenever the game window has slipped behind others by capture time.
Keep captures early (default frame 90), keep the window front, and re-shoot rather than
bisect.

67. **BESPOKE DUNGEON BOSSES — ALL SIX, the elite stand-ins RETIRED (+ a boot-blocker fix)** —
   *(the 16 themes THE TEN left on elite stand-ins now get REAL bespoke bosses, not the js
   template (Baz's call: authored, one signature mechanic each). Scope agreed with Baz: 6
   consolidated bosses, related themes routed to best fit — ALL SIX BUILT, so every dungeon
   ends in a bespoke fight and spawn_authored never falls through to a stand-in for a real
   theme: (1) CAVERN TYRANT (cave/crystalcave/darkdepths/saltmine) — erupting STONE SPIKES
   (bite on the rise, then stand as no-go pillars) + a GROUND-SLAM rubble ring; (2) STORM
   HERALD (stormspire/windbarrow) — a LIGHTNING STORM of telegraphed columns (skystrike) on
   your mark + chain-bolt fans; (3) MUMMY KING (tomb/ossuary/hollowroot) — RAISE THE DEAD:
   channels untouchable, then skeletons claw from grave-mounds + a Slowing curse bolt; (4)
   DREAD KNIGHT (castle) — pure melee reads: a SHIELD CHARGE dash + a wide GREATSWORD SWEEP
   arc; (5) ROT HORROR (searuin/tarpit/blightvault) — SPREADING BLIGHT: poison pools that
   carpet the floor + poison bolts; (6) BROODMOTHER (ruins/bellbarrow) — SNARE + SWARM: sticky
   web patches + capped spiderling streams + a Slowing spinneret shot. Each: 3 phases, enrage
   per third, reward banked by the arena, boss bar + name-splash. All under boss/ as self-
   contained modules (art + component + spawn + tick + deaths), registered in BossPlugin
   (nested-tuple groups) + routed in spawn_authored. Reused infra: EBolt, skystrike, mob_bundle
   summons, Afflicts(slow/poison), Statuses pools, spawn_burst. **CRITICAL BOOT-FIX shipped
   here:** milestone 65's boss name-splash added a second `ResMut<Banners>` to dungeon
   `navigate` — but SwapCtx already carries one, so the schedule failed to build and the game
   PANICKED AT EVERY BOOT (B0002). clippy + tests DON'T build the schedule, so m64/65/66 stayed
   green while the port had silently not-booted since m65. Fixed by using swap.banners. LESSON
   (now enforced): any system/plugin/SystemParam change gets a BOOT SMOKE, not just clippy+tests
   — `WRIFT_SHOT=1 ... ./target/release/wriftheart` (clean exit + a >50KB PNG). BOOT VERIFIED
   after each boss batch (clean exit, 0 panics, 105 KB render). clippy 0 + 16 green + boots.)*
66. **TIERED HARVEST — the pick/axe ladder, node gates + tiered drops (js TOOL_METALS +
   ORE_LADDER + reqTier)** — *(done — deeper lands now demand, and reward, better tools.
   TEN new TOOLS (iron/silver/gold/mithril/voidsteel × pick+axe, js TOOL_METALS t2..6): the
   shared pick/axe head RECOLOURED by the metal's overlay (icon + swing) — which also closes
   the flagged "material-recolored swing sprites" tail item. ItemDef gained `tool_tier` +
   `tool_mat`; the recoloured swing sprites are pre-baked once into AttackArt.tiered (keyed by
   item id) and selected at swing spawn; Swing/AttackTool carry the tier. NODE GATES: every
   ore/tree node now carries a `req_tier` from World::zone_tier (js harvestTier) + a `tier` for
   its drops; resolve_combat's tool gate rejects a too-weak head (right tool, low tier) exactly
   like the wrong tool — and a Tinked `note` surfaces the "NEEDS A STRONGER PICK/AXE" toast
   (js resistTool). TIERED DROPS: gather.rs ore_at_tier/wood_at_tier (ORE_LADDER copper→
   voidsteel, WOOD_LADDER hardwood/ironbark/voidwood at T3+); a rock drops stone + ~35% its
   zone ore, a tree wood + ~30% its zone timber; the boulder VEIN ART climbs copper→mithril by
   tier. THE LOOP CLOSES: the forge's ironpick..voidsteelpick recipes (already extracted, now
   RESOLVING because their outputs + the tiered mats are registered) let you forge a better
   pick from the ore a lesser pick just mined. inventory.remove_one bool from m65 stayed.
   clippy 0 + 16 green. Only recipe fill-ins still hiding: `sleepingbag` + `well`. NOT
   shot-verified — playtest: mine an arctic/deep vein with a weak pick (it tinks + toasts),
   forge the metal pick, mine it.)*
65. **DEVIATION TAIL, part 1 — SLIPPERY ICE + BOSS NAME-SPLASH** — *(done — two of the
   flagged small deviations closed. SLIPPERY ICE (js footG=='ice' && !paved, deferred from
   the weather pass as terrain physics): the hero now carries velocity (Player.vx/vy) on
   arctic ice — builds toward the input slowly (grip 0.09), coasts to a stop when released
   (friction 0.025), a wall kills the slide on that axis — exactly the js momentum model.
   The "on ice" test reads world.ground_name at the foot tile == "ice", excused by a road/
   processional/street/bridge deck laid over it (grid.code_at in '=','p','_','B'); overworld
   only. BOSS NAME-SPLASH on arena entry (js drawBossName, was STILL-OPEN from the boss
   milestone): when the boss doors slam shut (navigate's arena-lock), the guardian ANNOUNCES
   itself — a new Banners::boss(name) raises the big-letters town-slot splash with a
   "- IT GUARDS THE SHARD -" sub-line; the name is the boss's own BossName for THE TEN (the
   arena query now reads Option<&BossName>), a generic "THE GUARDIAN" for the elite stand-ins.
   inventory.remove_one now returns bool (home-craft needed it; harmless elsewhere). clippy 0
   + 16 green. STILL DEFERRED (not cheap tail): material-recolored SWING sprites are the
   tiered pick/axe arc tint (js aSwing/pSwing bake with m.ov) — DONE in milestone 66 (the
   tiered tools landed); per-rune WAND ICONS (js wandTipIcon) is a gem recolor across ~9 icon-bake sites with
   no central helper — cross-cutting for little gain; the LOOTGOBLIN CROSS-ROOM chase (js
   relocates an escapee with a saved real-time deadline) is a saved-state mini-system, its own
   pass. NOT shot-verified per "save the token" — playtest ice on an arctic screen + a boss
   arena entry.)*
64. **HOME STORAGE CHEST + THE BUILDABLE HOME (js playerHouse + drawStorage + homeCraft)** —
   *(done — a two-pane bank AND the home it lives in. The STORAGE window (app/storage.rs,
   its own Screen::Storage like the shop): left pane BAG, right pane CHEST, A moves the
   selected WHOLE stack across, LT/RT (or left/right) switch sides, B closes + saves.
   PlayerStash (Vec of {id,qty}, STASH_CAP 24, saved) with add/remove_one/count; unique
   items refuse storage, stackables merge, the cap holds. DISCOVERED mid-build: the storage
   chest only lives in the "house" interior def, which spawns for the player's BUILT home —
   never ported — so storage was unreachable. So the BUILDABLE HOME came with it (app/home.rs,
   the noted crafting fill-in): the `house` item (kind STRUCTURE) is crafted at a workbench,
   placed at your feet (the cooking-fire idiom — one home per save, a second RELOCATES it),
   re-stood whenever you enter its room (house_wake), and its door (added to interior.rs's
   candidate list) opens the "house" interior — the one with the BED (sleep, already served)
   and now the CHEST (services.rs serves "storage" -> Screen::Storage). The world sprite reuses
   the town "home" front (PropArt.fronts). PlayerHouse ({room,x,y}) + the stash ride the save.
   HOME-CRAFT POOL wired (js homeCraft): while inside your house the CRAFT page counts + spends
   chest materials too (bag first, then the chest for the remainder; chest draws are never
   spared by the CRAFT keystone) — threaded stash + a home flag through craft_tab + the slideout
   like `learned`; inventory remove_one now returns whether it fired so the fallback works.
   DEVIATIONS (flagged): placement is front-facing place-at-feet (js ghost-placement + rotation
   deferred); v1 DEFERS pack-up (removing the home for a mat refund) + the respawn-at-home warp.
   clippy 0 + 16 green; NOT shot-verified per "save the token" — playtest: craft house at a
   workbench -> place -> enter -> use the chest + craft from it.)*
63. **CRAFTING STATIONS + BLUEPRINTS — the whole forge/bench chain (js craftTable +
   RECIPES + learnedBlueprints)** — *(done — the Cooking Fire was the port's only
   placeable station; now the FULL crafting tree stands up. Extracted all 106 recipes
   (tools/extract_recipes.mjs -> src/recipes_data.rs, GENERATED) with their station +
   blueprint gate + output count; items::recipes_for(station, learned) filters to
   REGISTERED outputs + LEARNED blueprints (the fill-in-as-it-ports rule, same as loot).
   NEW ITEMS: 7 STATION kits (workbench/forge/alchemy/enchanter/fletcher/jeweler/
   farmtable) + 19 BLUEPRINTS (9 station/tool bps + the 10 js RECIPE_BPS "Recipe: X"
   schematics) + the 9 tiered-harvest mats the recipes cost (silver/gold/mithril/
   voidsteel ore reuse the copper-nugget shape retinted; hardwood/ironbark/greenheart/
   petalwood/voidwood reuse the log shape) — iron + gem were already in. FLOW: the
   workbench is a HAND recipe (bootstrap — craft it in your pack, place it), every other
   station is workbench-crafted behind its blueprint; a bp* item routes through the new
   app/blueprints.rs (LearnedBlueprints resource + LearnBlueprint msg, saved), veto-
   consumed only if newly learned. play.rs use-item now routes ANY kind=="STATION" to the
   cooking place-at-feet handler (generalised: per-kind art + place message live in the new
   app/station_art.rs — a shared oak-table body under a themed centrepiece, forge as a
   stone furnace) and any kind=="BLUEPRINT" to LearnBlueprint. THE FORGE COMMISSION (js
   craftGen) is fully wired: the craftw*/crafta* preview ids resolve on demand via
   procgen::preview (a leaked, cached "Rare Sword (Rolled)" def carrying a craft_gen
   marker); crafting one calls procgen::generate_pinned, which REJECTION-SAMPLES entropy
   so the seed's first rng draw lands the wanted base/slot — the rolled `~` id still
   round-trips through resolve. Learned blueprints ride the save (SaveData.blueprints).
   DEVIATION (flagged): stations place FRONT-FACING at your feet only — the js ghost-
   placement + 4-facing rotation + home-interior placement stay the crafting-overhaul
   deferral (consistent with the Cooking Fire). REMAINING fill-ins (recipes hide until
   their output ports): the well (bpwell-gated placeable), house/coop/barn buildables,
   sleepingbag, and the tiered pick/axe tools (iron/silver/gold/mithril/voidsteel) — all
   belong to the tiered-harvest + home systems. clippy 0 + 16 green (added forge_previews_
   resolve + commission_pins_the_base to procgen tests); NOT shot-verified per Baz "save
   the token" — he playtests the place/craft loop.)*
62. **THE MOB ROSTER — 13 goblin-stand-ins ported (js enemies.js)** — *(done —
   the deeper-biome/dungeon mobs that had been spawning as plain goblins now stand
   up REAL, with their own art + AI. Extended tools/extract_mobs.mjs for their
   sprite grids (regen: 52 mob-art kinds); added EIGHT new data-driven Ai variants
   + interpreters to the one mob_think table: RingBurst (mirefly 6-way venom /
   palehowler 8-way slow — the bolt volley grew an `afflict` that clings its status
   to the player it strikes), PhaseClock (bellsnail sealed-then-out / boglight
   solid-then-faded, invuln + harmless off-beat, `invert` flips which half is
   active), GazeStalker (saltstatue — stone while watched via the player-facing dot,
   stalks the instant you look away), Summoner (gravewarden raises skeletons — a new
   Summon MobAct), Strafer (tidecrab circles + pincer-lunges), OrbitDart (honeydrone
   orbits + darts when your back's turned), Suction (sandmaw DRAGS you in), SkyCaller
   (stormcaller — a telegraphed app/skystrike.rs bolt lands under your feet), Swapper
   (switchshade trades places with you). The three player-manipulating mobs route
   through DEFERRED effects applied after the mob loop (a mob can't hold &mut Player
   mid-iteration); pface (the hero's facing) is threaded into mob_think for the gaze
   checks. cultist maps to the existing Caster (fan 1 + kite), vinesnare to Dormant
   (a stationary snare). Two-state art (vinesnare/bellsnail/saltstatue) + boglight's
   phase-alpha select by state in sync_mobs. SHOT-VERIFIED (WRIFT_SHOT=newmobs):
   both batches lined up with distinct sprites + live AI — switchshade SWAPPED the
   harness hero into the mob line and killed it, the behavioural proof. FLAGGED:
   gravewarden's live-minion cap is paced by cooldown (the interpreter can't count
   the room). ROSTER NOW COMPLETE — the last two (hardest) also landed:
   GLIMMERLING (Ai::Glimmer — flees, pops a spark BURST when crowded, locks a
   telegraphed light-BEAM at mid-range with a real dodge window) and WITHERHEART
   (Ai::Drainer — a slow HOMING drain-orb, poppable with a swing since it carries
   Health 1, plus SELF-HEAL that regrows its wounds when left alone, pain
   resetting the timer — the regen rides ai.rs, which owns Health). Their
   bespoke attacks live in app/mobfx.rs (Burst / Beam / DrainOrb + ticks).
   Every biome/dungeon roster kind now spawns its REAL mob — no goblin
   stand-ins remain.)*
61. **WEATHER DEEP-SIM (inc 2 — the gameplay effects, js Weather.slows/isRaining)** —
   *(done — inc 1 (engine + visuals + lighting) was already in; inc 2's gameplay
   hooks are now ALL live. The last piece: TRUDGING — a blizzard or sandstorm that
   has really come in (heavy snow/dust, vis > 0.5) keeps the SLOW status topped up
   while you're outdoors (js `addStatus('slow', 6)` each frame; sheltered indoors +
   underground), and the movement code already halves a slowed hero. The other
   inc-2 effects had landed with their own systems and are confirmed wired: RAIN
   WATERS every tilled tile in the room (farm.rs, every 16 frames while raining),
   RAIN STOPS FIRE spreading + douses standing flames (fire.rs), and FISHING reads
   the LIVE weather (fishing.rs passes weather.cur to roll_fish — the weather-gated
   rainfish/voidfin now bite). SLIPPERY ICE (js footG=='ice' && !paved — sliding on
   frozen ground) was TERRAIN physics, not weather, so it landed later in the deviation
   tail — see milestone 65 (DONE). Clippy 0, 16 suites.)*
60. **THE TRAVELLING TRADESMAN + MAPBOTTLE (js tradewagon/caravanStock + fish junk)** —
   *(done. MAPBOTTLE: ~1.4% of fishing casts (10% of the 14% junk) the deep gives up
   a BOTTLED MAP (js rollFish) — read like any treasure chart via the existing ReadMap
   route; the fish-junk test grew mapbottle. THE CARAVAN (app/caravan.rs + stock.rs +
   shop.rs): worldgen already seeded a `tradewagon` on rare wide roadside patches
   (~1.5% of non-start rooms) but nothing stood it up — now a wake system (the
   mound_wake idiom) renders the cart (js WAGON art) + a seeded shopkeeper and drops
   a blocker under the wheels; walk up and INTERACT and shop::open_caravan populates
   ShopState straight from stock::caravan_stock (js caravanStock: 6-8 MATERIAL wares
   in small stacks, the finer stock — iron/silver/gold/mithril/voidsteel, hardwoods,
   silk, gems — only riding with caravans out in the deeper lands, registry-filtered
   + seeded-stable per site, sold-today ledger keyed on the wagon's room). The shop
   SCREEN is driven by ShopState (not InsideState), so the caravan reuses the whole
   buy/sell window with no interior. SHOT-VERIFIED (WRIFT_SHOT=caravan): the cart +
   shopkeeper standing in the wild. Clippy 0, 16 suites.)*
59. **THE PROCEDURAL ITEM GENERATOR (js generate/genItem/genWeapon/genArmor)** —
   *(done — src/procgen.rs, the Diablo-style gear engine, the LARGEST remaining
   parity gap. Every weapon/armor is a rolled item: base archetype (6 weapons /
   6 armours) x material ramp (7 metals + 5 leathers, stat mul + icon recolour)
   x quality (0.9-1.25) x affixes (up to 2 prefix + 1 suffix by tier), packed
   into a stable `~<base36>` id (low 3 bits rarity, bit 3 kind, rest entropy).
   An id is a REGENERABLE handle: resolve() decodes it, rolls the def with the
   js Mulberry32 stream (the rs rng matches genRng byte-for-byte) seeded off the
   id, and LEAKS it to 'static behind a cache (the js REGISTRY) — same id -> same
   item forever. items::get resolves `~` ids through procgen. WIRING: roll_loot
   SUBSTITUTES every common..epic weapon/armour drop with a fresh roll (fixed
   gear defs survive only as shop staples + forge craftables; legendaries/
   consumables/trinkets keep fixed defs); gear vendors (blacksmith/fletcher ->
   weapon, armory -> armour) roll 4 wares up to the zone tier; wild shops roll
   their weapon/armour category slots. Generated WEAPON combat stats (dmg/crit/
   critmult/knock/leech) ride the def's `stats` array, read by play.rs's swing
   branch — which also grew a SwingBonus component + swing_bonus_hits so bonus
   knockback AND lifesteal land on the strike (resolving the "leech banked"
   gap for gear leech too). Generated ARMOUR stats are worn-gear stats the
   pipeline already sums, and each piece carries a leaked ArmorLook so
   worn_refresh renders it ON the hero (js def.armorLook) — no static table
   entry. The item detail pane gained the wstats/stats line (DMG 4 / +2 ARMOR).
   Guarded by procgen tests (id round-trip + cache identity, wearable armour +
   worn look, rarity/kind bit decode). DEVIATION (flagged): the in-hand SWING
   sprite uses the tool's default art, not a material-recoloured blade (the
   ICON is material-tinted); forge craftGen previews join the crafting-station
   port.)*
58. **VICTIM MORTALITY (js encounters.js victim deathEffect)** — *(done —
   completing the encounter civilians (they already fled + shouted; they were
   INVULNERABLE, flagged). Each victim now carries a combat body: Team::PLAYER
   so enemy attacks strike it while YOUR swings (hurt_team Enemy) pass harmlessly
   through — you protect the civilian, you can never cut it down. Health 8,
   HurtProfile invuln 48 (js's generous i-frames so a swarm can't delete it
   instantly) + flash 10, Blood; the victim tick syncs its Hitbox to its bespoke
   x/y each frame so resolve_combat can target it, and sync_enc_people blinks it
   on the hurt frame. A slain victim (Health 0) runs victim_deaths: a blood
   burst + a corpse in a pool of blood (RoomActor, clears on room change, js
   deathEffect), then it's gone. This RESOLVES the flagged deviation on the
   encounter victims. Clippy 0, 16 suites, boot smoke clean.)*
57. **THE LOOT GOBLIN (js enemies.js lootgoblin — the runaway treasure mob)** —
   *(done — app/lootgoblin.rs, a bespoke mob (ogre pattern): a gold-recoloured
   goblin (js GOLDPAL q/Q -> gold over the shared goblin frames), HARMLESS
   (damage None), hp 10, that BOLTS from you. Its flee is the js evasive juke:
   spooks within 150 ("CANT CATCH ME"), circles the OPEN MIDDLE for a ~7s grace
   (steering off whatever edge it nears with the MRG-30 nudge so it doesn't
   escape straight off), speed 1.7 + a sine jink, and only AFTER the grace does
   it bolt for an exit and slip out an edge — banished, gone with the gold, no
   XP/drops. Every hit sheds a spray of coins (3-5 at 2-5cu, keyed on the
   hp-drop) and a 12% trinket; a real KILL is the jackpot (45-99cu + a scatter
   of coins + a 50%-x-luck rich loot roll at boost 1.6 + 20 XP). Wired into the
   existing worldgen roll (entities.rs already seeds "lootgoblin"; battle/mod.rs
   now stands up the real thing instead of the goblin stand-in). SHOT-VERIFIED
   (WRIFT_SHOT=lootgob): the gold goblin mid-flee beside the hero. DEVIATION
   (flagged): the js RELOCATES an escaped goblin to the next room with a saved
   real-time deadline (chase it across rooms); the rs port ends the chase at
   the edge — it simply gets away. The cross-room persistence layer joins a
   later pass.)*
56. **ELITE NAME TAGS + PACK-AFFIX PROJECTION (js eliteName + setPackAffixes)** —
   *(done — champions.rs, extending the existing champion/elite/6-affix machinery.
   NAME TAGS: each elite gets a floating "AFFIXNAME AFFIXNAME BASENAME" label (js
   affixName + BESTIARY name via a new mobs_art::bestiary_name), baked once and
   trailed centred above the elite-scaled body, dying with its owner. PACK-AFFIX
   PROJECTION (js setPackAffixes): while ANY promoted leader lives, the union of
   every live leader's projectable affixes lands on the room's non-leaders — the
   packApply values (venomous Afflicts poison 130, chilling Afflicts slow 100,
   vampiric Lifesteal, volatile AffixVolatile, swift speed x1.3, toughened
   defense +1) — and the moment the last leader falls, a PackProjected snapshot
   reverts EXACTLY what was lent (base speed restored, added components removed,
   defense un-bumped), no max-HP ever borrowed so it reverts clean. SHOT-VERIFIED
   (WRIFT_SHOT=elite): two elites with ground auras + legible name tags
   ("VAMPIRIC VENOMOUS SKELETON ARCHER" / "...VENOMOUS SPITTING SPIDER") — which
   promptly killed the 3-HP harness hero, the behavioural proof. DEVIATION
   (flagged): the js also tints a faint pack-aura under each borrower; that
   visual joins a later polish pass.)*
55. **blockShotsOnProps — props stop shots (js game.js)** — *(done — a
   projectiles.rs pass despawns any straight SHOT (player arrows + non-fire spell
   bolts, enemy arrows + caster bolts + webs) whose hitbox enters a SOLID PROP —
   the same RoomBlockers rects that stop your feet (rocks, trees, bushes,
   buildings). FIRE bolts are excluded: app/fire.rs already stops them ON brush
   (igniting it first), so filtering them here keeps a firebolt from dying a tick
   before it can light the world. The boomerang/grapple claw/Kingsplitter beam
   are hand-flighted weapons with their own stop rules, untouched. This RESOLVES
   the deviation flagged in the bow (46) + wand (52) milestones — shots no longer
   sail through bushes. Clippy 0, 16 suites, boot smoke clean.)*
54. **GRAPPLE HOOK + SPRING BOOTS (traversal utilities)** — *(done. Both are
   player-MOTION states carried ON the Player component (js p.grapple / p.hop),
   slotting into tick's movement chain right after knockback + before the tongue-
   reel: a fresh hit clears them (js onHurt), and take-and-run copies keep them
   out from under move_axis's &mut. GRAPPLE HOOK (app/traversal.rs, epic GADGET,
   cd 40): a press fires the claw forward at 5.5 along the 8-way aim; biting a
   WALL TILE lodges it and sets p.grapple {tx,ty,t:36}, and tick reels the hero
   FAST (sp 5, near 4) until he arrives, the timer dies, or he wedges — with a
   taut rope drawn hand->claw every tick (stays attached while walking, js), a
   150px fizzle on a miss. SPRING BOOTS (rare BOOTS, cd 26): a press sets
   p.hop {sx,sy,tx,ty,total:13} — a lerp bound 30px forward, but only if it
   LANDS somewhere clear (a wall in the target scuffs with a tink); the arc
   lifts the sprite by hop_z = sin(t*PI)*10 (js drawHero's hopЗ). Both ride
   the registry rule into their js shops (fletcher/misc pools) + hand recipes
   (js gates them behind the workbench/fletcher blueprints — flagged, hand for
   now). Loader clears grapple/hop/hop_z on every world swap alongside the
   knockback clear. Dev panel: TRAVEL KIT (grapple + boots + bubble ring).
   SHOT-VERIFIED (WRIFT_SHOT=grapple): the claw fired right, the tan rope
   taut from the hero's hand across the field to the lodged claw. DEVIATION
   (flagged): the js hook also snags ENEMIES (reel a foe to you / you to a
   tree); the rs hook bites wall tiles only until entity-snag ports.)*
53. **SATCHELS + ANTIDOTE + BUBBLE RING (small item fills)** — *(done. SATCHELS:
   four tiered bag-row upgrades (js SATCHELS) — Small/Satchel/Large/Travelers Pack,
   each grows the bag to a set row count (8 slots/row) via the STRICT tier chain
   (only a bag exactly one row short grows — expand_bag + the satchel_target ladder
   in play.rs; a skip attempt just clicks). ANTIDOTE: cures poison + slow (a new
   CureStatus message to status.rs), consumed only if you were actually sick — the
   veto reads the same Statuses play.rs already holds. BUBBLE RING (epic trinket,
   `bubble` gear flag): app/shield.rs grows a Bubble resource that recharges over
   200 frames while worn (js bubbleTimer), pops to deflect ONE incoming projectile
   (ordered AFTER shield_block so a shield-blocked shot never also drains it),
   then recharges — with the faint cyan deflector sphere around the hero while
   charged (js drawHero). All three ride the registry rule into their js shops
   (general/alchemist/jeweler/trader) + hand recipes (satchels/antidote flagged
   as workbench/alchemy-table deviations until those stations port). Clippy 0,
   16 suites, boot smoke clean.)*
52. **WANDS + SPELLS + REAL FIRE (js magic: SPELLS/castWand/spellBolt + the
   wildfire loop)** — *(done — app/wands.rs + app/fire.rs. ONE unique wand
   (magic-shop 60c) casts its socketed rune's spell from an ability slot,
   auto-repeating while held; USING a rune sockets it and pops the old rune
   back into the bag (arcane is the bare default; the socket rides the save).
   The four spells verbatim: ARCANE BOLT 2 mana / FIREBOLT 3 (real fire) /
   FROST SHARD 3 (chill 150) / SPARK BOLT 4 (speed 7, PIERCES the line).
   Casting spends mana or FIZZLES — the MP bar flashes red (js manaFlash 16)
   and regen holds for 70 frames after a real cast (js castCool); mana
   potions (8) and elixirs (full) drink from a slot and veto at full. Bolts:
   spell-stat damage (the catalog's magerobe/magefocus/arcanesigil rows now
   BITE), crit fields, water-sail flight, glowing core + a fading trail of
   motes, colour bursts on death. Elements ride the SWING-PROC PIPELINE:
   frost carries ChillHit, fire ScorchHit — one affliction path for swings,
   procs, and spells alike. REAL FIRE (fire.rs): a firebolt touching
   flammable brush IGNITES it — grass lets the bolt streak on (js passFire),
   a bush or tree stops it in a burst; burning things wear flickering flame
   tongues, feed the lighting overlay (r36 blaze pools via BurningLights),
   and burn down on the js clocks (grass 40 / bush 70 / tree 220) into the
   NORMAL gather death — drops + the regrowth ledger, same as a chop.
   WILDFIRE: every 16 ticks each burning thing (props AND foes marked by
   fire or the Ember Fang) rolls 20% per flammable neighbour within a tile
   and sets touching foes alight; rain stops the spread AND douses standing
   flames (the target survives); nothing spreads indoors/underground.
   Dev panel: MAGE KIT. SHOT-VERIFIED (WRIFT_SHOT=magic): wand slotted, MP
   half-spent, the torched bush ablaze AND a neighbour caught by the
   wildfire roll on its own. GOTCHA (cost a hang): the wildfire's
   burning-foe scan must ride a READ-ONLY pass over the same foes query the
   ignition arm borrows mutably — two queries = B0001 at startup.
   DEVIATION (flagged): burning bushes stay solid until they fall; the
   wand's slot icon stays arcane-tinted; blockShotsOnProps (js props
   stopping player shots) remains unported for arrows and bolts alike.)*
51. **THE WORN ARMOR LOOK (js ARMOR_LOOK + bakeGeared + drawAccents)** —
   *(done — actors/hero.rs: equipped head/body/feet pieces render ON the hero.
   The js three-layer recipe verbatim: (1) bake_hero_geared recolours the
   placeholder chars — body armor over the outfit g/G, a COVERING helm over
   the hair dome h/H only (off-head j/J tails stay hair; a crown perches on
   top instead), boots over the shoe-leather 'e' AND the trouser-shaft 'd'
   so the recolour tracks every walk frame; (2) accent_pokes paints the
   style marks over the baked 16x16 — helm brow + dome glint, the miner
   lamp (forehead from the front, leading edge in profile, hidden behind),
   horns, crest ridge, hat brim, crown circlet + points, plate centre-seam
   + pauldrons, mail ring texture, stud rivets, vest/tunic belts, robe tie
   + hem, greaves knee plate; (3) LEFT mirrors the finished RIGHT at the
   IMAGE level (flip_image) so asymmetric accents stay on the leading edge,
   exactly the js F.left = flipH(F.right). ARMOR_LOOK: all 26 js rows.
   The BALD easter egg keeps its dignity: bald + covering helm falls back
   to 'short' so the helm has dome pixels to paint (js line for line).
   app/play.rs worn_refresh watches the worn trio and re-bakes HeroArt on
   change; the anim tick reads the bank every frame so the swap re-skins
   instantly. SHOT-VERIFIED (WRIFT_SHOT=worn): dragonhelm's red crest,
   platemail greys over the outfit, the greaves' knee-plate band — and the
   HP bar at 3/4 proving the catalog's stats live. FLAGGED: procedural
   armor pieces (it.armorLook) join the item generator.)*
50. **THE GEAR CATALOG (js GEAR + the armor/trinket tables — 48 wearables)** —
   *(done — tools/extract_gear.mjs -> src/gear_data.rs (GENERATED, the extractor
   pattern: Assets.bake stubbed to capture grids + palettes, both js tables
   probed — the direct define('id') calls AND the looped { id: '...' } rows).
   Ten HEAD (leathercap -> crownofvalor), nine BODY (clothtunic -> aegisplate),
   seven FEET (sandals -> sevenleague), and the full stat-row/gear-flag trinket
   shelf (powerring, critring, ironheart, titangrip, berserkertotem,
   assassinmark, warlordbanner, phoenixfeather, lodestone, manacrystal,
   magefocus, focuslens, arcanesigil, vampirefang, gamblerscoin, swiftcharm,
   regenring, luckamulet, greedcharm, vigorpendant, compass, LANTERN...) —
   icons, rarities, prices, stats verbatim. items::get CHAINS the generated
   table behind DEFS (all_defs() feeds the codex catalog + award totals), so
   the whole shelf resolves with zero hand-copied literals. LOOT_POOLS
   rebuilt to the js lists in js order, filtered to the registry (satchels,
   procedural weapons, springboots/grapple/bubblering stay commented until
   their ports). The registry rule converges the armory/jeweler/blacksmith
   shelves automatically. The stats pipeline (skills recompute + gear_stat)
   makes defense/maxhp/move/luck/coin/crit/critmult/leech/regen/magnet/
   iframes pieces LIVE ON EQUIP; the lantern's light flag now feeds
   lighting.rs. Guarded by gear_catalog_integrity (unique ids, 8x8 icons,
   wearable slots, prices, js spot-values). FLAGGED: spell/maxmana/haste/
   knock/conelight stat rows ride inert until their systems port; the WORN
   ARMOR LOOK (js ARMOR_LOOK hero overlays — cap/helm/vest/mail/robe/boots
   styles) is its OWN next increment; bubblering wires into shield.rs
   later.)*
49. **THE GUILDS + WRIFTHEART CODEX TABS (js drawGuildsDex + drawWriftheart —
   the last two stub pages go real)** — *(done — codex/guilds_tab.rs +
   codex/wriftheart_tab.rs replace stubs.rs for their TabIds; every codex page
   is now the real thing. GUILDS: every hall you've stepped into, richest
   restoration first — city rows with five crest pips (lit in the wing's
   colour once restored) and the n/5 tally, up/down browsing with scroll
   markers, and the right pane telling the selected city's five wings:
   RESTORED in crest colour, have/need progress in gold, each perk promised
   in gray and held in gold (js line for line). No halls keeps the '? ? ?'
   mystery. WRIFTHEART: the assembling heart — the js bezier heartPath
   CPU-SAMPLED into a polygon and rastered: dark crystal fill, progress-lit
   rim + inner bevel, the facet web faintly joining all ten sockets, THE
   WOUND (the jagged fissure + rift-light echo + two hairline side-cracks,
   knitting closed as prog rises), veins linking recovered shards, gold
   reliquary studs, and the sockets themselves — held shards glow as sprites
   in their land's colour, empty ones are dim diamonds already hinting at
   the colour they wait for. Arrows cycle the shards (gold selection box);
   the right pane is draw_pane with the shard's chapter; INTRO/FINALE runs
   beneath, purple once WHOLE. The shot harness's codex scene grew WRIFT_TAB
   (guilds|wriftheart) + seeded halls/relics. SHOT-VERIFIED: Brightmoor 3/5
   with The Scholars 3/7 in gold, and the broken heart at 2/10 — wound,
   web, sockets, studs, halo all reading. DEVIATION (flagged): the js
   light-show (god-rays wheel, radial aura pulse, jewelry shine-sweep,
   drifting motes, live heartbeat) is distilled to a static halo at
   mid-pulse; those layers join a polish pass.)*
48. **THE CRIT SYSTEM (js atk.crit/critMult + the resolveCombat roll)** —
   *(done — combat.rs CritChance component + the stat plumbing. The js model
   verbatim: an attack CARRIES its crit (chance + multiplier); resolve_combat
   rolls once per landed hit AFTER defense (js order: max(1, base - defense)
   -> round(dealt x critMult)) and the HitLanded message now says `crit` — the
   gold sparkle (js spark(), 0xfcd000) bursts from fx.rs on every critical.
   Sources wired: swings carry spec.crit (sword 0.05 / axe 0.05 / pick 0, the
   js intrinsics) + the tree/traits/gear crit stat + status crit (KEEN's 0.12
   pie buff and HUNTERS HOUR's 0.1 now BITE instead of riding inert); arrows
   carry the player stat (js arrow()); the multiplier is 2.0 + critmult
   everywhere. TreeStats banked `critmult` (PRECISION / EXECUTIONER /
   ASSASSIN nodes and the +5% CRIT DMG smalls now pay out; `crit` was
   already summed). The Keen Eye trait (+8%) rides the same s("crit") term.
   resolve_combat took ResMut<GameRng> for the roll (the js Math.random()).
   Verified: clippy 0 + 16 suites + a full-schedule runtime smoke.
   FLAGGED: spellbolt crit joins the wand port; procedural-weapon intrinsic
   crits (dagger 0.12, saber 0.10, rapier 0.12...) join the item generator;
   critring/gamblerscoin join the jeweler trinket batch.)*
47. **THE WOODEN SHIELD + BLOCKING (js 'shield' + player.blocks())** — *(done —
   app/shield.rs + the item/equip/save wiring. The shield: kind SHIELD, uncommon,
   price 120, dur 12, blacksmith staple (the stock table already listed it) +
   the js hand recipe (wood 5 + fiber 2). Equip it to an ability slot and HOLD
   the button: the guard raises (js p.blocking, recomputed every free tick),
   you walk at HALF SPEED, every swing is held, and app/shield.rs turns away
   EVERY incoming enemy projectile — arrow, bolt, web — ordered between the
   shot ticks and resolve_combat so a deflected shot can never also land. Each
   block wears the shield one notch (dur rides the InvEntry + a shield_dur
   save field, serde-defaulted so old saves load clean); the last block still
   blocks, then it SHATTERS: wooden splinters, the 'wood' crack, "YOUR SHIELD
   SHATTERS" (js addLoot line). The raised shield draws on the hero exactly as
   the js: a narrow steel-rimmed EDGE sliver at the leading arm for the side
   facings, the full boss-domed face low in front for down, tucked mostly
   behind the body for up. Blocks deflect from ANY side — the js code never
   checks facing (only the item desc claims "the front"); the CODE is what
   ports. Shields shelve in bag group 1 (the js sort slot) and ride the
   ability bar via the SHIELD equip arm. Dev panel: SHIELD row. SHOT-VERIFIED
   (WRIFT_SHOT=shield, injected test hold): guard raised facing right, edge
   sliver at the arm, an archer's arrow caught a pixel from the boss dome.
   FLAGGED: the Bubble Ring auto-block joins that trinket's port; the slot
   durability bar joins the HUD polish pass.)*
46. **THE BOW + ARROWS (js items.js 'bow'/arrow() — the ranged weapon)** —
   *(done — app/archery.rs + the item/pool/recipe/drop wiring. The bow: WEAPON,
   uncommon, cooldown 22 / lock 8, slot-press routed in play.rs beside the
   boomerang. The quiver pays FIRST (js use() veto): an arrow leaves the bag per
   shot, and a dry bag is just the tink click — no cooldown, no lock. Arrows fly
   the js arrow(): 8-WAY AIM (held movement keys for diagonals, cardinal facing
   as fallback — js aimVec), speed 4.4, life 70, damage 2 x (1 + melee) sharing
   the swing's stat term, hitbox 6x6, ONE bite (HitOnce + retired on the
   HitLanded that names them attacker), rotated to the flight line (shaft /
   bright head / twin fletching, the js draw), and they SAIL OVER WATER — only
   walls and rocks stop them ('~' passes). Sources at js rates: the fletcher +
   trader shelves converge via the registry rule (their tables already listed
   bow/arrow), bow joins the UNCOMMON loot pool, hand recipes bow (wood 3 +
   string 2, js) and arrow x5 (wood 1 + stone 1 — DEVIATION flagged: js whittles
   arrows at the workbench/fletcher stations, still on the backlog), Recipe
   grew an `n` output count (js outN) for the bundle, every felled foe may
   scatter 1 ammo (5% x luck, both death paths), tall grass hides a stray
   arrow (5%, js brush), and the skeleton archer already dropped 1-3. The
   Wispstone now swats ENEMY ARROWS from the air too (SwattableBolts grew the
   EnemyArrow arm — the js wisp always did). Dev panel: ARCHER KIT (bow + 20
   arrows). SHOT-VERIFIED (WRIFT_SHOT=bow): four arrows loosed in four facings,
   three caught mid-flight at the right rotations. FLAGGED: crit fields ride
   with the crit-system port; shield-deflection with blocking.)*
45. **THE OPENING CINEMATIC (js drawCutscene — every new game starts with the
   story)** — *(done — app/cinematic.rs: six scenes over ~24 seconds (LEN 1480,
   js CUTSCENE_LEN), fired by loader.rs on every FRESH start (js gameState
   'cutscene'), skippable with any menu key (skip presses are CONSUMED and the
   cinematic systems run before menu_tick, so the same press can't also open
   the pause menu/codex/slideout — the js had one input path, the rs port has
   many listeners). The hero is frozen underneath (ModeCtx.cutscene — the 16th
   field, AT the SystemParam cap). The six scenes on the js clock: the WHOLE
   AGE (the finale's heart art with the fracture healed shut, pulsing on the
   easy heartbeats at 10/50/90/130/170/210), the SUNDERING (three dying beats,
   then at t=400 the face swaps to the broken palette and ten relic-coloured
   shards burst outward — the 'hurt' crack lands on the js frames 400/406),
   TEN SHARDS TEN LANDS (the shards sink into the land strip), the CHOIR ON
   THE HILL (robed silhouettes, the warpCharge hymn swells at 750/852, the
   ember called down at 928), the COTTAGE WAKING (moonlit window, the hero's
   OWN down-idle sprite in bed, the 1010 thunder), and the VILLAGE ABLAZE
   (real TOWN_FRONTS art under fire glow). Era cards anchor the time-jumps
   (LONG AGO / THE SUNDERING 312 YEARS AGO / TEN SHARDS TEN LANDS / PRESENT
   DAY - THE YEAR 312 A.S., closing line THE WRIFTHEART MUST BE MADE WHOLE
   at 1300). Scene staging is STATE-BASED (Local staged/last_ph), not
   t==0-keyed, so the WRIFT_SHOT=cutscene scene can jump the clock anywhere
   (WRIFT_CUT_T). DEVIATION (flagged): the js scenes are full canvas
   paintings; these are DISTILLED compositions built from real game art —
   the finale's heart, the relic shard colours, the town building fronts,
   the hero's own sprite bank. FLAGGED: era-card alpha fades ride the
   despawn (no fade-out yet).)*
44. **THE FINALE — the gate knights, THE WRIFTHEART, and the mend (js finale
   arc)** — *(done — THE LAST BIG SYSTEM. THE GATE GUARDIANS (app/darkknight.rs,
   js darkknight): two towering knights in void-touched plate flank the Black
   Castle's path — slow 0.32 stalk because they never need to hurry, greatsword
   sweeps on the ogre's club-swing pattern (hitbox live for the back half,
   reach 14, damage 7), dread auras on the champions' ring machinery, hp 44 /
   def 2 / knockresist 0.9 at x1.3 scale. Both fall -> "THE GATE STANDS
   UNGUARDED" forever (CastleGuards rides SaveExtras). THE WRIFTHEART
   (boss/wriftheart.rs, bespoke — the riftlord x12 stand-in retires): the
   broken heart of the Whole Age, 60x56 rift crystal with THE FRACTURE jagging
   down its face, hanging at the hall's head on a slow, wrong bob. Its kit:
   THE HEARTBEAT (an arena pulse ring on the cinematic's dying drum — only the
   rim hurts, and ONE silent chime marks the way through), THE SHARD STORM
   (aimed 5-bolt glass fans), THE WOUND (void tears that open at the edges
   and STAY), THE CALL (voidlings pour from the crack, three at a time), and
   past the last third it HURLS ITSELF across the hall in shuddering lunges.
   hp 130 / knockresist 0.92, phases at 66/33 with thunder bursts.
   WRIFTBANE lands: the Kingsplitter's swings and beams carry the mark, and
   every marked hit bites the heart TWICE — the blade that broke it,
   finishing the work. The fall runs the existing finale flow ("THE FAR SIDE
   OF THE WOUND FALLS SILENT" -> the YOU WIN overlay) and now sets game_won
   (saved, js gameWon); pressing on from the win shows a small CREDITS card
   (an rs gift — js had none): MADE BY BAZ / EVERY SPRITE EVERY SONG EVERY
   SECRET / AND YOU WHO WALKED IT WHOLE. SHOT-VERIFIED: the knights standing
   over the harness hero at the castle path ("ALL THAT WANDERING, AND THIS IS
   THE REST."), and the heart mid-fight, boss bar up, glass in the air.
   FLAGGED: the knights' shouldered-blade idle overlay (folded into the swing
   flash), the opening cinematic (its own pass), gameWon bestiary card.)*
43. **THE HIDDEN SIDE-VIEW CHAMBER (js enterSideScroll — the gravity room)** —
   *(done — app/sidescroll.rs: the push-block secret's OTHER destination, a
   19x13 side-view chamber run by its own physics: gravity 0.34 capped at 5,
   walk 1.3, ladder climb 1.1 with the js mount/snap/step-off-only-onto-a-
   ledge rules (no jump needed — that's what makes it solvable for everyone).
   Climb to the ledge, open the SECRET CACHE (roll_loot 0.9 + 40-79 copper,
   once per dungeon — SideLooted rides SaveExtras), and climb out the exit
   stairs (press UP) to land back beside the shoved block. The chamber owns
   the frame while it's up: ModeCtx gates top-down movement, the droom
   re-stands on the way out through navigate()'s climb-out arm.
   DEVIATION (flagged, an improvement): js sends EVERY secret here; the rs
   port had already built hidden top-down VAULTS as its stand-in, so the two
   now SPLIT the secrets on a stable per-dungeon hash — half the mazes hide
   a vault, half the chamber. SHOT-VERIFIED: the hero grounded in the dark
   chamber, the ladder rising to the cache ledge. FLAGGED: springboots
   hop-jumping (the boots port), the js ambient tint [10,12,22] (the dungeon
   dark stands in), chest-open hint prompt.)*
42. **THE KINGSPLITTER QUESTLINE — the Saltmaze, its hymns, the Choirmaster,
   and the blade (js saltmaze arc)** — *(done — the generator's maze floors
   were parity-pinned all along; the runtime finally walks them. THE ARCH
   (app/saltmaze.rs): the half-buried salt door stands at the world's one
   Saltwastes site (worldgen row + generated 48x40 art: speckled salt-stone,
   the carved bell, the widening dark mouth, drifted heaps) — no map pin, the
   lore books are the map; press to descend (seed ^ 0x5a17b311, five maze
   floors). THE HYMNWORK: floor 2's DARK HYMNS were already wired
   (lighting.rs drowns the torchlight to 0.94); floor 3's CHANT now ticks —
   linger ~15s and the hymn crests ("THE HYMN RISES - MOVE ON" at 75%) and A
   ZEALOT ANSWERS at your side; floor 4's MIRROR HALLS run the js lost-woods
   rule in navigate(): in a mirror room every exit walks you back in unless
   your feet sing the Maze Song — N, W, E, S ("LEFT AND LEFT AND ROUND ABOUT,
   AND DOWN THE BELLS DARK THROAT IS OUT"), west-at-rest stays honest, a wrong
   turn resets the hymn. THE CHOIRMASTER (boss/choirmaster.rs, bespoke — the
   js template boss was scrapped with the rest): the floating hierophant
   whose head IS a bronze bell, pale eyes in its dark mouth; THE TOLL (he
   stills, the bell swings, a closing ring of 14-20 chimes rolls out — only
   the rim hurts), THE CHORUS (aimed salt-bright fans), THE CONGREGATION (two
   zealots at his side, replenished); crossing 66/33% quickens the hymn.
   Past him, NO reward chest — THE FIRST BELL'S ALTAR stands at the head of
   the sanctum through the whole fight, and the warp rune home stays SEALED
   until the blade is taken up (js rune.sealed). PRESS at the rested blade:
   THE KINGSPLITTER (legendary, unique; swings at 4 base, and AT FULL HEALTH
   IT SINGS — a piercing beam of light, 4.2px/f, whole-line HitOnce).
   SHOT-VERIFIED: the arch in the salt flats, and the Choirmaster mid-fight
   (boss bar + a chorus fan in flight — he'd already knocked one harness hero
   clean out the dungeon door). FLAGGED: the altar's blade-descend ceremony
   (it rests immediately), the chant hymn METER (toast stands in), the
   beam's wriftbane doubling (lands with the finale), beam trail glitter.)*
41. **CHAMPIONS, ELITES & THE OGRE (js makeChampion/makeElite + AFFIXES +
   enemies.js ogre)** — *(done — worldgen has rolled champ/elite flags onto mob
   rows all along (odds climb with distance, parity-pinned); the spawn path's
   bare hp-multiplier stand-in is replaced by the REAL js promotion
   (app/champions.rs). SIX AFFIXES: Venomous/Chilling (on-hit statuses via
   Afflicts), Swift (Mob.speed_mul — every stride scales), Vampiric (its hits
   on the hero mend it), Toughened (hp x1.6, +1 armor, knock resist), Volatile
   (the AffixVolatile death blast, sharing the emberling's spawner). A CHAMPION
   is hp x2.5 / +1 damage / one affix / a pulsing ground-ring aura tinted to
   it; an ELITE is TWICE the size (Mob.size_mul grows the draw from the feet +
   the hitbox x1.7 centred), faster, hp x4 / +2 damage / TWO affixes / DOUBLE
   loot. Fallen leaders bump the champions/elites ledger and cough up gear
   (roll_loot 0.5 / 0.8 x2). THE OGRE (app/ogre.rs, js-verbatim): the
   mini-cave roster's brute finally real — four hand-drawn facings (24px
   fronts, 28px profiles, flipped back view), the knotted CLUB a LIVE overlay
   at his fist (shouldered at rest, wound higher through the windup, hidden
   mid-swing), five states (prowl / trembling charge windup / the pounding
   3.4px charge / point-blank slam windup / rooted mid-swing), and the
   clubSwing sweep whose hitbox lands only on the downstroke with impact dust.
   His mini-cave roster slot spawns him real at x3. SHOT-VERIFIED: the ogre
   looming over the hero he'd ALREADY FLATTENED (the shot harness keeps
   donating its heroes to science). FLAGGED: pack-affix projection (the leader
   lending affixes to the room's lesser mobs), elite floating name tags,
   goblin-champ speed, the js ground-crack impact stroke, and THE DARK KNIGHT
   — the castle gate guardian rides the FINALE milestone now that his
   greatsword pattern (the club swing) is ported.)*
40. **UNIQUE TRINKETS batch 1 + THE GEAR-STATS PIPELINE (js NEW_GEAR)** —
   *(done — the foundational piece first: worn gear finally has STATS. ItemDef
   grew `stats: &[(&str, f64)]` and skills_tab::recompute folds
   items::gear_stat(inv, name) in beside the tree + traits (the js player.stat
   shape); a gear_refresh watcher re-sums on any bag/equip change. Salt Crown /
   Stillwater Pearl also reach MANA (gear maxmana raises the pool, manaregen
   feeds the trickle). Stats with no consumer yet (spell/haste/critmult/knock)
   ride along inert until their system ports — flagged.
   THE EIGHTEEN (app/uniques.rs, js-verbatim rows + icons): Ember Fang (scorch
   proc -> the foe burns, 1 per 30 w/ flash; grass ignition waits on the fire
   system — flagged), Winter Shard (chill proc -> the foe crawls at half
   cadence; the skip also slows its attacks a touch — flagged), Midas Tooth +
   Soul Locket (kill procs beside the volatile hook in deaths.rs: coin bursts /
   1-HP mends, luck-scaled), Bramble Band (thorns bite the attacker back),
   Grudge Purse (a snatchable coin spills when you're struck), Saint's Glass
   (+35% luck, +10% crit — SHATTERS FOREVER at the first blow, gone from the
   gear slot with a toast), Owl Talisman (HUNTERS HOUR all night), Wispstone
   (the orbiting grave-wisp: contact singe + it SWATS enemy bolts from the
   air), Warden's Knuckle (longer mercy frames), and the nine stat-row
   tradeoffs (Salt Crown / Grave Coin / Boar Heart / Hollow Bone / Rift
   Splinter / Bell Clapper / Stillwater Pearl / Harvest Knot / plus the
   defense rows) that need no code at all now that gear stats flow. THE
   WINDWOOD BOOMERANG flies (out 24 ticks at 3.4, homing back at 3.8,
   chilling both ways). HitLanded grew `attacker` (thorns + proc marks);
   swings roll their procs at spawn (chance = stat x (1 + luck), js formula).
   All the uniques joined the LOOT_POOLS at their js rarities (the pools'
   fill-in-as-it-ports rule). SHOT-VERIFIED (scene "uniq"): the grave-wisp
   orbiting the hero. FLAGGED: fletcher craft/stock for the boomerang, fire
   spread, golden-trinket variants + batch 2 (the memory's idea list).)*
39. **TREASURE MAPS & THE SHOVEL (js readTreasureMap + doDig + digMound)** —
   *(done — app/digging.rs. THE CHART (uncommon, 90c; the rare BOTTLED MAP
   rides along for the fishing-treasure wiring) reads under open sky only:
   an X lands on a clear, dry tile in a plain room 4-10 out (js roll —
   towns/castle/shard-dungeons/saltmaze/dupes skipped, 24 tries then "TOO
   SMUDGED" and the chart is NOT consumed; max 5 held). The codex world map
   pins a red X (and folds the room into frame beside the quest pins). The
   room grows the mound of disturbed earth at the spot — nudged clear of
   decor with the nudge SAVED (js rule: pin, mound, and dig agree forever;
   the hall_wake idiom). THE SHOVEL (24c, wood 2 + stone 3 — the flagged
   recipe stub returns) digs the faced tile via the farm.rs slot-press idiom:
   water swallows every scoop, walls and tilled soil resist, and an X within
   one tile of forgiveness pays 20 + tier*25 + rand20 copper, two
   roll_loot(0.25 + tier*0.3) drops, and a 15% follow-up chart — the trail
   continues. Ordinary ground coughs up scraps 8% of the time; every dig
   bumps the ledger. TreasureMaps rides SaveExtras; play.rs's use-routing
   writers got their own UseRoutes bundle (tick hit the 16-param cap).
   SHOT-VERIFIED (scene "dig"/"digmap"): chart-read toast, the
   TREASURE UNEARTHED payout (+72 copper + potions at tier 2), and the red X
   pin on the codex map. FLAGGED: mapbottle not yet in the fish-treasure
   table, mound glint/crumb anim frozen, trader-stock wiring for
   shovel/chart (the js trader pool lands with the wagon's stock pass).)*
38. **CRACKED WALLS & SECRET CAVES, part 2 — the cavern mobs and the SONG OF
   OPENING (js enemies.js + songstone arc)** — *(done — the four missing pool
   dwellers exist, js-verbatim stats/art/mechanics as four new Ai variants:
   PRISM SHARD (CrossTurret: rooted crystal, a turning cross of four bolts —
   an eighth-turn per volley so no angle stays safe), DEEP CRAWLER
   (SoundHunter: blind, it tracks your FOOTFALLS — stand still and it loses
   you; last-heard spot rides the mob scratch fields), EMBERLING (Fuse: a
   walking coal that arms at 30px, strobes HOT, and self-destructs —
   MobDef.volatile + MobAct::SelfDestruct; ANY death detonates it via the
   shared blast spawner, js blast(): R18, enemy-side, so it can't chain foes),
   ASH GEYSER (Vent: rooted, rains three arcing rocks around you —
   MobAct::Rocks reuses the hurler's ArcRock + shadow telegraph). All four
   verified in one WRIFT_MOB shot (a new harness hook: WRIFT_MOB="a,b,c"
   lines any mobs up in the dungeon start room). THE SINGING STONES answer at
   last: the carved monolith (js SONGSTONE_BMP) stands where worldgen has
   always placed it (2.5%/room, dest 70% cavern / 30% hidden shop); playing
   the SONG OF OPENING in its room splits it (flute.rs -> OpeningSung message
   -> caves.rs: mana spent only when something answers, js rule) into the
   two leaning halves around a dark stair-mouth — a CaveDoor underneath, so
   the descent/shop entry rides the part-1 machinery unchanged.
   OpenedSongstones rides SaveExtras; sung stones re-stand as doors forever.
   Scene "song" (WRIFT_TOWN="1,-9", WRIFT_SING=1). FLAGGED: the OGRE (the
   mini-cave roster's fifth kind) waits for the champions/dark-knight pass —
   its live club-swing overlay IS that pass's pattern; until then his slot
   falls back to an elite stand-in. Also flagged: songstone note-glow chase +
   hum anim frozen in the bake.)*
37. **CRACKED WALLS & SECRET CAVES, part 1 — bombs, cracks, cave doors, and the
   descent (js world.js placement + game.js arc)** — *(done — worldgen has been
   seeding `crackedrock` (7%/room, inner-ring wall tiles) and `songstone`
   entities all along (entities_parity pins them); the app layer finally
   answers. THE BOMB (js verbatim: 30 coin, stone 2 + fiber 1, cooldown 18)
   drops at your feet, fuse 75, then a 44x44 blast — NEW `Team::Hazard` in
   resolve_combat: it hurts foes AND you (the shot harness learned this the
   hard way) and shatters nodes past the tool gate (the js 'boom' rule). THE
   CRACK rides the GatherNode path (pick, hp 4, shake) as a fissure overlay on
   the wall tile; its death rolls the js hash (22% hidden shop / 38% mini cave
   / 40% underground cavern), records the door FOREVER (CrackCaves rides the
   save via the new SaveExtras bundle — SaveCtx is capped, so guilds/stations/
   caves now travel as ONE SystemParam), carves the arched cave-door mouth, and
   toasts a hint of what's beyond. PRESS at the mouth: 'mini' -> a 5-6 room
   cave with an elite MINI-BOSS standing in for the boss (DungeonRun.mini +
   the js MK roster ogre/golem/revenant/charbrute/icetroll at x3 — the rift-
   champion machinery reused); 'biome' -> a 1-2 floor underground cavern from
   the js POOL (crystalcave/fungal/lavatube/darkdepths/frostcavern — the three
   new cave themes were already in themes.rs) with a full boss, biome None so
   NO shard (bonus loot only); 'shop' -> the HIDDEN SHOP interior (the
   caveshop def was already extracted; door_enter's candidate list now includes
   shop-dest cave doors). Cave seeds are js-verbatim (base ^ 0xca5e3d ^ per-
   crack imuls); entrance_key "rx,ry:cx,cy" banks progress per crack. SHOT-
   VERIFIED (scene "cave", seed-1337 crack at 2,-9): the bomb beside the
   fissure, the growl toast, and the torch-lit mini cave entrance hall.
   FLAGGED for part 2: the five pool mobs still missing in rs (prismshard /
   emberling / ashgeyser / deepcrawler + the OGRE with its live club overlay —
   their spots skip today), songstones + the Song of Opening, the crack's
   low contrast on dark forest-edge walls.)*
36. **COOKING — the Cooking Fire, meals, and timed buffs (js items.js + tables)** —
   *(done — potions heal, MEALS BUFF (the js law). Nine dishes + the tonic land
   js-verbatim (ids/rarities/descs/icons; roast, hearty stew, spiced skewer,
   veggie saute, pumpkin pie, berry tart, grilled fish, fish chowder, anglers
   fry) + the rareherb material; ItemDef grew `dish: bool`; eating routes
   play.rs -> EatDish -> app/status.rs eat_dish, mapping each dish onto the
   already-ported buff DEFS (wellfed/guarded/mighty/swift/keen/lucky; the stew
   also cures poison; 90-120s js durations). THE COOKING FIRE is the port's
   FIRST placeable station (app/cooking.rs): craft the kit (stone 8 + wood 6),
   use it in the overworld wilds (towns/dungeons/interiors refuse) and the camp
   kitchen stands at your feet — stew pot, embers, steam — with the js
   table-base blocker; PlacedStations rides the save (threaded beside guilds).
   PRESS beside it -> the slide-out CRAFT page opens in STATION MODE
   (CraftState.station): the ten js cook recipes replace the hand list, with
   the "@FISH" wildcard (any fish counts; the cheapest is consumed first, and
   the CRAFT-save tree stat can't spare a wildcard fish). The Provisioners'
   wing now demands real COOKED DISHES (ReqMatch::Dish) — the fresh-milk
   stand-in retires. Scene "cook" drives the whole loop (larder -> place ->
   window; WRIFT_COOKCRAFT crafts, WRIFT_EAT proves the HUD buff row).
   BONUS: a new recipes_resolve test exposed FIVE latent recipe stubs whose
   outputs never existed (bow / shovel / bombs / shield / workbench — silently
   hidden by recipes_for's existence gate since the first craft port); each is
   now a flagged comment awaiting its system (ranged weapons, digging, cracked
   walls, blocking, the crafting overhaul). DISH_BUFFS is data + a wiring test
   (every dish has a buff row, every buff resolves in DEFS).
   DEVIATION (flagged): the fire is hand-craftable and places at the hero's
   feet — js gates it behind the bpcook blueprint + the workbench and uses
   ghost-placement mode; both join the crafting overhaul, as do the other
   stations (forge/alchemy/farmtable...) and recipe pins. FLAGGED: no ember
   flicker/steam anim (static bake), station sprite not js's woodTable rig.)*
35. **THE GUILDHALL — every city's community center (js increments 1-3)** —
   *(done — src/guildhall.rs (the five WINGS + bundle reqs as data, js verbatim;
   wings_are_sound pins every fixed item id) + app/guildhall.rs (per-city
   GuildLedger saved via write_save/collect/apply_to threading — SaveCtx sits at
   the 16-field cap, so `guilds` rides as a parameter; fresh starts clear it) +
   app/hall_exterior.rs (the town-side face). ENTER: the city's hall-corner
   "guildhall" entity finally RENDERS — the js staged draw ported fill-for-fill
   into a baked 124x88 image (boarded -> scaffolds (2-3) -> repaired (4) -> lit
   + pennant + window glow (5), crest pennants on the door line, GUILDHALL
   plaque via the shared font), stood up by hall_wake (the yard_wake idiom) with
   the js-verbatim body blocker; the js door zone (x+46,y+16,20x14) opens the
   hall as a lit, peaceful one-floor "dungeon" (gen has tagged gwing rooms all
   along; banner THE GUILDHALL; town music). INSIDE: each wing room stands up
   its DONATION ALTAR (solid, crest-lit once restored) via altar_wake;
   PRESS -> the checklist window (shop idiom, WINDOW layer over the play area;
   the hero freezes via ModeCtx.donate like the js guildDonate); E donates one
   matching bag item per line (kind/ids/rare-fish matchers), a filled bundle
   brings the guild home: toast, one-time reward (rare seeds / lucky hook /
   roll_loot(1.6) / a skill point / feast potions), all five -> THE GUILDHALL
   STANDS WHOLE + the guild seal. WIRED PERKS: Anglers (fish sell x1.5 in that
   city, threaded like the festival clock through sell_list) and Provisioners
   (the inn rests you free). SHOT-VERIFIED: the boarded face in Riverwick
   (seed-1337 city at -2,-12), the entrance hall, the provisioners checklist,
   and the full donate -> "THE PROVISIONERS RETURN TO THE CITY" arc (harness:
   WRIFT_SHOT=guildhall + WRIFT_WALK/WRIFT_BAG/WRIFT_DONATE plays the whole
   bundle). BONUS FIX: label() now floors its position — centred labels with an
   odd measure() landed on half-pixels and the canvas upscale sheared the
   glyphs everywhere. FLAGGED: the hall steward + desk (js:1582), tillers
   stall / smiths stock / scholars discount perks, cooked-dish req (FRESH MILK
   stands in until cooking ports), wing-room dressing per guild, face pop-in
   after room slides (the yard_wake limitation), pennant/glow sway frozen.)*
34. **RIFT SPIRES — the endless descent (js rift arc)** — *(done — the "rift"
   entities worldgen has been placing all along finally EXIST: a 44x52 tiered
   black tower (generated grid, glow seams + rimmed maw) painted in room_props
   with its solid mass, at every rift site (e.g. 19,0 / -19,0 / 0,-21 on seed
   1337). PRESS AT THE MAW -> floor 1: a fast riftvault mini-floor (1 floor,
   min(9, 4+depth) rooms, NO key-hunts — "speed is the rift's rhythm"; new
   GenOpts.rift types the deepest room a lockless Boss arena). Every foe is
   riftScale'd (js verbatim: hp x(2+0.25d), damage +1+d/4 capped 7) and the
   floor's CHAMPION is the theme heavyweight at x3 on top of the scale (js
   makeElite + riftScale; the affix system is flagged). THE CHAMPION FALLS ->
   the gilded purse chest, the warp rune home, a FREE depth-tiered roll_loot
   (boost 0.3+0.3d — THE endgame fountain, earned floor by floor), and the
   RIFT GATE: a jagged purple tear; touch it and the next floor regenerates
   deeper (depth folds into the seed) while the way home stays the original
   overworld doorstep. RIFT RECORD (js riftBest) rides the stats ledger as
   `riftbest` (zero save churn; toast on a new floor record). Rifts NEVER bank:
   entrance_key empty, both serialize_run sites guarded — every visit
   regenerates. Banner: "THE RIFT - FLOOR N". VERIFIED in-shot: the spire
   standing at 19,0 among its dead-tree court. FLAGGED: champion affixes, xp
   depth-scaling, the codex map pin for discovered spires.)*
33. **FARM ANIMALS — coops, hens, barns, and dairy cows (js both increments)** —
   *(done — app/farm_animals.rs + seven new items (js ids/prices verbatim:
   chicken 150 / egg 25 / cow 400 / milkpail 120 / milk 40; coop 120 / barn 250
   as PLACE-AT-FEET kits — flagged deviation until the blueprint placement
   system ports; the farm/produce/general stock-table entries that referenced
   them finally resolve). THE LOOP: raise a coop (4 roosts) or barn (3 stalls)
   on clear ground, release hens/cows beside their homes (js range + cap
   checks and every toast line), pet each once a day (drifting heart fx, cluck/
   moo from the audio bank, `pets` stat), and a petted cow + a pail in the bag
   = one milk a day. HENS LAY AT DAWN (js henLay verbatim): loved-since-last-
   lay hens always, neglected 50/50, absences capped at 3 — yard eggs are
   long-life pickups scattered by the coop. Idle AI is the js wander: pick a
   spot in the yard, dally, peck onward (waits/spans per species, clock-idle vs
   step frames, flip by heading), positions written back to the SAVED rows
   every step. Persistence: a Livestock resource (coops/barns/animal rows)
   rides SaveData + SocialCtx (fresh-reset wired). Yards re-stand idempotently
   on room entry, forced re-wakes after placements, and day rollovers. Art:
   js coop/hen/cow bakes verbatim; the vector barn redrawn as a 48px gambrel
   grid. Interact priority: pet_tick consumes the press before talk_tick so a
   hen underfoot never opens a chat. NOT YET: coop/barn crafting recipes (the
   blueprint path), egg/milk cooking (waits on the cooking pass).)*
32. **FESTIVALS — the year keeps its promises (festivals.js + game.js hooks)** —
   *(done — app/festivals.rs. The four fairs, one per 28-day season, all on DAY
   12: THE SEED FAIR (spring — the first town you enter gifts 3x3 in-season
   seed packets, itemget jingle), THE GREAT CAST (summer — fish sell for DOUBLE
   through the shop's sell list), THE HARVEST FAIR (fall — crops double the
   same way), and BELLNIGHT (winter — stand in any town past dusk, darkness >
   0.35, and the bells ring the `blessing` status over you until the day turns:
   +luck, slow mending, on the new status rig, with the bellring toll from the
   new audio bank). Towns DRESS for the day: two sagging bunting flag-lines in
   the fair's colour over the square (RoomActor — swept on leave, re-hung on
   entry). Once-per-day markers (js festivalSeenDay/blessedDay) ride the save
   (SaveData + SocialCtx grew a FestivalLedger; fresh-reset wired). The codex
   CALENDAR now names the season's fair with a countdown beside the day counter
   and rings its day in the fair's colour. sell_list/redraw grew a clock param
   for the price hooks. VERIFIED in-shot: Seed Fair day staged via WRIFT_CLOCK —
   bunting hung over Silvervale + both welcome toasts. NOT YET (flagged):
   festival folk with their own lines; the stats `festivals` counter feeds the
   Fairgoer award as-is.)*
31. **SOUNDTRACK VARIETY — five new loops (rs originals, Baz's ask)** — *(done —
   tracks.rs grew from the js three to EIGHT: BOSS (relentless A-minor sawtooth
   assault over a pumping sub-bass and a full war kit, 0.085s/16th — plays the
   moment ANY authored boss bar is up, in any dungeon), FINALE (an E-minor
   processional for the Black Castle: wide square lead, broken-chord arps, doomed
   roots), NIGHT (the overworld nocturne — long airy triangle phrases, no drums;
   day_darkness > 0.72), FROST (crystalline bells + deep cold roots for the
   arctic's open world), and DREAD (two grinding sawtooth drones + a voice that
   barely dares, with a lone heartbeat kick — graveyard/burnt/chaos lands). The
   picker priority: boss > finale > guildhall-town > dungeon > town > biome
   (frost/dread) > night > overworld. All five are authored in the same
   note-string sequencer and render into the same seamless wrap-around loop
   buffers; the sync test now pins all 24 voice strings to their tracks'
   sixteenth totals. Composed fresh — the js had no equivalents to copy; tune to
   taste on playtest.)*
30. **STATUS EFFECTS (status.js + player.js statuses)** — *(done — app/status.rs.
   The full 15-effect registry (blessing/hunterhour/waysong/poison/burn/slow/
   shock/warpcd/ward/wellfed/mighty/guarded/swift/lucky/keen) with names, colours,
   stat mods, and 10x10 pixel icons redrawn from the js vector glyphs (widths
   pinned in test). One Statuses resource (js p.statuses; add() = max-refresh);
   the tick runs js-exact: LIVE defense recompute every frame (tree defense +
   status defense — GUARDED and its expiry land at once), the DoTs (poison every
   36 / burn every 30 w/ flash-6, never landing the killing blow), and REGEN
   finally unbanked from the tree (heal every max(40, 300-regen*26) below full,
   buffs included). Movement runs the js line: SLOW x0.5, SHOCK x0.3, +move
   buffs, floored at 0.4; melee buffs scale swing damage. ON-HIT DEBUFFS: new
   combat::Afflicts component forwarded through HitLanded (players only) —
   MobDef grew an afflicts field: scorpion venom (200), frostmite/icetroll chill
   (70/90), sporeling/myconid spores (120/150), spider WEBS mire (110) — the js
   values, live for the first time. MIGRATIONS: the bespoke Slowed resource
   retired (hive-queen honey, All-Eye slow-ray, mycelium carpet now speak
   status); the flute's Ward resource retired (wardsong = status "ward", +2
   defense); the minstrel finally plays his true WAYSONG (+move, gentle mending,
   1 min) on top of the mana refill. HUD wears an icon row (sidebar, blinks the
   last 3s). STILL OPEN: luck/crit application at their read sites, cooking-buff
   items, blessing/hunterhour sources (festivals/trinkets), warpcd display, the
   STATUS codex/menu tab, frostwyrm's slowing bolt.)*
29. **AUDIO — the game has a VOICE (audio.js port)** — *(done — src/app/audio/
   {mod,synth,tracks}.rs + the bevy "wav" feature. The sfx bus (sfx.rs) finally
   has its consumer: the js WebAudio synth re-implemented as PURE OFFLINE DSP —
   synth.rs carries tone() (phase-accum osc, WebAudio-exact geometric envelope
   ramps, exponential pitch glides), noise() (deterministic LCG white noise
   through RBJ biquads standing in for BiquadFilterNode), musicTone() (swell
   crescendo envelope), kick/snare/hat, and the flute note() voice. mod.rs bakes
   ALL ~37 js sfx recipes VERBATIM (swing/hit/enemyDie/.../bellring + the four
   flute notes at songs.js pitches) into in-memory 16-bit WAVs at startup
   (AudioSource{bytes}), and play_sfx plays every bus key fire-and-forget. MUSIC:
   tracks.rs copies the three authored loops note-string-for-note-string
   (overworld dark-epic march 128x0.15s, dungeon vamp 128x0.09s w/ full drum
   line, town pastoral 64x0.16s) and renders each into a WRAP-AROUND loop buffer
   so note tails fold over the seam — gapless loops via PlaybackSettings::LOOP.
   Track choice per the js call sites: dungeon action underground (guildhall ->
   town when it ports), town music on town rooms, overworld elsewhere + title.
   duckMusic ported: itemget/songmatch/bellring push the music sink to the js
   0.05 floor and release. js gain staging (master .5 / sfx .9 / music .38)
   baked into the PCM. The flute now SOUNDS: live notes emit noteU/D/L/R on
   press (flute.rs). A music-theory test pins every voice string to its track's
   sixteenth total (a drifted string would phase the loop) + every note name
   resolves. DEVIATIONS (flagged): track switches are hard cuts; the held
   ocarina voice (noteOn vibrato ease-in) plays as the one-shot note(); the js
   blur/focus audio-suspend and the sound on/off setting are follow-ups.
   VERIFIED: staged boss fight ran with zero rodio/decode errors — dungeon
   track + combat sfx live. The real test is Baz's ears.)*
28. **THE TEN, bosses 6-10: the roster COMPLETE** — *(done — all ten shard-dungeon
   guardians are authored; BOSSES.md carries per-boss detail. In brief:
   BOSS 6 THE BRIAR QUEEN (petalhall, briar_queen.rs): rooted rose-monarch,
   untouchable in bloom — twin-arm petal-spiral bullet hell + thorn HEDGES that
   grow on a lattice and reshape the arena (solid, smashable, self-wilting — the
   maze can't lock); ROOTS surface on the floor and smashing one INTERRUPTS the
   bloom for a 300f wilt window (wilt art swap, droop + drizzle).
   BOSS 7 THE MYCELIUM THRONE (fungal, mycelium_throne.rs): the boss is the
   ROOM — five pustule NODES creep a SPORE CARPET tile by tile (HashSet + fading
   decals, cap 56); standing on carpet = Slowed + sporeling HATCHES underfoot
   (real mobs, cap 3); nodes erupt spore-rings when crowded; the network dead =
   carpet recedes and the bared throne spits 3-fans.
   BOSS 8 THE ASH TITAN (charhall, ash_titan.rs): a charcoal giant in THREE
   riding armor plates (head/chest/legs — separate hittable entities on offset
   anchors); core untouchable while any holds; every break = +speed and a new
   move (dash at 1 broken, slam nova at 2, MELTDOWN soft-and-fastest at 3); a
   burning WAKE of fire-trail decals (contact 1, cap 40) follows every stride.
   BOSS 9 THE UNMAKER (riftvault, unmaker.rs): steals the rules — its HEX
   MIRRORS the held d-pad for 240f spells (NEW play.rs Hexed resource, the
   Slowed pattern); blink-teleports instead of walking; at each third spawns
   FALSE SELVES (Health 1, no touch damage, orbit-drifting the hero) — the real
   one's eyes GLINT on a cycle, and wounding it scatters the court; permanent
   VOID TEARS fray the arena edges. Hexed clears on its death.
   BOSS 10 THE HOLLOW STAR (wriftvault, hollow_star.rs): REWRITES DungeonLights
   every tick — the vault's torches die and the ONLY light is its radiance, the
   hero's small lantern-glow, and four orbiting STAR SHARDS; shards chain
   CONSTELLATION BEAMS (quads + static damage motes at 25/50/75%) you thread in
   the dark; METEORS fall on telegraphed rings at your feet; every shard broken
   shrinks the light, and bared it drifts for you through novas in the deepest
   dark. Torches return with the room teardown.
   Verification: clippy 0 + 16 suites at every boss; briar shot-verified
   (bloom + spiral + hedge); throne/titan/unmaker/star smoke-verified panic-free
   with shots QUEUED on the flickering black-capture episode. Tuning pass:
   Baz's first playtests cover all ten.)*
27. **THE TEN, boss 5: THE ALL-EYE** — *(done — boss/all_eye.rs, the Bog's
   beholder. A lidded orb (22x20 shut/open bakes — closed lash line vs bared
   gold iris) drifting a lemniscate, untouchable while its FIVE EYESTALKS live.
   Stalks (12x14, each tinted by role) orbit the orb, each its own menace on its
   own clock: PULLER (reels you into the orb on the play.rs Pulled rig, pink
   ray), BOLTER (3-fan bog EBolts), SUMMONER (wakes real leech mobs, cap 2),
   SLOWER (90f Slowed beam, cyan), and the GAZER — a half-alpha warning line
   locks on for 30f and its snap punishes MOVEMENT (p.moving at fire = 2 dmg;
   dead-still = untouched; the freeze-tag teach). Beams are rotated quads on the
   tongue's transform math, endpoint-tracked live. All five plucked -> THE GREAT
   EYE OPENS: soft, faster, whole-orb gazes + 8-bolt iris novas + voidling-style
   BLINK teleports after every 6 damage soaked. js purse on death; stalk kills
   burst + clean up their beams. VERIFIED in-shot: shut orb + all five tinted
   stalks orbiting, bar up. Reach it: swamp dungeon (9,9) or WRIFT_BOSS=bog.)*
26. **THE TEN, boss 4: THE GLACIER MAW** — *(done — boss/glacier_maw.rs, the
   Frost Cavern's ice-worm. Fights from UNDER the floor: Burrowed (untouchable,
   hitbox parked, sprite hidden) its CRACK races at the hero's feet — jittered
   crack decals dripped every 5 ticks, melting on a fade — then ERUPTS where it
   caught you (30f star-crack rumble telegraph, then a burst + 8-bolt ice ring
   + the worm surfaces). FOUR SOLID ICE PILLARS (RoomBlockers) stand in the
   arena: bait the eruption within 22px of one and the pillar SHATTERS ON IT —
   210f stunned, defense -2, slumped sprite (the smart play, four uses max);
   otherwise you make do with the Surfaced window (slither + 14f lunge-bites,
   240f then it re-burrows). Every dive scatters hoarfrost sheen decals (visual;
   true slide-physics flagged in BOSSES.md polish). Crack speed grows per dive.
   spawn_room_boss/spawn_authored grew a RoomBlockers param for arena furniture
   (all call sites + WRIFT_BOSS staging updated). VERIFIED in-shot: crack trail
   run + eruption + surfaced worm over the shot hero it caught, pillars standing.
   Reach it: arctic dungeon (9,-3) or WRIFT_BOSS=frostcavern.)*
25. **THE TEN, boss 3: THE HIVE QUEEN** — *(done — boss/hive_queen.rs, the Hive
   Hollow's guardian. An 18x22 gold-banded queen on a lazy hover, UNTOUCHABLE
   while any of four wall BROOD COMBS lives (16x16 wax-cell bakes, Health 10,
   Object-style hittable): each comb hatches REAL wasp mobs (cap 3, HiveDrone
   marker, deliberately no DungeonFoe so hatches never bank); every comb smashed
   quickens her tempo (+18% each). Her ROYAL GUARD orbits her always: three
   drone sentinels in a four-slot ring whose EMPTY slot is the rotating gap —
   land blows through it or thin the ring for good (guards stay dead). Moveset:
   telegraphed dive-bombs THROUGH your position, 3-stinger gold fans, and HONEY
   SLICKS gobbed at your feet (480f pools; standing in one runs you at 55% via
   the NEW play.rs Slowed resource — built shared: the All-Eye and future frost
   effects ride it). js purse on death, court swept. VERIFIED in-shot: queen +
   orbiting guards + all four combs, bar up. Reach it: honeyglade dungeon (3,-3)
   or WRIFT_BOSS=hivehollow.)*
24. **SECRET PUSH-BLOCKS + HIDDEN VAULTS** — *(done — the runtime for place_decor's
   ~15% secret roll (DRoom.secret at tile 4,3), js game.js pushT flow verbatim:
   a slate PUSH-BLOCK (js pushBlock's exact colours as a bake grid) squats on the
   tile, solid via RoomBlockers; stand on the ADJACENT tile holding INTO it — the
   target tile must be clear (never onto a pit, js rule) — and after 48 grinding
   frames ("stone" every 12) it slides one tile aside, hidden STAIRS bake in
   where it stood ("warpCharge"), DRoom.secret_done persists (RoomSave field).
   DEVIATION (flagged in code + here): the js secret leads to a SIDE-SCROLL
   gravity room — that mini-engine is its own later milestone; until then the
   stairs drop into a HIDDEN VAULT: a sealed doorless themed room generated per
   secret at floorgen time (fl.rooms key = parent+(100,100), deterministic, rides
   the ledger + saves for free), holding the way back up + a cache chest with the
   js secret-cache roll (20-49 coin + boost-0.9 loot — VaultChest marker branch
   in chest_touch). navigate() grew a same-floor secret-hop branch (the (4,3)
   pad, banner "A HIDDEN VAULT" going down); push_block_tick joined the dungeon
   FixedUpdate chain; all four room-wake sites spawn the block/stairs/vault
   contents. The golden parity dump skips vault rooms (rs-only; the pass consumes
   NO rng so all js-derived lines stay bit-exact) + a new invariant test pins
   vault pairing/back-links/sealed doors. Verified: clippy 0, 16 suites, smoke
   run panic-free — in-shot verification queued behind the same black-capture
   episode as the hydra.)*
23. **THE TEN, boss 2: THE WARREN HYDRA** — *(done — boss/warren_hydra.rs, the
   Vine Warren's guardian and the roster's literal hydra. THE SHAPE: a rooted
   HEART BULB (22x20, bark shell art + peeled-open flesh art; the bar's Health)
   at the arena's middle + five fixed BURROWS ringing it. Vine-serpent HEADS
   (14x20, shut/bite jaws, three neck-segment sprites strung burrow->head, per-
   head Health 7 flat) rise from burrows (3 at the start): sway on their perch,
   REAR + LUNGE when you close (16f windup pulling back, 9f strike locked at
   where you stood, reach capped 48px, 5f hold, retract), and SPIT a seed bolt
   at range (EBolt, vine palette, dmg 1). Severed heads leave a pulsing SAP
   STUMP (Health 1, 210-tick timer): STRIKE IT -> the burrow is cauterized
   forever, sap burst, and the heart takes 3 straight through the bark (floored
   at 1hp — the sting can't finish it); LET IT TICK OUT -> the head regrows AND
   a second rises from a fresh burrow (five-burrow cap = escalation ceiling).
   The heart is unhittable while ANY head stands (invuln top-up, the mimic's
   dormant trick) and peels OPEN whenever none do — so the last severed head is
   always the choice: spend the stump window on prevention or on damage.
   Cauterize all five and it lies open for the finish. Breathing scale pulse
   (quicker + deeper when open), rooted (kb 0). Death: warren-wide wither (heads/
   stumps/burrows despawn + triple flesh burst), js boss purse, unseal via the
   emptied DungeonBoss query. Reach it: the greenmaw dungeon (room 3,3) or
   WRIFT_BOSS=vinewarren. Art-grid widths pinned in tests. VERIFIED in-shot
   (after the capture episode lifted): three heads live — one mid-STRIKE at the
   hero — burrow rings, neck segments, and the barked heart mid-arena.)*
22. **THE TEN, boss 1: THE BONE COLOSSUS (+ the boss framework)** — *(done — Baz
   SCRAPPED the js template bosses ("horrible": one 2x mob sprite + crown cycling
   six shared attacks); BOSSES.md is the new design constitution — ten bespoke
   bosses, one per shard dungeon, authored ONE AT A TIME with playtests between.
   FRAMEWORK (src/app/boss/): spawn_authored(theme) dispatch in spawn_room_boss
   (unbuilt themes keep the elite stand-in), BossName component, and the js boss
   HP bar ported as sprite rig (name + 168px fill + third-mark ticks, reddening
   per third; Local<BarRig>, cleans up when the boss query empties). Bosses are
   self-contained actors (mimic pattern, NOT MobDefs) carrying DungeonBoss so the
   arena seal + boss_loot + shard/rune flow in navigate() work untouched.
   BOSS 1 (bone_colossus.rs): 44x41 bespoke composed art (20px horn-browed skull
   w/ glowing sockets, shoulder-capped ribcage, chunky knuckle-dragger arms
   COMPOSED per cycle so a lost arm is really gone from the sprite; grid widths
   pinned by unit test). Fight: stalks + rib VOLLEYS (aimed EBolt fans, bone/glow
   palette) + grasp LUNGE dashes + telegraphed STOMP NOVA (22f shudder, then a
   16-24 bolt full ring); at each third of HP it COLLAPSES — bone-pile sprite +
   the SKULL flies free (orbit-flee AI, 3-spread spit, defense -2 = bonus damage
   window) for 7s, then REASSEMBLES around the skull one arm poorer and faster
   (armless cycle = lunging bites + the 24-bolt nova). Death: triple bone burst,
   30-69 coin, guaranteed potion, 45xp (js boss purse); the emptied DungeonBoss
   query lets navigate unseal + stand up rune/shard/gilded chest as before.
   WRIFT_BOSS=<theme> stages any authored boss in the shot harness. VERIFIED
   in-shot: bar + name up, colossus stalking, rib volley mid-flight converging on
   the hero (who ate one — 1/3 HP). NOTE for shot timing: this Mac runs Update at
   120Hz (ProMotion) so FixedUpdate fires every OTHER frame — WRIFT_SHOT_FRAME
   numbers are ~2x the fixed-tick count. STILL OPEN (BOSSES.md): bosses 2-10,
   boss sfx, MOBS codex boss cards. (Name-splash on arena entry: DONE — milestone 65.))*
21. **MIMIC CHESTS, redesigned (Baz's call: "it needs to actually trick people")** —
   *(done — the js mimic (enemies.js mimic + game.js 16% roll) spawned as a VISIBLY
   different chest sprite AT the treasure room's chest spot — obvious twice over.
   DELIBERATE DEVIATION, per Baz: (1) placement moved to generation (floorgen's
   post-pass, standalone hash off seed/floor/room — NOT the gen rng, so existing
   seeds keep their exact layouts, pinned by the golden parity test): ~5% of PLAIN
   rooms that hold NO real chest/key/secret grow one at a regular CHEST_SPOTS
   anchor (same table as real chests — part of the disguise), DRoom.mimic. So every
   treasure-room chest stays trustworthy, and a "bonus" chest in an ordinary room
   is either luck or teeth. Minis/bonus caves included (the js !mini gate existed
   to protect the mini's only chest — ours replaces nothing). (2) Shut = the real
   chest's EXACT bake/anchor/z (spawn_mimic beside spawn_chest), invuln topped to 2
   each tick (blades thunk off, js), damage None. (3) Spring on reach (js dist<30):
   flash 8, damage 3, MIMIC_OPEN/BITE_ICON chomp frames (items_art, CHEST_ICON-
   derived so lid+trim match), 1.9px hop-lunges re-aimed every 26f, knockResist
   0.5, hp 12xHP_MUL — mimic_tick handles brain/collision/kb/blink, mimic_deaths
   the fall: burst + 10-19 coin + 25xp + it COUGHS UP A REAL CHEST at its spot (js
   homeX drop), DRoom.mimic_slain (RoomSave field, serde default) banks in the
   ledger so teeth only bite once; the coughed chest re-stands on re-entry until
   looted. WRIFT_MIMIC=1 stages one in the dungeon shot scene. VERIFIED in-shot:
   dormant = indistinguishable chest at 2 tiles; sprung = maw/teeth frames over a
   very dead level-1 shot hero (3 dmg vs 3 hp — the death path held). THE TONGUE
   (Baz: "like the frog tongue that can pull you in"): the js frogTongue rig, ported
   for the mimic — at mid-range (34..72px) it roots itself mouth-agape and lashes
   (8f out / 3f hold / 9f back, direction locked at launch, maxLen 62, 2px line +
   4px tip in the js tongue palette); the tip snags a non-invuln hero and REELS him
   into the maw (play.rs `Pulled` resource: 3.2px/f drag, 28f, breaks on arrival/
   timeout/wedge/any hit — js p.pulled verbatim; walking is overridden, swinging
   still works, so you fight the reel). The maw frames grew a two-tone tongue
   ('T'/'t' — the same palette as the lash, one flesh) and the mouth pins OPEN for
   the whole flick (frog st==2). The Pulled rig is GENERIC — the frog's deferred
   lash (mob_think.rs) can ride it next. WRIFT_MIMIC=sprung stages a pre-sprung one
   for the shot. VERIFIED in-shot: full-extension lash tip-on-hero, mid-retract
   blob, and grab->reel->maw->bite over a dead shot hero. STILL OPEN: no codex/
   bestiary card (MOBS tab iterates MOB_DEFS; the mimic is its own actor), spring
   screech + tongue snag wait on the audio port (sfx queued on the bus).)*
20. **DUNGEON TEXTURE pass 1 — pits, smashables, the real chest table** — *(done —
   PIT-FALLS: pits leave the solid grid (room_view drops them from features) — walk
   in and navigate starts the tumble (PitFalling resource, PIT_FALL 46): control
   locks (ModeCtx.pit beside fishing/fluting), pit_anim tips the hero 45 degrees +
   shrinks him into the hole (sync_player_sprite gated off), then TWO hearts and —
   if he lives — the same bank-and-eject as the ornate door (spat out the dungeon
   mouth); dead in the hole, check_death takes it. DESTRUCTIBLES: the 9 js
   DESTRUCTIBLE kinds (barrel/crate/table/bookshelf/weaponrack/armorstand/urn/
   bonepile/cobweb w/ debris colour + hp; cobweb 1-swing + never solid) leave the
   BAKE + solid grid and spawn as LIVE entities at every room wake (enter/stairs/
   slide-end) — painted by the shared prop painter into their own canvases, own
   blockers, any weapon thunks them, smashed = debris burst + 50% a little coin +
   2% real gear (roll_loot 0.2), and the tile records into DRoom.broken (run-
   persistent: RoomSave.broken rides the ledger + saves). CHEST LOOT: the js table
   verbatim — gilded = 30-69 coin + boost-1.6 roll; regular = 10-33 coin + boost-
   0.25 roll + 50% a stick of wood/stone. STILL OPEN on the backlog: secret
   push-blocks, real themed boss AIs, finale story text, WRIFTHEART codex tab,
   flammable-prop burning. (Mimic chests: DONE, redesigned — milestone 21.)*
19. **ENCOUNTERS inc 2 — the people of the camps (js victim/wanderer + threat banner
   + camp light)** — *(done — app/encounters.rs grew its second half. VICTIMS: the
   frightened civilians spawn WITH the foes (fresh rooms; scene.victims now records
   the js table's calls) — hero-bank looks off seeded formulas, flee AI verbatim
   (bolt from the nearest foe at 1.3, mill/face-you when safe, grid+blocker+bounds
   step), PANIC yells on a seeded timer while any foe lives and ONE thanks line the
   moment the room clears; floating speech labels ride the shout rig (hidden mid-
   slide). DEVIATION (flagged): victims are immortal until an NPC combat team exists
   (the js lets foes cut them down). WANDERERS: staged strangers persist with the
   DECOR as root children; TALK pays the one-time boon by role (MetWanderers, saved):
   hurt = a bandage/potion FROM YOUR BAG -> coin + rolled loot ("IF ONLY I HAD A
   BANDAGE..." otherwise — come back with one), lost = coin + thanks, herbalist =
   herbs + a potion, minstrel = FULL MANA ("a tune for the road" — DEVIATION: the js
   waysong speed-buff awaits the player status system); idle lines ever after.
   THREAT BANNER: entering an un-beaten hostile camp announces it through the town-
   banner slot ("PLUNDERED CARAVAN - CLEAR THEM OUT -"). CAMP LIGHT: campfires r44 +
   crystals r30 join the lighting overlay via a per-room Local cache (deterministic
   scene rebake; slide-offset like the torch fix). Verified in-shot: the caravan
   with its banner + two fleeing victims yelling HELP!/AAAHH!, and the LOST TRAVELER
   waiting in his camp. STILL FLAGGED: bandit/ogre/cultist real sprites (goblin
   placeholders), victim mortality, the js night-surge interplay.)*
18. **ACHIEVEMENTS — THE HALL OF DEEDS (js/achievements.js, verbatim)** — *(done —
   src/achievements.rs: all 79 awards across the 7 categories (contiguity + count
   pinned by test), the HIDDEN set folded into per-row flags ('? ? ?' until earned —
   the whole RIFT chain, the aspirational tiers), cur/goal as fn pointers off an
   [`AchStats`] snapshot (stat-derived goals like "fill the bestiary" work).
   codex/awards_tab.rs: snapshot() IS js achStats() — stats ledger keys + bestiary/
   discovered/visited/cleared-encounters/giver-done-sum/relics/town-names/learned-
   songs/tomes/progress/money/kingsplitter-in-bag/people (met + best hearts via
   people::hearts); unported systems (home, animals, guild wings, songstones, rifts,
   blueprints, bosses, gate) read 0 and their awards WAIT — honest, no fake unlocks.
   AWARD_TICKER: every 32 play-ticks, newly-earned deeds unlock with a gold "DEED
   DONE" toast + save request (once earned, stays earned — saved as sorted rows;
   fresh-slot reset). THE TAB: the js two-pane hall — left ledger (category banners
   in their colours, medal studs, live floor(cur)/goal progress, '?' for hidden, '*'
   earned, selection + centred scroll, category hop on left/right), right MEDAL
   PLAQUE (category, the STAR_ART trophy gold/gray, name, wrapped deed, progress or
   EARNED; gilded inner frame when earned). DEVIATIONS (flagged): dungeons counts
   ENTERED (no discovered-set yet); the js laurel-ring circles await a circle bake.
   Shot pending — the black-frame occlusion episode was active at land time (town
   probe also 44733 bytes); the tab is plain codex UI, Baz eyeballs in-game.)*
17. **FLUTE SONGS + MANA (js/songs.js + game.js flute state — the melody layer)** —
   *(done — src/songs.rs (8 songs verbatim: 4-note UDLR strings, mana costs, colours,
   descs + unlearned HINTS; match_tail suffix rule; the shared 7x7 ARROW cells with
   all four rotations, tested) + app/flute.rs. PLAY-MODE (the rod's slot pattern):
   flute press raises it IN THE LIVE WORLD — moves become notes (D-pad flips to
   arrows, js dpadDirs), player rooted (ModeCtx.fluting beside fishing), B lowers it;
   played tails match learned songs ONLY (unlearned real melodies fizzle with a
   toast); the catch REPLAYS itself (banner + notes lighting one by one) then casts.
   Take-run state machine (the borrow-safe owned-f pattern). CASTS: canticle = a
   2-tick 104px ring through the NORMAL combat pipeline (damage 1 + knock 2.5);
   wardsong = +2 defense 600f (Ward resource lifts it back off); lullaby = Mob.sleep
   300f (AI guard skips thinking; ANY hit wakes via HitLanded — golems immune);
   greensong = FarmTiles::ripen_room; sunsong = clock jump (day_darkness picks dawn/
   dusk); stormcall = WeatherState COMMAND channel (timed, outranked by the WRIFT
   pin; weather::precip_for per climate); returning = distance-sorted town picker off
   TownNames -> WarpTo message -> loader::handle_warp (swap_world_room + centre
   landing); opening fizzles unanswered (no songstones yet). MANA lands (12 base,
   js regen 2/80 accumulator) — the sidebar MP bar goes LIVE (hud_mana). TEACHING:
   the inn's BARD zone (already in the interior def) serves — first visit gifts the
   flute + Song of Returning + back-teaches read songbooks, sells a 30c spare, then
   cycles the 7 written-down hints; the SEVEN SONGBOOKS carry `teaches` (Book field,
   107 literals defaulted) and teach on pickup; a flute arriving by ANY path
   back-teaches (catch_up_tick watches the bag — better than the js's two call
   sites). SONGS codex tab replaces the stub: learned rows show name + THE ONLY
   WRITTEN NOTATION (note-coloured arrows) + desc; unlearned keep '? ? ?' + the
   rumour. Saved: learned songs (sorted rows; fresh reset). WRIFT_SHOT=flute (mid-
   replay banner + compass + motes) / codex WRIFT_TAB=SONGS (4/8 learned page) both
   verified on screen. NOT YET: note synth voices (sfx bus keys await the audio
   port), vignette/glimmer polish, the warp charge animation, songstones.)*
16. **QUESTS (js/quests.js + the game.js quest state — the town job board)** — *(done —
   app/quests.rs (pure logic js-verbatim: qhash FNV w/ SALT 0x5177e3a1, is_giver 45%,
   makeRngFor mulberry streams keyed (giver seed, done count), the 4 buildType arms +
   seeded type-shuffle w/ fresh-types-first STABLE sort, rollReward incl. the 14%
   roll_loot gear / 28% materials split, BESTIARY kind names, spiral findRoom 3..12
   rings). Quest/QuestKind are serde-owned (saved as-is: quests + questGiverDone +
   questCounter rows; SocialCtx grew all three). LIVE STATE: giver glyphs float over
   town folk ('!' offer / '-' in progress / '?' ready — '-' stands in for the js '·',
   no font glyph) tracking them as they wander; the NPC chooser gains QUEST (canTalk:
   their active quest, or an offer while the log has room; 1-option collapse kept);
   DialogState::Quest in dialog.rs = the js window verbatim (title/desc/GOAL/REWARD,
   ACCEPT-DECLINE / TURN IN-ABANDON-CLOSE, abandon CONFIRM sub-box where B backs out
   ONE layer); turn-in pays Greed-scaled coin + gain_xp + item (bag-full drops at your
   feet), bumps giver_done, +150 pts w/ the giver ("X WONT FORGET THIS" + heart) and
   the quests stat. TRACKING: KillCredit messages from both death systems (slinger
   counts as goblin, the js e.type rule) drive slay counts + bounty flags; the
   encounter clear watcher marks clear quests READY; fetch reads the bag live. BOUNTY
   ELITES: bounty_spawn_tick lairs the named elite (4x HP stand-in for makeElite) in
   its marked room each visit until slain — BountyTag excludes it from the room cache
   (the js no-spawnKind rule). DEVIATIONS (flagged): clear quests target HOSTILE camps
   only (the js could point at friendly wanderer camps — auto-cleared on arrival, a js
   bug not kept); bounty spawns at room centre (js findClearSpot nudge pending).
   Tests: deterministic generation + signature dedup, giver-rate band, progress arcs.
   FOLLOW-UPS LANDED same pass: the sidebar QUESTS list (hud.rs — bullet rows, live
   counts, name clipped never the count), codex MAP pins ('!' at the objective until
   ready, '?' at the giver — green when ready; pin rooms FOLD into the map bounds so
   an unexplored objective still frames), and the bounty's NAME floating over its
   elite. WRIFT_SHOT=quest (sidebar) / questmap (pins) stage a seeded 3-slot log.
   BAZ TWEAK: ready-to-turn-in reads WOW-GOLD '?' everywhere — the tracker row swaps
   its bullet for the gold '?', the giver's overhead glyph and map pin match (was
   green; '!' offer -> '-' in progress -> gold '?' ready).)*
15. **ENCOUNTERS inc 1 (js/encounters.js — the overworld's set-piece camps)** — *(done —
   app/encounters.rs + actors/encounter_art.rs. The FULL 24-def ENCOUNTERS table is in
   (order/weights parity-load-bearing; friendly defs included so the weighted pick
   never shifts when inc 2 lands). for_room 1:1: worldgen::rng::hash IS the js shape
   (salt 0x9e3d71b1), 10% BASE_CHANCE, shard/town/dry-room vetoes (dry_enough <= 8%
   interior water), tier+biome eligibility, weighted pick. Scenes stage decor (14
   authored props: wagon/tent/banner/crystal/ritual/webs/ice/stakes/gold/corpse/blood
   /bones/crate/campfire w/ 2-frame flicker + clutter passthroughs to PropArt) as
   room-root children with blockers, rebuilt identically every visit; FOES REPLACE the
   natural mob roll (spawn_room_mobs takeover; bandit/ogre/cultist ride the goblin
   placeholder until their AIs port), marked EncFoe + ArmedEncounter; the clear
   watcher records the room into ClearedEncounters (saved; sorted rows) the moment the
   last foe falls — "AREA CLEARED" toast + stats bump — and a beaten room stays
   PEACEFUL FOREVER (no camp, no natural mobs — the js rule). Same-day cache restores
   re-mark + re-arm survivors so a return-trip kill still clears. Verified in-shot:
   plunderedCaravan at seed-1337 (0,-14) — wagons/gold/crates + 3 bandits. NOT YET
   (inc 2): fleeing mortal VICTIMS + panic/thanks speech, friendly WANDERER boons
   (lost/minstrel/hurt/herbalist), the threat-name banner, campfire/crystal night
   light, night-surge interplay.)*
14. **FARMING (js/farm.js + the game.js farm hooks — the life-sim pillar)** — *(done —
   app/farm.rs. SIM 1:1: FarmTiles (room->tile->{home,tended,watered,crop}), hoe tills
   any SOFT EARTH (the 12-ground TILLABLE set; towns + Emberfall refuse), watering can
   12 pours (refill facing open water '~'/'B' or within 40px of a well; CanWater is a
   RESOURCE, not a per-entry tank — DEVIATION, one can is all you need), seeds plant in
   season (8 js crops verbatim in items::CROPS; produce + seed defs generated per row),
   dawn pass grows watered crops / dries+withers (3 days) / season-culls, wild soil
   untended DECAY=3 days reverts, rain waters the whole room every 16 frames
   (weather::Kind::Rain). Walk-up Interact HARVESTS (ripe planted -> bag; wild forage
   -> pickup at your feet + daily gather stamp); prompts.rs shows PICK (door > tome >
   crop, the js priority); farm_harvest_tick runs BEFORE talk_tick and consumes the
   press (js onObject). WILD CROPS: spawnWildCrops hash VERBATIM (imul stream; ~30%
   of wild rooms/day, 1-2 in-season plants on bare grass). RENDER: soil beds + stage
   plants + per-shape fruit (drawFruit's 7 shapes) painted into images (local Px
   painter), spawned as ROOM-ROOT CHILDREN by spawn_room_props (they ride slides;
   natural props SKIP tilled tiles — the js VEG rule; cosmetics carry GroundVeg{c,r}
   so the hoe can strip them) and rebuilt in place via FarmDirty (sync guards: skip
   mid-slide, skip if the root died this tick). Pulsing corner-bracket reticle on the
   hoe/can target tile (js tileReticle colours, Sprite::color tint over white bakes).
   SAVE: FarmRow rows + can_water in SaveData (SocialCtx.farm/can_water/farm_day —
   B0002: reach them THROUGH ctx); prune-on-load; fresh-slot resets. items:
   equippable() now admits kind TOOL + SEED (js `tool: true` — the rod predated this
   and had silently failed auto_equip); hoe/wateringcan get HAND recipes (DEVIATION:
   js crafts them at the farmtable STATION — stations not ported; wild shops carry
   them + seeds via the already-generated stock tables). Dawn-sim unit tests
   (grow/wither/season-cull/decay/rain-scope) + WRIFT_SHOT=farm (staged 4-bed plot:
   dry/wet beds, stages 0-ripe, green reticle; point WRIFT_TOWN at a wild grass room).
   NOT YET: home-plot permanence (needs housing), HUD water gauge on the can slot,
   sword-cuttable wild crops, watering-can hand pose, GREENSONG ripen (songs),
   crops stat -> ledger exists via stats.bump("crops").)*
13. **THE DEV PANEL (ground-up redesign — Baz: the js overlay was "a garbled mess")** —
   *(done — NOT a port: a full-screen console (app/dev.rs, Screen::Dev freezes the
   world codex-style; BACKQUOTE toggles, new Action::DevPanel). Layout: live INFO
   STRIP (seed/room/day/clock + biome/weather/shard count), CATEGORY RAIL left
   (WORLD/TRAVEL/HERO/ITEMS/QUEST, Q/E to switch), commands right with the cursor
   row highlighted and LIVE VALUES right-aligned; cycle rows (WEATHER, SHARD SITE)
   adjust with left/right; F runs; every action toasts through the LootLog. V1
   commands: time +1h / skip-to-dawn / season+1 / weather pin (WeatherState::force);
   warp home/castle/any shard site (World::shard_sites made pub, warp closes the
   panel + lands you centred); full heal / +100 copper / level up; fishing kit /
   key ring / potion pack; grant next shard / ALL shards (the castle-gate fast
   lane) / clear shards. OPAQUE backdrop — nothing garbles through (the js sin).
   FONT GOTCHA re-learned: the bitmap font has no '·' or backtick glyph — hint text
   uses '-' and the word TILDE. WRIFT_SHOT=dev stages it. Extend by adding a Cmd
   variant + a rows() entry + a match arm.)*
12. **WEATHER (Baz: "with rust and the shader is there any way to do it better?")** —
   *(done — the split Baz approved: SIM 1:1, PRESENTATION rebuilt in WGSL. SIM
   (src/weather.rs): DEFS/CLIMATE/BIAS/SEASON_FRONTS/hash/rollFront/weatherFor
   verbatim (front weights in JS OBJECT-KEY ORDER — parity-load-bearing), pinned by
   tools/extract_weather.mjs -> 72 golden rows, bit-exact first run. Three fronts a
   day (period = clock/(DAY_LEN/3)). STATE (app/weather.rs): the two-layer crossfade
   (EASE 0.014), wind targets, lightning rolls + GROUND STRIKES with a position,
   WRIFT_WEATHER pin, 18 leaf sprites for 'windy'. SHADER (gfx/weather_fx.wgsl, one
   full-screen quad at z 13.2): rain in THREE PARALLAX DEPTHS (hashed 1px columns,
   wind-sheared field), quantized snow in two depths, dust streaks + haze, and FOG as
   2-octave scrolling value noise POSTERIZED into alpha bands (the js drew 5 gradient
   blobs) — everything on the 304x208 pixel grid. TIE-INS (PORT-ORIGINAL, the pitch's
   point): weather mood feeds the lighting overlay (sky darkness + tint pull + strike
   lift, REDUCE FLASHING respected in both overlay + shader); CLOUD COVER DIMS THE
   SUN'S SHADOWS (day_a x (1-0.65·cloud)); STORMS CHOP THE WATER (WaterParams.storm:
   hurried waves, brighter glints); a ground strike punches a light hole in the
   darkness at the bolt's spot. VERIFIED in-shot: rain (slanted parallax streaks over
   Coldgate) + FOG (rolling banded banks — the pitch delivered). Rain density tuned
   sparser post-shot (gap 46+60h; dials in rain_sheet). AWAKENED: rainfish/voidfin
   now catchable in their weather (fishing passes Weather's LIVE id). NOT YET:
   sightCut vignette, slows (blizzard trudge), strike damage (consumeStrike),
   song-commanded skies, weather HUD line. FOLLOW-UPS (Baz playtest): rain FELL UP
   (sign slip — the pattern must advance +y with time: `y = sp.y - t*speed`; snow
   had it right) and was FAR too dense — thinned via a per-column existence gate
   (`wet`, ~25% empty) + gaps 44+52h + 1px-of-4px columns; dials live in
   rain_sheet(). TWO WGSL LESSONS, hard-earned: (1) `active` is a RESERVED WORD in
   WGSL — naga rejects it and (2) shader errors surface at RUNTIME on stderr, not
   at cargo build — a silently-failed pipeline renders NOTHING, which reads exactly
   like a too-subtle effect. After ANY .wgsl edit, run once WITHOUT 2>/dev/null and
   grep for `wgsl|naga` before judging the visuals.)*
11. **FISHING (core loop)** — *(done — js startFishing/updateFishing/resolveCatch +
   the items.js tables. ITEMS: fishingrod (TOOL, the RECIPES row now shows — it was
   waiting on the def), 12 fish (one FISH_GRID silhouette recoloured per species via
   icon_pal — literal-slice args promote to 'static), 3 junk. TABLES: FishRow
   (water/biomes/seasons/weather gates + lb range) + roll_fish (14% junk, rarity
   weights 60/26/10/2/1) — behaviour-tested (murk offers only its natives; winter
   mountains bite icefish, never sunfish). THE LOOP (app/fishing.rs, one system):
   rod-slot press -> front-tile water check ('~'/'B' + water_style murk/blue) ->
   bobber + rotated 1px line + prompt bar; the world RUNS while you wait (tick's
   ModeCtx grew `fishing` — rooted, no swings; a hit SNAPS THE LINE); bite at
   55+rand*150 flashes "!" + dips the float; tap Slot1/2 inside the rarity window
   (14/18/22/27) to land it — CAUGHT <NAME> N LB in rarity colour, junk snags,
   IT GOT AWAY. Presses consumed so nothing swings when the world thaws. FLAGGED
   waits: weather fish dormant ("clear" until weather ports), lure trinket (bite
   timing + window hooks in place), mapbottle (treasure maps), cooking buffs,
   season fish use calendar_tab::season_index ✓. WRIFT_SHOT=fish stages a lakeside
   cast (black-frame episode ate the proof shot — Baz playtest: craft a rod, face
   water, press its slot).)*
10i. **DUNGEONS ARC, step 9: THE BLACK CASTLE + the WIN** — *(done — the game is
   winnable in skeleton. src/actors/castle_art.rs: buildCastle transcribed (192x144
   Px painter — shaded blocks, crenellations, fire slits, banners, the rift-eye
   spire, the arched gate) + the gate STATE baked per (unlocked, shards): sealed =
   iron-banded doors + TEN SHARD-SOCKETS lighting as they land; unlocked = a static
   rift bloom (the roiling shimmer joins the glow pass). room_props spawns the
   facade bare + CastleGate; dress_castle (Update) re-bakes on shard-count change —
   the tenth shard swings the doors WHILE YOU WATCH — and swaps the gate's blocker
   rect (whole arch sealed -> top-half open, js hitboxes). enter_dungeon's castle
   branch: sealed = "THE GATE IS SEALED - N OF 10 SHARDS" toast; whole = the
   four-floor CASTLE finale (is_final, biome None — no shard beyond, js
   enterFinalDungeon). The finale's boss: the riftlord at 12x as THE WRIFTHEART
   stand-in (flagged, like every boss). Its fall -> "THE FAR SIDE OF THE WOUND
   FALLS SILENT" + the VICTORY overlay (js drawVictory: fade, THE WRIFTHEART IS
   MENDED / YOU WIN / keep-playing dismiss on INTERACT). VERIFIED in-shot: the
   castle at (0,-43) — towers, banners, rift-eye, sealed gate, sockets dark,
   graveyard grounds. REMAINING (task #23): texture backlog (destructibles,
   pit-falls, mimics, real boss AIs, chest loot table, WRIFTHEART codex tab,
   the win's FINALE story text + credits).)*
10h. **DUNGEONS ARC, step 8: the quest SAVES** — *(done — relics + the dungeon ledger
   ride the save file. SaveData grew `relics: Vec<String>` (sorted — deterministic
   bytes) + `dungeons: Vec<(String, DgSave)>`; the container's #[serde(default)]
   keeps old files loading clean. DgSave/RoomSave derive serde: rooms became a
   Vec<(floor,rx,ry,RoomSave)> (JSON maps need string keys), rosters store String
   kinds re-interned on apply via themes::intern_kind (roster names only ever come
   from the pool tables; a stale save's unknown kind is skipped). Dir derives serde.
   Relics + DungeonLedger moved into SocialCtx (the "new save resources nest here"
   rule) — and the B0002 rule paid out immediately: navigate + handle_load_slot had
   to DROP their standalone ResMut params and reach both through ctx.social. Slot
   switches clear both then restore from the loaded file. The save_round_trip canary
   covers the new fields. Shard progress + opened doors + looted chests now survive
   a full quit-and-relaunch.)*
10g. **DUNGEONS ARC, step 7: the GUARDIAN and the SHARD (the main quest exists)** —
   *(done — increment 4's spine. src/relics_data.rs GENERATED (tools/extract_relics.mjs)
   from js/relics.js: all 28 shards (name/colour/lore) + INTRO/FINALE/STORY;
   by_biome(). app: Relics resource (claimed set — in-memory beside the ledger,
   save-file layer flagged), DungeonRun.biome + .arena. THE FIGHT: entering the boss
   room with the guardian alive SLAMS every door (js sealBossArena — one-sided locks
   + in-place rebake; the boss survives the rebake, it's a RoomActor); the STAND-IN
   BOSS (flagged loudly: an elite of the theme roster's heavyweight at 6x HP wearing
   DungeonBoss — js Enemies.boss themed AIs port as their own pass) falls -> doors
   reopen (disarm), room banks cleared + boss_loot, THE GUARDIAN FALLS toast, and
   the loot stand-up spawns: the gilded reward chest (9,7), the land's SHARD glowing
   in its own colour (the js GEM icon baked per-relic), and the WARP RUNE home.
   Touch the shard -> Relics claims the biome, "THE <NAME> IS YOURS" in shard colour
   + "N OF 10 SHARDS"; touch the rune -> the same banked exit as the ornate door
   (go_home unification). boss_loot persists in the ledger: re-entry re-stands rune/
   chest/shard (unclaimed only) exactly like js loadDungeonRoom. NEXT (inc 5): the
   castle gate's shard sockets + relicsComplete -> the final dungeon; plus the
   backlog texture (destructibles, pit-falls, mimics, save-file relics+ledger,
   real boss AIs, the WRIFTHEART codex tab).)*
10f. **DUNGEONS ARC, step 6: dungeons are PERMANENT (in-run + re-entry)** — *(done —
   the ledger (js dungeonState): DungeonLedger, an in-memory map keyed by entrance
   "rx,ry" (the RoomCache precedent — the save-file layer is a flagged follow-up;
   handle_load_slot clears it). serialize_run banks the whole run on the ornate exit:
   per-room flags (cleared/looted/key_taken/bosskey_taken) + survivor rosters (kept
   live by bank_room, so it's a straight copy) + the REMAINING lock sets per floor
   (simpler than the js opened-list replay — we store what's still shut). On re-entry
   the deterministic regenerate happens first, then apply_ledger overlays the banked
   state: kills stay killed, loot stays looted, opened doors stay open, across exits
   and re-entries. Roster kinds stay &'static str (in-memory ledger — no interning).
   KNOWN GAP (flagged): dying inside a dungeon drops the unbanked run (swap_world_room
   clears InDungeon without serializing) — acceptable sting for now, wire death-path
   banking with the save-file layer. STILL OPEN: destructibles, pit-falls, secret
   push-blocks, mimics, save-file ledger; then bosses + shards (inc 4).)*
10e. **DUNGEONS ARC, step 5: keys turn, chests spring, the DARK comes down** — *(done —
   three more 10c deviations burned. (1) KEYS + CHESTS: key/ornatekey item defs (js
   icons; the ornate's violet gem via icon_pal) + a Chest entity (PORT-ORIGINAL
   stand-in art, open/closed lids): the treasure chest at its generated spot, the
   small key's chest at room centre, the gilded ornate-key chest beside the treasure
   (js spots). Walk onto a chest to spring it — held keys go straight to the bag
   (LootLog toast), treasure spills coins + one boosted roll (v1 roll, FLAGGED: the
   js chest loot table ports with the loot pass). Room flags (looted/key_taken/
   bosskey_taken) live on DRoom for the persistence layer. (2) THE UNLOCK (js
   tryLockedDoor): walking into a locked gap with the right key consumes it, opens
   BOTH faces forever (in-run), re-bakes the room in place, toasts UNLOCKED! / THE
   ORNATE DOOR SWINGS WIDE; no key = a cooldown bounce. (3) DUNGEON-DARK: the
   lighting overlay forks underground — theme ambience 0.78 (ambAlpha themes opt
   out, 'dark' hymn floors 0.94) through DARK_GAIN, the THEME'S tint, and the js
   light set: the hero's always-carried pool (r54; r84 with lantern gear when it
   ports), wall torches r28 (skipped where wide doors opened their wall), lit decor
   (fireplace 42 / brazier 30 / crystal+altar 26 / candelabra 22), pickups r16.
   B0002 LESSON: SaveCtx already carries ResMut<PlayerInv> — a system taking SaveCtx
   must reach inventory THROUGH it (ctx.inv), never as its own param (the panic
   HANGS the WRIFT_SHOT harness: a timeout, not a crash — check stderr for B0002
   before suspecting the capture). VERIFIED in-shot: the Vine Warren in gloom —
   torch pools on the walls, the hero's light, the centre falling dark. STILL OPEN:
   destructibles, pit-falls, secret push-blocks, persistence, mimics + boss court.)*
10d. **DUNGEONS ARC, step 4: the halls SLIDE and the halls BITE** — *(done — two of
   10c's deviations burned down. (1) SLIDING transitions: play.rs exposes start_slide
   (the Slide fields stay private); navigate spawns the neighbour room's root ONE
   SCREEN OVER (spawn_droom grew a delta) and hands the same in-flight machinery the
   overworld uses; tick's LANDING branch now forks on ModeCtx — dungeon lands spawn
   the room's roster, overworld lands keep spawn_or_restore/banners/visited. (2)
   ENEMIES: spawn_room_foes wakes an uncleared room's generated roster — ported kinds
   spawn REAL via mob_bundle, unported kinds wear the goblin placeholder (the
   overworld's registry-fill rule), each tagged DungeonFoe(kind) so bank_room can
   write SURVIVORS back into the run at slide start / stairs use: kills stay killed
   within the run, an emptied room reads cleared (save-file persistence still
   pending). InDungeon moved INTO SwapCtx — swap_world_room clears it on every
   outdoor stand-up, so death respawns + slot loads exit the dungeon for free;
   navigate TAKES the run out of the resource for its tick (borrow-free) and puts it
   back unless we left. Maze stairs-guards spawn on floor arrival. VERIFIED in-shot:
   hero walks INSIDE the dungeon and stops clean at a doorless wall (start room is
   W/E-doored — confirmed via a layout dump; the "missing" gaps were dark-on-dark
   floor at the screen edge, present all along). Room-crossing + foe shots blocked by
   a black-frame episode — Baz playtest covers them. STILL OPEN (10c list): chests/
   keys, destructibles, pit-falls, dungeon-dark, persistence.)*
10c. **DUNGEONS ARC, step 3: you can WALK THE HALLS** — *(done — the playable interior.
   src/dungeon/render.rs bakes a room to RGBA (js render() verbatim): brick walls w/
   staggered seams, RAW CAVE ROCK (per-tile mulberry32 off hashK("floor:x,y") — bosses,
   crevices, cracks), guildhall timber+wainscot, cave LIPS + still POOLS (lava pools
   molten, frost frozen), locked-door art (banded small-lock / gilded horned BOSS door),
   merged-void PITS, wall torches at TORCH_SPOTS, drawEntrance (pilasters + gold
   keystone + warm glow), and a PORT-ORIGINAL stairs tile. src/dungeon/prop_paint.rs:
   all 33 js PROP painters as fill-rect lists over a Px buffer (licence: magiccircle
   arcs plotted parametrically). app/dungeon.rs: press INTERACT at the mouth ->
   enterDungeon (js seed formula, THEME_BY_BIOME + FLOORS_BY_TIER), one baked sprite
   per room + RoomGrid synth ('M'/'.' — collision free), navigate() = door-gap edge
   walks (instant swap, SLIDE = flagged follow-up) + STAIRS at (4,3) between floors
   (banner re-announces 1F/B1/…) + the dungeonExit pad back to the overworld doorstep.
   play.rs grew ModeCtx {inside, dungeon} (RoomCtx was AT the 16-cap — nested bundle)
   gating the overworld edge-slide; interior doors / room-cache snapshots / critters
   all gate on InDungeon. WRIFT_SHOT=dungeon stages it (walks to the mouth, presses
   INTERACT; WRIFT_WALK="dir,frames" walks inside) — needed hold_for_test + a
   test_held accumulator in input.rs (poll_input REWRITES `held` each frame; only
   `pressed` accumulated). VERIFIED in-shot: THE VINE WARREN 1F standing (green brick,
   torches, ornate exit, banner) with the hero inside. DEVIATIONS (all flagged, all
   next increments): instant room swaps (no slide yet); no enemies/chests/keys as
   entities yet; destructibles baked SOLID; pits solid (no pit-fall); no dungeon-dark
   ambient; no dungeon save/persistence (js dungeonState) — leaving + re-entering
   regenerates.)*
10b. **DUNGEONS ARC, step 2: the shard monuments stand** — *(done — the overworld face.
   tools/extract_entrance.mjs slice-evals the ten archetype builders out of the live
   js/entities.js (blank/outlined + mPx/mDisc/mCone/mMouth + build* + ENTRANCE_STYLES)
   and marries them to js/relics.js shard colours -> src/actors/entrance_art.rs: 28
   baked 64x56 monument bitmaps (grid + palette + eye spots + glow), tier-6 lands
   wearing the colossus with their own glow like the js fallback. room_props renders
   the worldgen "dungeon" entity (now carrying its biome key in `sub`): sprite
   anchored mouth-on-tile (OX -24, OY -40), solid mass blocker (js hitbox), and
   app/dungeon.rs DungeonEntrance + 2x2 eye children whose shard colour BREATHES
   (alpha 0.25+0.45·sin(clock/24)). Verified in-shot at seed 1337's greenmaw site
   (3,3): monument + processional way + flanking torches + waymarkers + honour guard.
   BUG FIX EN ROUTE: sync_shadows' attach pass could insert Shadowed/Reflected on an
   owner that died the SAME frame (room-swap despawn racing command apply — latent
   since the ChildOf slide fix; registering DungeonPlugin reshuffled the schedule and
   exposed it as a boot crash). try_insert on both attach markers; orphan quads reap
   next tick. Debug recipe that found it: `cargo run --features bevy/debug` names the
   panicking system when ECS errors hide behind <Enable the debug feature...>.
   NEXT: press-to-enter at the mouth -> the themed interior bake (src/dungeon render:
   brick/cave/hall walls, pools, locked doors, pits, prop painters, torches,
   drawEntrance) + in-dungeon navigation + exit; then stairs/keys wiring.)*
10. **DUNGEONS ARC, step 1: the generator library (src/dungeon/)** — *(done — the pure
   data side of js/dungeon.js, parity-pinned. themes.rs: all 30 THEMES (hex -> u32,
   Style::Brick/Cave/Hall, pools, tints, guildhall ambAlpha) + ENEMY_POOL (castle's
   SIX-entry roster special-cased in pool()). decor.rs: PropMeta (w/solid/destructible/
   lit/detail) for all 33 props, THEME_DECOR pools, QUAD_PATTERNS + mirror_quad, and
   place_decor with the js rng call order preserved LINE BY LINE (discarded draws
   included — corner-cobweb rolls fire even when the corner's taken). floorgen.rs
   (`gen` is a Rust 2024 reserved word): genFloor (room-web growth, BFS dist map in
   DISCOVERY order for the deepest-room tie-breaks, treasure SPOTS, the full KEYWORK
   chain incl. farthest_safe_normal deadlock guard), genMazeFloor (backtracker
   labyrinth + guarded stairs + dead-end prize), genMirrorFloor (the lost-woods hall),
   genGuildhallFloor (wide-door great hall + 5 gwing rooms), and generate() (floor
   stack, MAZE_GIMMICKS/GUARDS/SIZE ladders, stairs linking). mod.rs: Dir (n/s/w/e —
   js DIR_VEC iteration order), Door::None/Open/Wide, DRoom/Floor/Dungeon (js getters
   -> cur()), solid_grid. PARITY: tools/extract_dungeon.mjs evals the LIVE js and pins
   467 golden lines across 7 cases (2-floor crypt, mini cave, 5-floor saltmaze with
   maze+mirror floors, guildhall, noLocks rift, rng-floor-count ruins, 4-floor tomb) —
   every room/door/lock/decor/pit/enemy, BIT-EXACT ON THE FIRST RUN. Floor::order
   keeps js Map insertion order (rng consumption order in the populate pass). NEXT
   (task #21 continues): app/dungeon.rs — overworld shard-site entrances
   (world.js shardData: 10 biomes, spiral siting, processional way), enter/exit
   (js enterDungeon/exitDungeon, THEME_BY_BIOME + FLOORS_BY_TIER), the room bake
   (js render(): brick/cave/hall walls, pools, locked-door art, pits, prop painters,
   torches, drawEntrance) + in-dungeon slides, stairs, keys.)*
9b26. **The Pocket Watch (first trinket + gear flags) & flyover shadows** — *(done —
   two Baz asks. (1) POCKET WATCH ported from js/items.js: def + I_WATCH icon
   (P/A/K chars — the rs palette already matched), slot "trinket", and a NEW
   ItemDef.flags field porting the js gear-flag booleans (clock/light/compass/…)
   with PlayerInv::has_gear_flag as the reader (WORN gear only, js semantics).
   Its power: a TIME section on the sidebar (app/hud.rs) — sun/moon pip + HH:MM,
   frame 0 = NOON, pip flips moon-blue when ambient_alpha > 0.5; rebaked only when
   the minute flips. TEMP DEVIATION: a fresh hero boots with the watch pre-worn in
   trinket 1 (Baz: "starting item for now" — handy while day/night feel work is
   hot); pull back to shop-only later — defining the item auto-shelved it in the
   general/trader/tool stocks (registry-fill rule), and the stock goldens absorbed
   it as designed. (2) FLYOVER SHADOWS: the title backdrop is CPU-baked, so the
   quad system can't reach it — bake_room now composites each prop's noon shadow
   (silhouette flipped at the feet, stretch 0.45, no shear, alpha 0.38*0.9) under
   ALL props, clipped off water tiles like play. GAMMA gotcha inverted: the GPU
   blends black in linear, so the sRGB bake darkens by (1-k)^(1/2.2), not (1-k).
   NOTE: WRIFT_SHOT=title already existed for flyover shots (the default boot
   state IS the title — the scene arm just stages nothing). FOLLOW-UP (Baz:
   "night doesnt get very dark"): the predicted DARK_GAIN calibration landed —
   the GPU blends the darkness overlay in LINEAR space vs the js's sRGB canvas,
   so raw js alphas read too bright; gain 1.16 restores the js midnight
   (derivation in lighting.rs). Every js ambient alpha routed through
   ambient_alpha gets it for free; dungeon 0.94 will clamp to 1.0. Plus the night
   TINT lifted [10,14,38] -> [12,18,58] (DEVIATION, Baz: "slight bluish tint" —
   the js navy composites to near-black; alpha keeps the darkness, blue keeps the
   moon). FOLLOW-UP (Baz: "trees are cut off" at flyover room edges): each room
   now bakes TWO images — terrain (room rect, FLY_Z 18.74) + props (MARGIN=56px
   overhang, PROP_Z 18.75) — so edge canopies + shadows draw past the seam, and
   the layer split keeps every canopy over every neighbour's floor (cross-room
   y-sort between overlapping canopies is approximated by the split; in-room
   stays exact). Verified in-shot: a canopy straddling a mid-screen seam, uncut.)*
9b25. **Shadow feel pass: shake, stumps, the sun-fade** — *(done — three Baz calls.
   (1) AXE-SHAKE: a chopped node's shadow swings with the trunk — the nodes query
   reads Option<&Shake> and offsets the anchor with the SAME formula apply_shake
   runs on the sprite (sin(t*1.7).round()*2), so the two stay pixel-locked; bushes
   and boulders wobble too, and a lakeside tree's REFLECTION shakes for free (same
   anchor pass). (2) STUMPS: growth-stage sprites (stump/sapling/young) aren't
   GatherNodes, so they got a new opt-in — CastsShadow{left,top,w,a}, a static
   feet-anchor component any plain sprite can carry; room_props stamps it on stage
   spawns with the grown tree's anchor, and the silhouette tracks the live stage
   art so each regrowth stage casts its own shape. (3) THE SUN-FADE: day_a was a
   two-value step (0.38 day / 0.24 night) — now the sun's strength IS the shadow's:
   sun_a = 0.38*max(elev,0)^0.6 + moon_a 0.10*max(-elev,0)^0.6. Full at noon, long
   and fading through dusk to NOTHING at the horizon, a faint moon shadow rising
   for the small hours, back with the dawn — and the golden-hour combo (max stretch
   + dying alpha) comes free. Verified via a new WRIFT_CLOCK=frames hook in
   debug_shot (pins time-of-day for any town_stage scene; noon/afternoon/dusk/
   midnight shots show the ramp). NOTE the ordering tolerance: sync_shadows vs
   apply_shake are unordered in Update — worst case the shadow leads the wobble by
   one frame, invisible at 8 frames of shake. FOLLOW-UP (Baz): shadows DROWN ON
   WATER — where the surface is, the REFLECTION is the shadow. ShadowMaterial
   gained the water-mask bindings + a rect uniform (the same fragment->room-px->
   mask-texel mapping reflection.wgsl uses; rect is the QUAD's rect, margin and
   all, since uv spans the quad), and shadow.wgsl multiplies alpha by dry =
   1-step(0.5, mask.r). A bank-side silhouette clips at the waterline; a plank
   bridge counts as water ('B' in the mask), so crossing one you lose the shadow
   and keep the mirror beside the deck — flag if a deck shadow is wanted (would
   need a bridge channel in the mask). Verified at the Silvervale bank: hero's
   shadow gone on water, reflection intact, villagers still shadowed on grass.)*
9b24. **Shadows: CAST becomes THE shadow; props join; the blob retires** — *(done —
   Baz's calls, in order: props needed grounding ("weird that they are on the players
   and not those"), then the whole BLOB mode + its VIDEO toggle retired — the shader
   look IS the look. shadows.rs is now cast-only: every shadow-bearer (player,
   villagers, goblins, mobs, critters, and GatherNode props — trees/bushes/boulders)
   gets a shadow.wgsl quad over its live art; tree canopies lean with the sun and
   growth stages re-shadow automatically (the material samples whatever the sprite
   shows). Lakeside props also REFLECT (the nodes ride the same anchor pass as the
   water mirrors). Removed: ShadowArt/blob_image, the mode-flip rebuild, the
   cast_shadows setting + menu row + WRIFT_CAST hook (old settings.json files with
   the field load fine — serde ignores unknowns). LESSON kept from the blob era: the
   integer-anchor rules still position every quad's FEET RECT (rounded owner base +
   integer offsets), so contact stays exact even though the shader edges are soft.
   FOLLOW-UPS (same day): (1) the ROOM-SLIDE bug — root-children owners (props,
   villagers) SCROLL during an edge slide but their quads spawned standalone and sat
   parked (the old tree transition bug, re-earned): quads now insert ChildOf(the
   owner's parent) so they ride the same root; the at() coords are root-local by the
   same convention props use. The absolute water OVERLAY hides while sliding instead.
   (2) THE RIPPLE (Baz): water.wgsl gained fine undulating crest lines (wavelength a
   few px, drifting down-surface, thin light crest + whisper of shade) on top of the
   slow interference waves.)*
9b23. **THE WATER PASS (PORT-ORIGINAL — "we moved to rust for a reason")** — *(done —
   three pieces on the shader infra. (1) THE WATER MASK (app/water.rs): every room
   stand-up bakes a 19x13 texture from the tile grid — r = water ('~' + under-bridge
   'B'), a = shore distance (ring probe, 3 deep; off-room counts as water so border
   lakes stay deep). Rgba8Unorm, NOT srgb — it's data. (2) THE LIVING SURFACE
   (gfx/water.wgsl + WaterMaterial): one room-covering quad between the water tiles
   (1.0) and the bridge decks (1.5) — two slow crossing sine waves make a drifting
   interference pattern, glints on the crests, faint trough shade, deep-water tint
   from the mask alpha. Dry rooms + interiors hide it. (3) REFLECTIONS
   (gfx/reflection.wgsl + ReflectionMaterial): every actor gets a mirror quad riding
   the shadow system's anchor pass (same live-sprite trick — the walk cycle reflects
   for free): flipped at the feet, tinted 50% toward the water, sine-rippled, faded
   with distance, and CLIPPED to the mask (rect uniform maps fragments -> room px ->
   mask texel) so it cuts exactly at the bank. Straight-alpha lesson: premultiplying
   AND blend-multiplying double-darkens — return vec4(tinted, a). sync_shadows'
   resources bundled into FxCtx (the 16-param cap again); reflections attach/follow/
   reap beside shadows in either shadow mode. New layers: WATER_OVERLAY 1.3,
   REFLECTION 1.4. WRIFT_POS="x,y" pins the hero for bank shots. The ripple math is
   ~90% of a future heat-haze. Verified in-shot: hero mirrored at the town lake,
   clipped at the grass edge, glint bands drifting.)*
9b22. **The port's FIRST SHADER: ShadowMaterial (Mesh2d + WGSL) for CAST shadows** —
   *(done — gfx/shadow.wgsl + gfx/shadow_material.rs (Material2dPlugin, embedded
   asset — no assets dir; the exe stays self-contained). CAST mode's shadow is now a
   unit-quad Mesh2d scaled by Transform, its fragment shader sampling the OWNER's
   live sprite art: flipped at the feet (quad top = the art's bottom rows), a real
   9-tap cross GAUSSIAN blur (radius in texels — a dial, not a trick), and a real
   UV SHEAR — the silhouette leans with the sun (west at dawn, tight at noon, east
   at dusk, faint moon shadow at night) while the FEET stay planted, which sprite
   transforms cannot do. The quad runs CAST_MARGIN (1.5x) wider than the art so the
   lean never clips; shadow.wgsl maps uv through the same margin — keep them in
   lockstep. Replaced the 4-echo sprite hack (1 draw instead of 5). Mode toggling
   (VIDEO -> SHADOWS) razes every shadow entity and re-attaches in the new shape.
   REUSE NOTE: the material plumbing is deliberately generic — the pending additive
   glow pass (lighting step 2) is this pattern + AlphaMode2d::Add + its own WGSL.
   Bevy 0.19 path crumbs: Material2d/MeshMaterial2d/AlphaMode2d live in
   bevy::sprite_render; ShaderRef in bevy::shader; WGSL imports
   bevy_sprite::mesh2d_vertex_output. Verified in-shot both modes.)*
9b21. **Actor shadows (PORT-ORIGINAL — the js draws none) + the pickup-glow depth fix**
   — *(done — `app/shadows.rs`: soft ground blobs under the player, villagers,
   goblins, biome mobs and critters, THE RUST WAY: baked blob textures + ONE system.
   THREE hard-won pixel rules (Baz's reports): (1) FLAT one-tone ellipse, hard edges —
   an alpha-gradient falloff reads as blur against 1-bit art; (2) NATIVE texture size
   per (w,h), cached — custom_size scaling (12px art -> 13px) is itself a blur; (3) a
   blob's position derives from the SAME rounded base as its owner's sprite plus an
   INTEGER offset (people: 16px box -> 12x5 blob at +2, dead centre) — rounding the
   offset sum separately drifted off-centre with the owner's subpixel position. ALSO
   uncovered: critter sprites were drawn in a wrong-size at() box (12x13 for every
   kind) and rendered offset from their own positions — per-kind native sizes now.
   PLUS the CAST prototype (Baz: "as long as we can easily pull it out"): VIDEO ->
   SHADOWS: BLOB/CAST — cast mode replaces the blob with the owner's OWN sprite,
   flipped + squashed + black-tinted at the feet (the silhouette animates with the
   walk cycle for free), swung by the sun (long west at dawn, tight at noon, long
   east at dusk, faint moon-shadow at night). EASY PULL: every piece is marked
   "PROTOTYPE (easy pull)" — one Settings field, one VIDEO row + confirm arm, the
   `sil` Anchor field + one if-branch in sync_shadows. WRIFT_CAST=1 stages it in
   shots. ALSO: max HP never folds below HP_BASE (Baz: no 1-heart starts — a bad
   trait's penalty only bites once tree Vitality lifts you above the floor). Each shadow-bearing type is a query line — attach on sight (Shadowed on
   the owner, ShadowBlob on the blob), follow every frame from the type's own anchor
   (mobs use their hitbox bottom; fliers get a smaller 0.5-alpha blob at their ground
   line; smalls shrink), reap when the owner goes; blobs ride the RoomActor sweep iff
   their owner does, so the player's crosses rooms. New layers::SHADOW = 3.95 — under
   the whole depth-sorted actor band, so overlapping actors never draw under each
   other's shadows. The player's hides with the body on death; fireflies are lights,
   no shadow. ALSO: the pickup-glow layering bug (Baz) — pickups carried a +0.5 z
   boost (~26px of depth), so a drop beside a trunk drew its 20px glow OVER the tree;
   both icon and glow now sort at the item's visual foot (+0.02/+0.01), and the
   one-frame spawn flicker at the band top is gone. Verified in-shot (red-debug
   bisect, then the soft final at the hero's boots).)*
9b20. **Feel + consistency pass (the 2026-07-16 design review, all approved)** —
   *(done — SIX pieces, each a deliberate DEVIATION or new architecture. (1) CORPSE
   RUN: the death-scattered bag never expires (js: 20s) — it lies where you fell, the
   room cache carries it across re-entry, and DAWN's world refresh is the recovery
   deadline. (2) SHOPS RESTOCK: BoughtShop split into `forever` (one-of-a-kind wares —
   gear/trinkets/blueprints: `!stackable || unique`) and `today` (staples, back at
   dawn); the js never restocks anything. Save grew sold_today/sold_day (serde-default,
   old saves fine; old bought_shop entries read as forever). (3) FRIEND RATES: the
   keeper discount is VISIBLE — a green FRIEND RATES / gold CONFIDANT RATES tag under
   the shop purse (ShopState.discount). (4) ONE DAY: hellos + gifts now roll on the
   DAWN day with the rest of the world (talk/dialog/people_tab moved off the js noon
   dayNumber). (5) STRANGER SMALL TALK (PORT-ORIGINAL content): under 3 hearts a
   villager alternates their stock line with a 16-line small-talk pool, deterministic
   per person+day — people::greeting() wraps the parity-pinned line_for(); Villager
   gained stock_line so the character line survives the variety. (6) ARCHITECTURE:
   `gfx/layers.rs` — THE z-ladder as named constants (new overlay modules migrated;
   legacy literals migrate as files are touched), and `app/sfx.rs` — the Sfx message
   bus standing ahead of the audio port (emit by js sound key; the synth lands as one
   consumer; emit points join with each system's audio pass).)*
9b19. **De-monolith pass (the ~500-line rule, enforced)** — *(done — the two worst
   hand-written files split, no behavior change, tests bit-identical. battle.rs (737)
   -> `app/battle/` by lifecycle: mod (plugin/RoomActor/GameRng/spawn+despawn/
   not_sliding, 119), ai (goblin+mob brains + knockbacks, 225), projectiles (swings/
   webs/rocks/arrows/bolts, 161), deaths (drop recipes + XP + bestiary, 133), fx
   (bursts/blood/sprite syncs, 138) — every external path unchanged via re-exports.
   mobs.rs (911) -> mobs.rs (350: shapes/bundles/art bank) + mob_defs.rs (190: the
   ROSTER as pure data — adding a mob is a row here) + mob_think.rs (392: the one AI
   interpreter). WATCH LIST (coherent, under judgment, split when they next grow):
   props.rs 638 (seeded tree generators, parity-pinned), char_tab.rs 623 (one dense
   widget), play.rs 593 (slide machinery is the next cut), worldgen/entities.rs 559,
   debug_shot.rs 545 (splits per scene family), gather.rs 516 (pickups.rs is the
   next cut). Generated files (buildings_art 1177, deathlines 1013, ...) are exempt —
   they regenerate from extractors.)*
9b18. **Dawn is THE daily reset + critters (birds/butterflies/rabbits/deer, fireflies)**
   — *(done — TWO parts. (1) ONE WORLD CLOCK (Baz's rule, deviates from the js): the
   whole world refreshes at DAWN — room snapshots expire, gatherables regrow, trees
   advance their stages, and (when farming ports) crops grow, all on the same farm_day
   roll. The js splits this: rooms/gatherables reset on the NOON dayNumber, only
   farming/trees on the dawn farmDay. Calendar/seasons/chats stay on the js noon day
   (sleep still lands you on a fresh calendar day). Switched sites: room_cache day
   keys, gather harvest stamps + the room_props taken() check (the two MUST agree).
   (2) CRITTERS (`actors/critters.rs`, js Entities.critter/firefly + spawnCritters):
   gentle-biome ambiance, re-rolled per visit, never saved/cached. By day 2-4 of
   rabbit/deer/bird/butterfly (butterflies favour the flowery biomes); startled ground
   critters bolt and VEER along walls (the rotate-the-heading escape); a startled BIRD
   commits toward the NEAREST TOWN (worldgen::towns::nearest_town, new) and flies off
   the map — the diegetic compass, one bird guaranteed when a town's in range. At
   night: 5-9 blinking FIREFLIES (figure-eight drift, eased aside when waded through,
   alpha-blinked dark between flashes) + maybe one prowler. Spawn seeding is the js
   mulberry32 stream (room+day+night); art is the js fillRect lists baked into frames
   (5 bird colours, wing-flap pairs; ground hop = whole-sprite bob). Spawner is
   REACTIVE (watches ActiveRoot change) so every room stand-up path gets ambiance with
   zero call-site edits; critters carry RoomActor and leave with the cast.
   DEVIATIONS (flagged): bird colours roll seeded (the js's one raw Math.random);
   critter-day = dawn day. Verified by tests + clippy; screenshots pending the
   occlusion state.)*
9b17. **Room state cache (js roomCache): same-day exact restore of the live layer)** —
   *(done — `app/room_cache.rs`. Step back into a room you left this morning and it is
   what you left: every surviving foe at its POSITION with its HEALTH (dead stays
   dead), every coin/item still on the ground with its remaining life (re-seated
   settled, no spawn-pop). At dawn snapshots go stale and the world refreshes (the js
   day rule). The STATIC layer was already record-driven and needed nothing: cut trees
   regrow via TreeGrowth stages, mined rocks / picked bushes suppress via GatherState's
   daily stamps, placed items + tomes via their permanent records.
   IMPROVE-DON'T-COPY: the js snapshots inside loadRoomEntities on leave; here a
   FixedPostUpdate system snapshots the CURRENT room every settled play tick (a few
   dozen copies — trivial), so no leave path (edge slide, door, DEATH — the scattered
   bag survives the respawn and is still lying there when you walk back) can miss one.
   The battle chain's spawns flush before FixedPostUpdate, so the snapshot always sees
   the tick's true end state; mid-slide it pauses (cur already names the NEXT room — a
   snapshot then would clobber it with an empty one, the gotcha). In-memory only, like
   the js: slot loads clear it and regen from records. spawn_room_mobs call sites
   became room_cache::spawn_or_restore (slide landing + swap_world_room; boot stays a
   fresh roll). DEVIATIONS (flagged): mob AI timers reset on restore (js re-seats the
   whole object, aggro included — ours re-aggros on sight); drops restore in TOWNS too
   (the js skips towns and loses them). REMAINING (separate increment): the js
   roomState SAVED kill/harvest records — killed mobs staying dead across a save/load
   within the same day; today a reload refreshes foes (the js pre-roomCache rule).)*
9b16. **Gifts (the GIVE flow: chooser menu, bag picker, taste/birthday reactions)** —
   *(done — `app/dialog.rs` under a new `Screen::Dialog`: the shared ACTION CHOOSER
   (js openChoice/UI.listBox — a centred auto-sized option box) + the pink-framed GIFT
   picker (js drawGiftPick: the WHOLE inventory is giftable, equipped tools included).
   INTERACT beside a villager with today's gift ungiven opens TALK / GIVE; if already
   gifted it collapses to a straight chat (the js 1-option rule). GIVE meets them first
   (gifting counts as the day's hello, js meetPerson), then the one-a-day gate ("YOU
   ARE TOO KIND. TOMORROW, MAYBE."). The gift itself (js verbatim): loved category
   +150 (know_love learned, "FOR ME? X - MY FAVORITE!", 2 hearts), disliked -30
   (know_hate, "OH. YOU REALLY SHOULD NOT HAVE."), polite +50; a positive gift on
   their BIRTHDAY counts x4 with the overjoyed line + 3 hearts, and gifting on the day
   marks know_bday. pts clamp 0..1000; lastGift saved (PersonRec grew the field, serde-
   default migrates). talk.rs refactored: ChatCtx SystemParam + meet_person/chat_with/
   spawn_heart/check_bday_learned shared with dialog.rs. NOTE (registry rule): the
   loved/hated categories (FISH/CROP/FOOD/GEM/TRINKET/MAP) match item `kind` fields —
   today's registry has none of those kinds, so every gift lands polite (+50, x4 on
   birthdays) until fish/crops/trinkets port; the scoring is already exact.
   DEVIATIONS (flagged): the js direct-gift shortcut (keyboard G / pad ▼ beside a
   villager) joins the bindings later; keeper-station chooser options (SHOP/REST as
   menu rows) join when needed. WRIFT_SHOT=talk now lands on the chooser;
   +WRIFT_GIFT=1 drives into the picker. (Shots pending — the black-frame occlusion
   state was active at build time; logic is unit/parity covered.))*
9b15. **Towns increment 4: PEOPLE (names/titles, TALK + speech bubbles, hearts, the
   PEOPLE codex tab, keeper discounts)** — *(done — FIVE layers. (1) IDENTITY:
   `people_data.rs` (GENERATED — tools/extract_people.mjs: 30F/30M name pools, 16
   keeper trades, the FRIENDLY/CONFIDANT dialogue) + `people.rs` (gender/name/title/
   taste/birthday/hearts/lineFor — every bit-mix pinned by tests/people_parity.rs, 10
   seeds x 3 suites against the LIVE js). Villagers gained pkey/pname: outdoor npcs
   "rx,ry:seed"; interior folk "i:rx,ry:kind:doorX,doorY:i" (the FIRST is the keeper
   and wears their trade — "BRAM THE INNKEEP"). (2) TALK: `app/talk.rs` — stand within
   26px + INTERACT: the day's first hello +20 pts (cap 1000; 100 = 1 heart), a pixel
   heart drifts up, the pink toast, birthdays shared at 2 hearts (calendar SEASONS),
   tastes slip out at 1+ hearts on the js 1-in-3 day roll; friends/confidants swap the
   stock line for tiered dialogue (deterministic per person+day). The ledger
   (PeopleLedger: pkey -> {pts, lastChat, knowBday/Love/Hate, town}) is a new SaveData
   field; SaveCtx hit Bevy's 16-param cap, so social state nests in SocialCtx
   (bought + people) — the documented pattern for save growth. (3) PRESS PRIORITY,
   improve-don't-copy: ActionState::consume() makes the js ladder explicit — door >
   book > counter > villager, each system eats the press it acts on, talk_tick ordered
   last. (4) THE BUBBLE (js drawNpcChat): nearest villager in 40px shows the name chip
   (grey/green/gold by tier); speaking (chatT 220) lifts the chip and opens the
   blue-bordered bubble. (5) THE PEOPLE TAB (codex/people_tab.rs replaces the stub):
   fold-able place banners (wanderers under THE WANDERING FOLK last), warmest-first
   rows with portraits (villager down-frames, cached per seed) + mini heart rows; the
   right pane = standard dex pane + gender mark (pixel Venus/Mars — the js draws
   vectors), tier, ten-heart row with the earned heart PART-FILLED, HEART N: x/100,
   last-spoke, tastes-a-mystery, birthday. Keeper discounts LIVE: shop::keeper_discount
   (3+ hearts 95%, 7+ 85%) reprices the shelf in stock_up and the inn's rest.
   DEVIATIONS (flagged): no option chooser yet (TALK is the only option; js collapses
   1-option menus anyway) — GIVE/QUEST arms join with gifts/quests; pane tail rows
   pulled up 2-4px (our canvas is 8px shorter, see CANVAS_H). WRIFT_SHOT=talk stages a
   town hello; WRIFT_TAB=PEOPLE + WRIFT_ROW=<n> stages the codex roster/detail.)*
9b14. **Towns increment 3: shops + services (vendor stock, BUY/SELL window, inn rest,
   chapel heal)** — *(done — FOUR layers. (1) STOCK: `stock_tables.rs` (GENERATED —
   tools/extract_stock.mjs pulls the EFFECTIVE post-init js tables; items.js pushes
   RECIPE_BPS blueprint ids into STOCK at module init, so the source literals are NOT
   the runtime tables) + `stock.rs` (shopStock/wildStock verbatim, same xxhash-mix rng
   draw-for-draw — tests/stock_parity.rs pins 50 kind×seed selections against the LIVE
   js). REGISTRY RULE: the selection runs on the FULL js lists, then drops unported ids
   — shelves converge to js bit-for-bit as items port. Registry grew by 7 (greaterpotion/
   elixir + leather/meat/gem/iron/string; ItemDef gained `icon_pal` for the js bake
   overrides — iron reuses the copper nugget grid in grey). (2) THE WINDOW: `app/shop.rs`,
   its own `Screen::Shop` state (js shopOpen flag — improve-don't-copy): 200x150 gold
   panel, BUY/SELL tabs on LT/RT, purse + prices in coin-metal-tinted denominations
   (coinStr: 100C=1S, 100S=1G), 9-row scroll list, rarity-coloured names, can't-afford
   red. Buys check unique/bag-room, toast, and mark the ware SOLD OUT FOR GOOD per shop
   (BoughtShop ledger, keyed "rx,ry,stock,doorX,doorY" — saved, new SaveData field).
   Sells move ONE unit at 40% list. (3) SERVICES: `app/services.rs` — the interior
   interact zones (extracted per def) drive a bottom-centre prompt bar ("F SHOP"/REST/
   BLESSING); INN rest costs ceil(40*(1+tier*0.5)) by zone depth; CHAPEL heals free;
   both toast. A takeable tome at your feet outranks the counter under it (js
   skipAction). (4) SLEEP: the js doSleep fade (38 fade / 36 hold / 38 wake + Z Z Z),
   full heal + clock jump to next morning + save at full black; `screen::playing` now
   also gates on Sleeping, so sleep IS the world-freeze (js update() returns early).
   DEVIATIONS (flagged): gear vendors' 4 procedural wares + wild weapon/armor draws join
   with the item-generator port; storage/wandtable/bard/lorevendor counters stay silent
   until their systems port; a bed just sleeps (spawn-point chooser joins with the
   respawn port); keeper-hearts/guild/festival discounts join with their systems.
   WRIFT_SHOT=shop stages it (WRIFT_TOWN="-13,-13" is a seed-1337 city centre with a
   GENERAL STORE; the stage walks the first vendor door and presses INTERACT at the
   counter).)*
9b13. **Controls overhaul: derived prompts everywhere, the D-pad shortcut cluster, and
   quick-access actions (Baz's design — deviates from the js pad map)** — *(done —
   THREE parts. (1) PROMPTS FOLLOW REBINDS: every on-screen prompt was already derived
   from Bindings at draw time; the one gap — the HUD slot labels + bottom hint only
   rebuilt on pad connect — now also rebuilds on bindings.is_changed(). The interact
   bubble re-derives per tick. (2) THE D-PAD IS THE SHORTCUT CLUSTER: movement rides the
   left stick in free roam; defaults ▲ INTERACT, ▼ INVENTORY, ◀ MAP, ▶ SKILL TREE. In
   ANY non-Play screen the js dpadDirs rule applies (input::DpadDirs, set from the
   Screen state): the D-pad feeds the four direction actions for menu nav and its
   shortcut bindings go quiet, so ▼ scrolls a menu instead of reopening the inventory.
   (3) EVERY codex tab + slide-out page is a bindable quick-access action (Calendar/
   People/Guilds/Mobs/ItemDex/Songs/Awards/Stats/Lore/Wriftheart + Craft/Status — 12 new
   actions, UNBOUND by default, all on the CONTROLS page, all persisted by slug in
   settings.json): press one in free roam and the codex/slide-out opens straight to that
   page. VERIFIED: the CONTROLS page shows the new map (moves '--' on pad, ▲▼◀▶ glyph
   column); 29 rebindable rows scroll.)*
9b12. **Lore tomes + the LORE codex tab** — *(done — `lore_books.rs` (GENERATED:
   tools/extract_lore.mjs pulls all 107 tomes verbatim — id/title/category/author/
   where-pool/spine colour/text — plus the spine grid and the js BOOK_PLACES pools +
   bookIdFor pick). PICKUPS: PickupKind::Book — the category-coloured spine over its own
   glow, a wide grab zone (books sit on furniture), sits forever, collects into
   GatherState.tomes (saved; read tomes never respawn anywhere) with the NEW TOME toast.
   SPAWNS: the four authored Emberfall fragments in the ruin; library interiors keep 4
   free shelf tomes; ~1 in 3 other buildings keeps one on its furniture (the js
   deterministic per-location rolls, book_spots re-extracted into interiors_art).
   THE LORE TAB (replacing the stub): the js two-column study — 6-col tome shelf ('?'
   spines until found) + the reading pane (title/byline rule, wrapped + PAGINATED text,
   PAGE x/y footer); confirm picks a found book UP (gold frame, arrows/confirm turn
   pages, off the last page it sets down), Sort/Inventory flip pages in either mode,
   closing the codex sets any open book down. Verified on screen (shelf + pane + the
   ground fragments + toast). REMAINING: the library scholar's tome purchase (shops),
   dungeon/camp/castle wild spots (their systems), the WRIFTHEART tab (relics).
   HARNESS FIX: shot scenes that jump Title->Codex/Pause now run cleanup_title first
   (the title UI ghosted under the codex in shots; unreachable in real play).)*
9b11. **Emberfall's loose stones (NEW, Baz's addition — not in the js)** — *(done — five
   stone pickups scattered in the village rubble, free pickings for a fresh hero.
   Built on a new PLACED-pickup primitive (gather::spawn_placed_item): a ground item
   with no spawn pop and NO despawn timer that waits until taken — and once taken stays
   gone FOREVER (GatherState.placed, a permanent per-room tile record, saved per slot;
   Baz's spec: no respawn, unlike the daily gatherables).
   PLAYTEST FIX (death doubles): the standing hero stayed visible beside his corpse —
   sync_player_sprite ran once more before the Screen::Dead transition applied and
   re-showed the body it had just hidden. Visibility now keys off hp <= 0 directly, so
   the sync can't win that race.)*
9b10. **Announcement banners (towns / regions / interiors)** — *(done — `app/banners.rs`,
   port of game.js townBanner/biomeBanner/interiorBanner + BIOME_INFO + getTownName.
   Landing a room-slide announces a TOWN by name every entry (scale-3 gold + the
   '- A QUIET VILLAGE -' sub; EMBERFALL shows '- IN RUINS -'), or the REGION when the
   biome key changes (all 24 BIOME_INFO entries verbatim; towns swallow the region
   banner). Town names are the js procedural pool (20 prefixes x 12 suffixes, unique per
   game, coord-hashed start index, resolved at the town footprint CENTRE so districts
   share one name) and SAVED per slot. Entering a building raises the quiet title plaque
   (dark bar + gold rules). Fades ride the js profiles per kind; swap_world_room anchors
   the region SILENTLY (loads/respawns/exits never announce). Verified on screen:
   EMBERFALL-IN RUINS, GOLDFORD-A QUIET VILLAGE, the FARM STALL plaque.)*
9b9. **The burnt home village (EMBERFALL)** — *(done — the room one EAST of spawn
   (js isHomeVillage = startRX+1) is the hero's home village, burned in the opening:
   room_props overlays the js buildRuinedVillage scene on the normal room — six radial
   scorch-earth decals (CPU-baked, stretched per radius), the five gutted shells at their
   fixed spots (ruinGrid char grids extracted per-seed into buildings_art::RUINS; only
   the rubble mound blocks), eight skeletons in the streets (SKELETON_DOWN grid), and
   eleven pieces of smouldering wreckage from the clutter bank. Mobs are already absent
   there in the worldgen stream (parity-pinned), matching the js safe-haven rule.
   REMAINING: the EMBERFALL town banner + '- IN RUINS -' sub (banners unported), the four
   authored lore-book fragments (lore unported), ember night-glow (lighting pass).)*
9b8. **Towns increment 2: interiors + enter/exit + INTERACT** — *(done —
   `app/interior.rs` + `actors/interiors_art.rs` (GENERATED: tools/extract_interiors.mjs
   records every fillRect Interiors.make paints — a recording-canvas shim — into per-kind
   DISPLAY LISTS, ~800 rects each, plus the solid grid, exit mat, spawn, interactables,
   lights and folk; 22 kinds). Stand in a doorway + press INTERACT (the js action, newly
   ported: F / pad D-pad-up, rebindable, CONTROLS row included) and the overworld room
   swaps for the scene: one rasterized image, a foreground bottom-wall strip (Sprite
   rect-crop) the player tucks behind on the way out, interior solidity as the live
   RoomGrid, and the building's folk as villagers (keeper holds their post + faces you,
   js `still`; identities from the js door-salted iseed formula). Walk the doorway mat to
   leave — back on the doorstep, homeCooldown 45. Doors are DERIVED from the room's
   entity layout on the press (no live door state). Guards: no room-slides indoors, no
   autosaves indoors (the position is interior-local), interiors read daylit,
   swap_world_room clears Inside (a load/death indoors can't strand the flag — SwapCtx
   carries it). WRIFT_SHOT=interior (+WRIFT_TOWN) walks in headlessly; farmstall + home
   eyeballed. REMAINING (inc 3): the interactables (shop/rest/heal/bard/lorevendor),
   speech bubbles + people names, wild 'shop'/caveshop doors on the overworld,
   per-building furniture reseed, interior warm lights.)*
9b7. **Towns alive, increment 1: townEntities + storefronts + villagers** — *(done —
   `worldgen/town_entities.rs`: the js townEntities ported draw-for-draw (district
   recipes for market/homes/green/farmrow/quarter/yards/hall, plot shuffles, wells +
   braziers, orchards, worked field rows, deco scatter, the market's reserved Tillers
   stall pitch, and per-district FOLK with stable identity seeds + chatter lines) —
   pinned by tests/town_entities_parity.rs: 316 descriptors across 12 town rooms in 3
   seeds match the live JS bit-for-bit, npc seeds included. tools/extract_towns.mjs
   machine-extracts the 19 storefront grids (the entities.js architecture-kit output),
   the well and the 2-frame brazier into `actors/buildings_art.rs` (PropArt bakes them).
   room_props spawns: 48x48 fronts anchored js-style with full-mass blockers (doors open
   with the interiors port), the well, flickering braziers (TorchAnim on the shared
   clock), and VILLAGERS — `actors/villager.rs` + hero::random_look (the js NPC look
   pools + LCG, so every villager keeps their face): amble near home on a 44px leash,
   stop and face you inside 40px, gait bob, staggered wander clocks, riding the room
   root through slides. WRIFT_SHOT=town (WRIFT_TOWN="rx,ry" picks the room) — two
   districts eyeballed on screen. DEVIATIONS/REMAINING: villagers don't block the
   player yet (static-blocker model), no dialogue bubbles/names (people.js port), no
   interiors/enter-doors, no shop economy, guildhall + stallspot spawn nothing, torches
   don't cast night light yet.)*
9b6. **YOU DIED + deathlines** — *(done — `app/death.rs` + `deathlines.rs` (GENERATED:
   tools/extract_deathlines.mjs pulls all 1007 epitaphs verbatim from js/deathlines.js).
   Dying costs half your coin + all progress toward the next level (the level stays), and
   the BAG scatters as ground pickups around the corpse (equipped slots + gear stay).
   Screen::Dead freezes the world under the js sequence: dark fade + grey wash (DEVIATION:
   the js 'saturation' canvas composite has no sprite equivalent), growing two-tone blood
   pool, the hero rotated on his side, YOU DIED (scale 3, blood red) fading in over a
   never-repeats epitaph and the itemized toll, then CONTINUE / TITLE SCREEN. Respawn =
   the start room, full HP, immediate save; TITLE runs the respawn first (the js rule) so
   CONTINUE on the title resumes the respawned save. Respawn reuses the loader's
   swap_world_room (one world-swap path for slots AND death). GOTCHAS learned: a
   next.set() transition lags a frame, so one-shot triggers (check_death) must guard
   against re-firing (the double-run zeroed the toll numbers); ground pickups + glows
   carry RoomActor, so the actor sweep already despawns them (an explicit pickup sweep
   double-despawned). WRIFT_SHOT=death drives kill -> menu -> CONTINUE -> respawn
   headlessly (WRIFT_HOLD=1 freezes on the menu for screenshots); verified on screen.)*
9b4. **Title screen + 4-slot saves** — *(done, VISUALS UNVERIFIED (black-frame episode
   was active; re-shoot `WRIFT_SHOT=title` when it lifts) — `app/title/` + screen.rs:
   the game now BOOTS into Screen::Title; the world spawns frozen underneath (loaded from
   the NEWEST slot) and the title covers it. flyover.rs: the js terrain drift, each
   visible room CPU-baked to ONE image (tiles + dressing + tree/bush/boulder props,
   painter-sorted) at a single z over the world — the live room spawner's z-bands would
   interleave with the play world. mod.rs: CONTINUE / LOAD GAME (2+ saves) / NEW GAME /
   OPTIONS / EXIT GAME on the rounded soft panel + version corner; slots.rs: the picker
   with per-slot HERO/LVL/season/day cards, double-press delete + overwrite guards;
   crawl.rs: the idle attract-mode story crawl (Relics.STORY verbatim, looping, edge
   fades). loader.rs: ONE LoadSlot message hot-swaps the live world (apply_to the save or
   reset-to-defaults, room root/cast/pickups respawn, player repositioned, slot claimed).
   save.rs went multi-slot: save1-4.json + name/ts meta (#[serde(default)] so old files
   keep loading — Baz's save1.json migrates as slot 1), latest-ts boot, ActiveSlot
   retargets autosaves. The pause menu gained QUIT TO TITLE (save-first) and the title's
   OPTIONS opens it settings-only (Screen::TitleOptions, GAME tab hidden). GOTCHA learned:
   the initial state's OnEnter runs BEFORE PreStartup commands flush — resources it reads
   must be registered at plugin-build time. DEVIATIONS (flagged): no backdrop blur (no
   ctx.filter; sharp drift under the same gradient), flyover water frozen at frame 0,
   JOIN A FRIEND absent (co-op post-parity), NEW GAME starts a default HERO directly (the
   creator is its own milestone), slot cards show no shard count yet (relics unported).
   WRIFT_SHOT scenes: `title`, `newgame` (headless loader smoke test); play-side scenes
   now force Screen::Play first.)*
9b3. **Pause menu parity + settings** — *(done — `app/menu/` (mod/tabs/controls) +
   `settings.rs` + `persist.rs`: the js tabbed 280x180 panel verbatim — GAME (RESUME /
   SAVE / AUTOSAVE / EXIT GAME) · VIDEO (pixel perfect / screen shake / brightness /
   reduce flashing / fullscreen) · SOUND · CONTROLS (ACTION/KEY/PAD rebind table,
   scrolling, next-input capture for keys AND pad buttons, RESET DEFAULTS). Settings +
   custom bindings persist to settings.json (bindings as action-slug -> key-label string
   rows; unknown rows drop quietly — the save-file rule). Consumers wired: BRIGHTNESS
   lifts the lighting overlay's ambient alpha, PIXEL PERFECT drives the canvas scaler
   (integer FLOOR — round could crop — vs free fractional fit, the js default), AUTOSAVE
   gates the heartbeat + pause checkpoint while the SAVE row always writes (SaveRequest
   message). SCREEN SHAKE / REDUCE FLASHING / SOUND persist now and get consumed when
   their systems land. DEVIATIONS (on screen, flagged to Baz): EXIT GAME replaces QUIT TO
   TITLE + the save-slot picker until the title screen lands (SAVE flashes SAVED!);
   FULLSCREEN is a real toggle where the js printed an OS-shortcut hint its webview
   couldn't act on; the CO-OP tab is post-parity scope. Capture gotcha solved in
   debug_shot: a state transition lags next.set by a frame — an eager second Pause press
   toggles the menu straight back shut (the stage system spaces its synthetic presses).
   WRIFT_SHOT=pause + WRIFT_TAB=<GAME|VIDEO|SOUND|CONTROLS> stages it.)*
9b. **Day/night + lighting (darkness pass)** — *(increment 1 done — `app/lighting.rs`:
   the js time-of-day cycle (dayDarkness cosine over DAY_LEN, frame 0 = noon; ambient
   alpha DAY_MIN 0.16 -> NIGHT_MAX 0.8; night-blue tint [10,14,38]) driving a
   CPU-RASTERIZED darkness overlay — exactly how js/lighting.js works (it paints an
   offscreen canvas per frame): fill the tint at ambient alpha, then cut a radial hole per
   light with the js destination-out gradient (stops 1.0 / 0.7 @ 55% / 0.0, sequential =
   multiplicative). 83k pixels/frame is sub-ms; rebuilt only when the alpha or a light
   moves; drawn at z 13 over the play area + the strip above it (so canopy spill can't
   stay day-bright), under the loot feed (13.5) and HUD. Lights v1 = ground pickups (r 16);
   the hero has NO overworld light by design (that's lantern gear; dungeons always grant
   one). bevy_light_2d was evaluated and REJECTED: it tops out at Bevy 0.18 (we're on
   0.19) and its shader falloff wouldn't reproduce the js gradients. Debug:
   WRIFT_TIME=0..1 pins the phase (0.5 = midnight). DARK_GAIN=1.0 is the linear-blend
   calibration knob — tune after a side-by-side with the js at night. REMAINING: the
   additive glow pass (step 2 — needs a custom additive material; baked pickup glows
   cover today's content), weather/dimAmbient hooks, dungeon/interior/cave ambients,
   night-mob spawns, lantern/coneLight gear.)*
10. **Parity grind** — dungeons, mobs, codex, farming, songs, quests, …

Stop and playtest after 6 — that's the first point where it *feels* like the game.

### Parity-test pattern (reuse it)

World-gen determinism proved the workflow for porting anything the JS computes: extract the JS
function's VERBATIM source, run it in node over a table of inputs (include the mean cases —
negative coords, boundary values), emit the results as a generated Rust golden file, and assert
the Rust port reproduces them. Do this for every deterministic system (loot rolls, dungeon
layout, item generation, biome maps) — it turns "looks right" into "provably identical".

## Invariants — the things that must NOT drift

- **384x208, 80px sidebar, 19x13 tile room.** Every layout constant assumes it. (The
  js canvas is 216 tall with 4px dead bands framing the play field; Baz asked for the
  canvas to hug the content — the window's integer scaler letterboxes outside instead.)
- **The palette is canon.** `assets::palette()` must stay byte-identical to `js/assets.js`.
- **The font renders `A-Z 0-9 . , - + % : ; ' ! / > < ( ) ? *` and space only.** No lowercase,
  no em-dash. Ported strings that break this render as gaps.
- **World gen determinism.** The JS world is a pure function of (seed, rx, ry) via a specific
  FNV-ish hash + a `mulberry32`-style PRNG. Rust's `HashMap`/`rand` will NOT reproduce it —
  the hash and PRNG must be ported **arithmetically exactly**, including `Math.imul`
  (= `i32::wrapping_mul`) and `>>> 0` (= `as u32`). Get this wrong and every existing world
  changes. Port `hash()`/`makeRng()` first and diff them against the JS on a table of known
  inputs before building anything on top.
- **Balance philosophy: PURPLE IS ENDGAME.** Rarity odds, drop scarcity, and the single
  weighting formula come over unchanged.
- **The Baz easter egg** (naming the hero "Baz" -> bald) survives the port.

## Gotchas already hit

- **Even-sized textures only** for centre-anchored sprites at integer positions: an odd
  width puts the sprite's edges on half-pixel boundaries and the rasterizer smears the
  pattern (rows double/drop — circles turn into clovers). `font::bake_text` pads to even
  for this reason; the skill-tree shapes learned it the hard way. When authoring any new
  sprite/shape, make both dimensions even.
- **Alphas copied from the JS are too bright here**: canvas blends in sRGB, Bevy in LINEAR
  — translucent overlays/glows need roughly half the JS alpha (codex overlay went fully
  opaque; the skill tree's nebulae/halos run at ~50% of the JS values).
- **Never enable MSAA on the render-target camera**: Msaa::Sample4 on the pixel camera
  renders BLACK intermittently on Metal (worked twice, then every frame black until
  reverted). Canvas-style anti-aliasing comes from textures instead: supersampled coverage
  alpha for shapes, and LINEAR-sampled edge-fade strips for rotated lines (see the skill
  tree's `line` strip in skills_tab.rs).
- **Bevy Bloom cannot do selective sprite glow** (tried + reverted 2026-07-16): the plan
  was hdr + Tonemapping::None + a prefilter threshold of 1.0 so only emissive sprites
  (colour > 1.0) bloomed. Dead end: bloom's FIRST downsample karis-averages (firefly
  suppression) BEFORE the threshold, crushing everything toward <=1 — threshold >=1.0
  passes NOTHING, and 1-4px HDR sparkles are precisely the fireflies it suppresses. (The
  pipeline itself worked: threshold 0 washed the whole frame, no black frames, palette
  unchanged under Tonemapping::None — so bloom remains viable for a WHOLE-SCENE pass like
  the lighting.js port, just not for per-sprite glints.) Glows and sparkles are baked
  radial sprites in gather.rs instead.

- **Coordinate flip.** JS: top-left origin, +Y down, sprites drawn from their corner. Bevy:
  centre origin, +Y up, sprites anchored centre. Use `at()` in `main.rs` — don't hand-roll it.
- **Bevy 0.19 renamed events to messages** — it's `MessageReader<WindowResized>`, not
  `EventReader`. The API churns between Bevy versions; when unsure, read the real source at
  `~/.cargo/registry/src/*/bevy-0.19.0/examples/` rather than trusting memory or old docs.
- **`Image::new_fill` + `pixel_bytes_mut`** is the `ctx.createImageData` analog (see
  `examples/2d/cpu_draw.rs`); the low-res canvas pattern is `examples/2d/pixel_grid_snap.rs`.

## Running

```sh
cargo run          # first build is slow (Bevy, ~500 crates); after that it's quick
```

### Visual verification (WRIFT_SHOT)

For anything geometric on screen (rotations, anchors, z-layering), do NOT argue from math —
shoot it and look. `src/app/debug_shot.rs` (inert without the env var):

```sh
WRIFT_SHOT=1 WRIFT_SHOT_PATH=/tmp/swings.png cargo run   # opens a window ~2s, then exits
```

It freezes the sim and stages a scene, screenshots the window, and quits on its own.
Scenes: `swings` (every sword/axe facing at three sweep phases, tinted RED=start / white /
CYAN=end, bodies at the real z layers), `codex` (MAP tab over a seeded visited set), `mobs`
(the bestiary tab). Add a scene whenever a new visual system needs eyeballing. This caught
what three rounds of on-paper angle derivation could not settle. NOTE: the capture waits
until frame 90 — the first launch after a fresh compile renders black around frame 40
(Metal still compiling pipelines), so never capture earlier. SEPARATE FAILURE MODE: the
machine sometimes enters a PERSISTENT black-capture episode — every scene, every run,
byte-identical 46,449-byte all-black PNGs, no log errors, NOT fixed by warm re-runs or a
later capture (WRIFT_SHOT_FRAME=400 still black; the env var overrides the capture frame).
It comes and goes on its own (seen at 18:54, ~20:59, and 21:55+ on 2026-07-15, each time
recovering later) and does NOT indicate a code regression — check the sizes of past shots
in the scratchpad before bisecting code. The game itself renders fine when run normally.

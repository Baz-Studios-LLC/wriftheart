// extract_mobs.mjs — biome-mob art + bestiary extraction from the live JS. Output:
//   src/actors/mobs_art.rs — GENERATED: the base-roster sprite grids (right-facing; Rust
//                            flips left at bake time like the goblin frames) + palette
//                            overrides + the BESTIARY names/descs for the ported kinds.
//
// Pattern (see extract_props.mjs): eval enemies.js verbatim with Assets.bake stubbed to an
// identity capture, and inject module internals into the IIFE return.
import { readFileSync, writeFileSync } from 'fs';

const JS = '/Users/brettbazaar/Code/Game Dev/Wriftheart/js';
const RS = '/Users/brettbazaar/Code/Game Dev/wriftheart-rs';
const src = (f) => readFileSync(`${JS}/${f}`, 'utf8');

// --- Stubs ---
const palMatch = src('assets.js').match(/const PALETTE = \{[\s\S]*?\n  \};/);
const PALETTE = eval(`(() => { ${palMatch[0].replace('const PALETTE =', 'return')} })()`);
const flip = (g) => g.map((r) => r.split('').reverse().join(''));
globalThis.Assets = {
  PALETTE,
  bake: (grid, pal) => ({ grid, pal: pal || null }),
  flipH: (g) => (g && g.grid ? { grid: flip(g.grid), pal: g.pal } : flip(g)),
};
globalThis.clamp = (v, a, b) => Math.max(a, Math.min(b, v));
globalThis.window = globalThis;
globalThis.Sound = { sfx: () => {} };
globalThis.Entities = { luckMult: () => 1 };
globalThis.Room = { PX_W: 304, PX_H: 208 };
globalThis.Player = { buildFrames: () => ({ down: [{}], up: [{}], left: [{}], right: [{}] }), randomLook: () => ({}) };
const noopCtx = new Proxy({}, { get: (t, k) => (k === 'canvas' ? fakeCanvas() : () => noopCtx), set: () => true });
const fakeCanvas = () => ({ width: 16, height: 16, getContext: () => noopCtx });
globalThis.document = { createElement: fakeCanvas };

// enemies.js: inject the sprite consts + BESTIARY into the IIFE return.
let src2 = src('enemies.js');
const retAt = src2.lastIndexOf('\n  return {');
src2 =
  src2.slice(0, retAt) +
  `\n  globalThis.__MOBS = { S_BOAR, S_WASP_A, S_WASP_B, S_THORN, S_THORN_DORM,
     WOLF_FRAMES, S_BEAR_A, S_BEAR_B, S_SPIDER, S_SCORP_A, S_SCORP_B, S_BURROW,
     S_VULT_A, S_VULT_B, S_GOLEM, S_BAT_A, S_BAT_B, S_HURL, BESTIARY,
     S_ZOMBIE, S_ZOMBIE_B, S_SKELETON, S_ARCHER, S_ARROW, SLIME_ELEMS, S_FROG_FR,
     S_LEECH_A, S_LEECH_B, S_GNAT_A, S_GNAT_B, S_LURKER_A, S_LURKER_B,
     S_GHOUL, S_WRAITH, S_REVENANT, S_FROSTMITE, S_ICETROLL, S_FROSTWYRM,
     S_CINDERHOUND, S_CHARBRUTE, S_PYREWRAITH, S_SPORELING, S_MYCONID,
     S_SPOREMOTHER, S_CHAOSWISP, S_VOIDLING, S_RIFTLORD,
     S_CULTIST, S_VINESNARE_UP, S_VINESNARE_HID, S_BELLSNAIL_IN, S_BELLSNAIL_OUT,
     S_BOGLIGHT, S_GRAVEWARDEN_A, S_GRAVEWARDEN_B, S_HONEYDRONE_A, S_HONEYDRONE_B,
     S_MIREFLY, S_PALEHOWLER_A, S_PALEHOWLER_B, S_SALTSTATUE_STONE, S_SALTSTATUE_WALK,
     S_SANDMAW_A, S_SANDMAW_B, S_STORMCALLER_A, S_STORMCALLER_B,
     S_SWITCHSHADE_A, S_SWITCHSHADE_B, S_TIDECRAB_A, S_TIDECRAB_B,
     S_GLIMMER_A, S_GLIMMER_B, S_WITHERHEART_A, S_WITHERHEART_B };` +
  src2.slice(retAt);
eval(src2);
const M = globalThis.__MOBS;

// --- Emit ---
const esc = (s) => s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
const gridRs = (g) => `&[${g.grid.map((r) => `"${r}"`).join(', ')}]`;
const palRs = (g) => {
  if (!g.pal) return '&[]';
  const entries = Object.entries(g.pal).map(([ch, hex]) => `('${ch}', 0x${hex.slice(1)})`);
  return `&[${entries.join(', ')}]`;
};
const frameRs = (g) => `MobFrame { grid: ${gridRs(g)}, pal: ${palRs(g)} }`;

// The ported roster: kind -> its animation frames (right-facing; wolf is per-facing).
const SIMPLE = {
  boar: [M.S_BOAR],
  wasp: [M.S_WASP_A, M.S_WASP_B],
  thornling: [M.S_THORN],
  bear: [M.S_BEAR_A, M.S_BEAR_B],
  spider: [M.S_SPIDER],
  scorpion: [M.S_SCORP_A, M.S_SCORP_B],
  burrower: [M.S_BURROW],
  vulture: [M.S_VULT_A, M.S_VULT_B],
  golem: [M.S_GOLEM],
  bat: [M.S_BAT_A, M.S_BAT_B],
  hurler: [M.S_HURL],
  zombie: [M.S_ZOMBIE, M.S_ZOMBIE_B],
  skeleton: [M.S_SKELETON],
  archer: [M.S_ARCHER],
  leech: [M.S_LEECH_A, M.S_LEECH_B],
  gnat: [M.S_GNAT_A, M.S_GNAT_B],
  lurker: [M.S_LURKER_A, M.S_LURKER_B],
  ghoul: [M.S_GHOUL],
  wraith: [M.S_WRAITH],
  revenant: [M.S_REVENANT],
  frostmite: [M.S_FROSTMITE],
  icetroll: [M.S_ICETROLL],
  frostwyrm: [M.S_FROSTWYRM],
  cinderhound: [M.S_CINDERHOUND],
  charbrute: [M.S_CHARBRUTE],
  pyrewraith: [M.S_PYREWRAITH],
  sporeling: [M.S_SPORELING],
  myconid: [M.S_MYCONID],
  sporemother: [M.S_SPOREMOTHER],
  chaoswisp: [M.S_CHAOSWISP],
  voidling: [M.S_VOIDLING],
  riftlord: [M.S_RIFTLORD],
  cultist: [M.S_CULTIST],
  mirefly: [M.S_MIREFLY],
  boglight: [M.S_BOGLIGHT],
  honeydrone: [M.S_HONEYDRONE_A, M.S_HONEYDRONE_B],
  gravewarden: [M.S_GRAVEWARDEN_A, M.S_GRAVEWARDEN_B],
  palehowler: [M.S_PALEHOWLER_A, M.S_PALEHOWLER_B],
  sandmaw: [M.S_SANDMAW_A, M.S_SANDMAW_B],
  stormcaller: [M.S_STORMCALLER_A, M.S_STORMCALLER_B],
  switchshade: [M.S_SWITCHSHADE_A, M.S_SWITCHSHADE_B],
  tidecrab: [M.S_TIDECRAB_A, M.S_TIDECRAB_B],
  // Two-state (frame 0 = ACTIVE/out, frame 1 = DORMANT/stone) — the AI picks by state.
  vinesnare: [M.S_VINESNARE_UP, M.S_VINESNARE_HID],
  bellsnail: [M.S_BELLSNAIL_OUT, M.S_BELLSNAIL_IN],
  saltstatue: [M.S_SALTSTATUE_WALK, M.S_SALTSTATUE_STONE],
  glimmerling: [M.S_GLIMMER_A, M.S_GLIMMER_B],
  witherheart: [M.S_WITHERHEART_A, M.S_WITHERHEART_B],
};
// The slime family shares one grid; each element recolours it (js SLIME_ELEMS).
for (const [elem, E] of Object.entries(M.SLIME_ELEMS)) {
  SIMPLE[E.type] = [E.spr];
}
// The frog's three poses (idle / hop / tongue agape).
SIMPLE.frog = M.S_FROG_FR;

let out = `// GENERATED by tools/extract_mobs.mjs from js/enemies.js — do not edit.
//! mobs_art.rs — base-roster mob sprites (right-facing grids; flip left at bake) + the
//! bestiary names/descs. Wolf carries per-facing frames like the goblins.

pub struct MobFrame {
    pub grid: &'static [&'static str],
    pub pal: &'static [(char, u32)],
}

`;
for (const [k, frames] of Object.entries(SIMPLE)) {
  out += `pub static ${k.toUpperCase()}_FRAMES: &[MobFrame] = &[${frames.map(frameRs).join(', ')}];\n`;
}
out += `pub static THORN_DORM: MobFrame = ${frameRs(M.S_THORN_DORM)};\n`;
out += `pub static ARROW_SPR: MobFrame = ${frameRs(M.S_ARROW)};\n`;
for (const f of ['down', 'up', 'right']) {
  out += `pub static WOLF_${f.toUpperCase()}: &[MobFrame] = &[${M.WOLF_FRAMES[f].map(frameRs).join(', ')}];\n`;
}
out += `\n/// Every simple-frame kind -> its animation strip (wolf is per-facing, above).\n`;
out += `pub static ALL_FRAMES: &[(&str, &[MobFrame])] = &[\n`;
for (const k of Object.keys(SIMPLE)) out += `    ("${k}", ${k.toUpperCase()}_FRAMES),\n`;
out += `];\n`;
const KINDS = Object.keys(SIMPLE).concat(['wolf']);
out += `\n/// (kind, bestiary name, bestiary desc) — js Enemies.BESTIARY, the ported subset.\n`;
out += `pub static BESTIARY_INFO: &[(&str, &str, &str)] = &[\n`;
for (const k of KINDS) {
  const b = M.BESTIARY[k];
  out += `    ("${k}", "${esc(b.name)}", "${esc(b.desc)}"),\n`;
}
out += `];\n`;
writeFileSync(`${RS}/src/actors/mobs_art.rs`, out);
console.log(`mobs_art.rs written: ${Object.keys(SIMPLE).length + 1} kinds`);

// Extract the deathlines pool from the frozen JS reference into src/deathlines.rs.
// Verbatim data port: evals the LINES array source (string literals + comments only).
import { readFileSync, writeFileSync } from 'node:fs';
const src = readFileSync(new URL('../../Wriftheart/js/deathlines.js', import.meta.url), 'utf8');
const m = src.match(/const LINES = \[([\s\S]*?)\n  \];/);
if (!m) throw new Error('LINES array not found');
const lines = eval('[' + m[1] + ']');
const esc = (s) => s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
const out = `//! deathlines.rs — the random epitaph pool under "YOU DIED" (GENERATED from
//! js/deathlines.js by tools/extract_deathlines.mjs — do not hand-edit; the font renders
//! A-Z 0-9 + limited punctuation, which the source pool already respects).

pub const LINES: &[&str] = &[
${lines.map((l) => `    "${esc(l)}",`).join('\n')}
];
`;
writeFileSync(new URL('../src/deathlines.rs', import.meta.url), out);
console.log('extracted', lines.length, 'lines');

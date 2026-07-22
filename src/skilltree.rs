//! skilltree.rs — the PASSIVE TREE data + logic (port of js/skilltree.js): a constellation
//! radiating from a free central START. Eight themed branches at 45°, each minors -> a
//! notable -> a two-lane loop -> a second notable -> a KEYSTONE with a real tradeoff;
//! adjacent branches' first notables are ring-linked so routing matters.
//!
//! Pure data + queries here (node layout pinned to the JS by tests/skilltree_parity.rs);
//! rendering/interaction live in app/slideout/skills_tab.rs.

use std::collections::HashSet;
use std::sync::OnceLock;

pub const REFUND_COST: u32 = 40; // coin per refunded point (leaf nodes only)

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub kind: &'static str, // start | minor | notable | keystone
    pub name: &'static str,
    pub stats: Vec<(&'static str, f64)>,
    pub cost: u32,
    pub links: Vec<usize>,
}

/// JS Math.round (half toward +infinity) — the layout coordinates round through this.
fn js_round(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

struct Builder {
    nodes: Vec<Node>,
}

#[allow(clippy::too_many_arguments)] // add() mirrors the JS add(id,x,y,kind,name,stats,cost)
impl Builder {
    fn add(&mut self, id: String, x: f64, y: f64, kind: &'static str, name: &'static str, stats: Vec<(&'static str, f64)>, cost: u32) -> usize {
        self.nodes.push(Node { id, x: js_round(x), y: js_round(y), kind, name, stats, cost, links: Vec::new() });
        self.nodes.len() - 1
    }
    fn idx(&self, id: &str) -> usize {
        self.nodes.iter().position(|n| n.id == id).unwrap()
    }
    fn link(&mut self, a: usize, b: usize) {
        if !self.nodes[a].links.contains(&b) {
            self.nodes[a].links.push(b);
        }
        if !self.nodes[b].links.contains(&a) {
            self.nodes[b].links.push(a);
        }
    }
    /// One branch — REDESIGNED (Baz, deviates from the js `branch`): the geometry,
    /// costs and links keep the js constellation, but every slot is now a BESPOKE
    /// node — no two nodes in a branch repeat a reward, and each is chunkier than
    /// the old sprinkle. Slots walk: m1, m2, notable, split lanes (a1/a2 - b1/b2),
    /// notable, m3, keystone (the keystone still pays for its power).
    #[allow(clippy::too_many_arguments)] // it IS the branch definition's arity
    fn branch(
        &mut self,
        key: &str,
        deg: f64,
        m1: (&'static str, Vec<(&'static str, f64)>),
        m2: (&'static str, Vec<(&'static str, f64)>),
        a1: (&'static str, Vec<(&'static str, f64)>),
        a2: (&'static str, Vec<(&'static str, f64)>),
        b1: (&'static str, Vec<(&'static str, f64)>),
        b2: (&'static str, Vec<(&'static str, f64)>),
        n1: (&'static str, Vec<(&'static str, f64)>),
        n2: (&'static str, Vec<(&'static str, f64)>),
        m3: (&'static str, Vec<(&'static str, f64)>),
        ks: (&'static str, Vec<(&'static str, f64)>),
    ) {
        let a = deg * std::f64::consts::PI / 180.0;
        let (dx, dy) = (a.cos(), a.sin());
        let (px, py) = (-dy, dx);
        let p = |d: f64, s: f64| (dx * d + px * s, dy * d + py * s);
        // Max HP stays SPRINKLED on the notables (the js note): the spine of every
        // build earns a little life on the way out.
        let hp_add = |s: &Vec<(&'static str, f64)>, h: f64| {
            let mut o = s.clone();
            if let Some(e) = o.iter_mut().find(|(k, _)| *k == "maxhp") {
                e.1 += h;
            } else {
                o.push(("maxhp", h));
            }
            o
        };
        let mut n = |sfx: &str, pos: (f64, f64), kind, name, stats, cost| {
            self.add(format!("{key}{sfx}"), pos.0, pos.1, kind, name, stats, cost)
        };
        let m1i = n("m1", p(26.0, 0.0), "minor", m1.0, m1.1, 1);
        let m2i = n("m2", p(50.0, 0.0), "minor", m2.0, m2.1, 1);
        let n1i = n("n1", p(76.0, 0.0), "notable", n1.0, hp_add(&n1.1, 4.0), 2);
        let a1i = n("a1", p(102.0, -22.0), "minor", a1.0, a1.1, 2);
        let a2i = n("a2", p(128.0, -22.0), "minor", a2.0, a2.1, 3);
        let b1i = n("b1", p(102.0, 22.0), "minor", b1.0, b1.1, 2);
        let b2i = n("b2", p(128.0, 22.0), "minor", b2.0, b2.1, 3);
        let n2i = n("n2", p(152.0, 0.0), "notable", n2.0, hp_add(&n2.1, 6.0), 3);
        let m3i = n("m3", p(176.0, 0.0), "minor", m3.0, m3.1, 4);
        let ksi = n("ks", p(202.0, 0.0), "keystone", ks.0, ks.1, 5);
        let start = self.idx("start");
        for (x, y) in [(start, m1i), (m1i, m2i), (m2i, n1i), (n1i, a1i), (a1i, a2i), (a2i, n2i), (n1i, b1i), (b1i, b2i), (b2i, n2i), (n2i, m3i), (m3i, ksi)] {
            self.link(x, y);
        }
    }
}

fn build() -> Vec<Node> {
    let v = |s: &[(&'static str, f64)]| s.to_vec();
    let mut b = Builder { nodes: Vec::new() };
    b.add("start".into(), 0.0, 0.0, "start", "THE SPARK", vec![], 0);
    b.branch("war", 0.0,
        ("HEAVY HANDS", v(&[("melee", 0.04)])),
        ("FOLLOW THROUGH", v(&[("melee", 0.06)])),
        ("IRON GRIP", v(&[("knock", 0.15)])),
        ("STAGGERING BLOWS", v(&[("knock", 0.25), ("melee", 0.03)])),
        ("SHRUG IT OFF", v(&[("iframes", 8.0)])),
        ("BLOODLUST", v(&[("melee", 0.06), ("leech", 0.01)])),
        ("BRUTE FORCE", v(&[("melee", 0.08), ("knock", 0.2)])),
        ("WARLORD", v(&[("melee", 0.12), ("iframes", 4.0)])),
        ("TITANS ARM", v(&[("melee", 0.1), ("knock", 0.15)])),
        ("JUGGERNAUT", v(&[("melee", 0.35), ("knock", 0.5), ("move", -0.08)])));
    b.branch("bld", 45.0,
        ("SCAB", v(&[("regen", 1.0)])),
        ("THICK BLOOD", v(&[("maxhp", 8.0)])),
        ("SLOW PULSE", v(&[("regen", 2.0)])),
        ("CLOTTING", v(&[("regen", 3.0), ("maxhp", 6.0)])),
        ("RED SIP", v(&[("leech", 0.015)])),
        ("HUNGER", v(&[("leech", 0.025)])),
        ("OPEN VEINS", v(&[("leech", 0.02), ("melee", 0.04)])),
        ("RED FEAST", v(&[("leech", 0.03), ("regen", 2.0)])),
        ("HEARTSBLOOD", v(&[("maxhp", 14.0)])),
        ("VAMPIRE", v(&[("leech", 0.08), ("melee", 0.12), ("regen", -4.0)])));
    b.branch("for", 90.0,
        ("LUCKY PENNY", v(&[("coin", 0.08)])),
        ("SHARP EYES", v(&[("luck", 0.06)])),
        ("COIN SCENT", v(&[("coin", 0.12)])),
        ("HAGGLER", v(&[("craft", 0.08)])),
        ("WIDE NET", v(&[("magnet", 10.0)])),
        ("TREASURE SENSE", v(&[("luck", 0.08)])),
        ("KEEN EYE", v(&[("luck", 0.1), ("magnet", 8.0)])),
        ("GOLDEN TOUCH", v(&[("coin", 0.2), ("luck", 0.08)])),
        ("FOUR LEAF", v(&[("luck", 0.12)])),
        ("MIDAS", v(&[("coin", 0.5), ("luck", 0.25), ("maxhp", -12.0)])));
    b.branch("mag", 135.0,
        ("EMBER MIND", v(&[("spell", 0.05)])),
        ("DEEP WELL", v(&[("maxmana", 3.0)])),
        ("FLOW", v(&[("manaregen", 1.0)])),
        ("SPRING TIDE", v(&[("manaregen", 2.0)])),
        ("FOCUS", v(&[("spell", 0.06)])),
        ("OVERCHARGE", v(&[("spell", 0.08)])),
        ("ATTUNEMENT", v(&[("spell", 0.08), ("maxmana", 3.0)])),
        ("ARCHMAGE", v(&[("spell", 0.12), ("manaregen", 2.0)])),
        ("STARLIT WELL", v(&[("maxmana", 6.0)])),
        ("ARCHON", v(&[("spell", 0.45), ("maxmana", 10.0), ("maxhp", -18.0)])));
    b.branch("wnd", 180.0,
        ("LIGHT STEP", v(&[("move", 0.03)])),
        ("STRIDE", v(&[("move", 0.04)])),
        ("SWIFT HANDS", v(&[("haste", 0.06)])),
        ("FLURRY", v(&[("haste", 0.08)])),
        ("TAILWIND", v(&[("move", 0.04)])),
        ("SLIPSTREAM", v(&[("move", 0.05), ("haste", 0.02)])),
        ("QUICKSTEP", v(&[("move", 0.05), ("haste", 0.04)])),
        ("TEMPEST", v(&[("haste", 0.08), ("move", 0.04)])),
        ("ZEPHYR", v(&[("move", 0.06)])),
        ("GALESOUL", v(&[("haste", 0.18), ("move", 0.12), ("maxhp", -20.0)])));
    b.branch("pre", 225.0,
        ("STEADY AIM", v(&[("crit", 0.015)])),
        ("KNIFES EDGE", v(&[("critmult", 0.08)])),
        ("FIND THE GAP", v(&[("crit", 0.02)])),
        ("COLD READ", v(&[("crit", 0.03)])),
        ("CRUEL ANGLE", v(&[("critmult", 0.12)])),
        ("DEEP CUTS", v(&[("critmult", 0.15)])),
        ("PRECISION", v(&[("crit", 0.025), ("critmult", 0.1)])),
        ("EXECUTIONER", v(&[("critmult", 0.25), ("crit", 0.02)])),
        ("DEATHMARK", v(&[("crit", 0.04)])),
        ("ASSASSIN", v(&[("crit", 0.12), ("critmult", 0.4), ("melee", -0.12)])));
    b.branch("gth", 270.0,
        ("GREEN THUMB", v(&[("gather", 0.06)])),
        ("TIMBER SENSE", v(&[("gather", 0.08)])),
        ("ROOT AND STEM", v(&[("gather", 0.1)])),
        ("MOTHER LODE", v(&[("luck", 0.08)])),
        ("LONG ARMS", v(&[("magnet", 12.0)])),
        ("GLEANER", v(&[("coin", 0.1)])),
        ("FORAGER", v(&[("gather", 0.12), ("magnet", 6.0)])),
        ("HARVESTER", v(&[("gather", 0.15), ("luck", 0.08)])),
        ("DEEP ROOTS", v(&[("gather", 0.12)])),
        ("BOUNTIFUL", v(&[("gather", 0.5), ("luck", 0.2), ("move", -0.06)])));
    b.branch("crf", 315.0,
        ("CAREFUL HANDS", v(&[("craft", 0.05)])),
        ("SCRAP SENSE", v(&[("craft", 0.06)])),
        ("REINFORCE", v(&[("defense", 1.0)])),
        ("PLATING", v(&[("defense", 1.0), ("maxhp", 6.0)])),
        ("BARTER", v(&[("coin", 0.1)])),
        ("TRADE ROUTES", v(&[("coin", 0.12)])),
        ("TINKERER", v(&[("craft", 0.1), ("coin", 0.08)])),
        ("MASTERWORK", v(&[("craft", 0.15), ("defense", 1.0)])),
        ("SPARE PARTS", v(&[("craft", 0.12)])),
        ("ARTIFICER", v(&[("craft", 0.6), ("coin", 0.25), ("melee", -0.08)])));
    // The ring road: adjacent branches' first notables connect so paths travel sideways.
    for (x, y) in [("war", "bld"), ("bld", "for"), ("for", "mag"), ("mag", "wnd"), ("wnd", "pre"), ("pre", "gth"), ("gth", "crf"), ("crf", "war")] {
        let (a, bb) = (b.idx(&format!("{x}n1")), b.idx(&format!("{y}n1", y = y)));
        b.link(a, bb);
    }
    b.nodes
}

/// The constellation, built once.
pub fn nodes() -> &'static [Node] {
    static NODES: OnceLock<Vec<Node>> = OnceLock::new();
    NODES.get_or_init(build)
}

pub fn start() -> usize {
    0 // "start" is always the first node added
}

/// Sum an allocated tree's total for one stat (port of `stat`).
pub fn stat(taken: &HashSet<usize>, name: &str) -> f64 {
    let ns = nodes();
    taken
        .iter()
        .flat_map(|&i| ns[i].stats.iter())
        .filter(|(k, _)| *k == name)
        .map(|(_, v)| v)
        .sum()
}

fn allocated(taken: &HashSet<usize>, i: usize) -> bool {
    i == start() || taken.contains(&i)
}

/// May this node be bought? (a point-cost check is the caller's business)
pub fn linked_to_tree(taken: &HashSet<usize>, i: usize) -> bool {
    nodes()[i].links.iter().any(|&l| allocated(taken, l))
}

/// Can this node be removed without stranding part of the tree? (BFS from start without it.)
pub fn leaf_safe(taken: &HashSet<usize>, i: usize) -> bool {
    let ns = nodes();
    let rest: Vec<usize> = taken.iter().copied().filter(|&k| k != i).collect();
    if rest.is_empty() {
        return true;
    }
    let mut ok = HashSet::from([start()]);
    let mut q = vec![start()];
    while let Some(c) = q.pop() {
        for &l in &ns[c].links {
            if !ok.contains(&l) && l != i && allocated(taken, l) {
                ok.insert(l);
                q.push(l);
            }
        }
    }
    rest.iter().all(|k| ok.contains(k))
}

/// Directional cursor move (port of the update() cone search): nearest node inside a ~60°
/// cone the way you pressed — spatial first (the tree is a MAP), link-following fallback.
pub fn nav(cur: usize, dx: f64, dy: f64) -> Option<usize> {
    let ns = nodes();
    let dl = dx.hypot(dy);
    if dl == 0.0 {
        return None;
    }
    let (ux, uy) = (dx / dl, dy / dl);
    let c = &ns[cur];
    let mut best = None;
    let mut bs = 0.0;
    for (i, n) in ns.iter().enumerate() {
        if i == cur {
            continue;
        }
        let (vx, vy) = ((n.x - c.x) as f64, (n.y - c.y) as f64);
        let len = vx.hypot(vy).max(1.0);
        let d = (vx * ux + vy * uy) / len;
        if d > 0.5 {
            let s = d / len; // alignment over distance: near + on-axis wins
            if s > bs {
                bs = s;
                best = Some(i);
            }
        }
    }
    if best.is_none() {
        let mut lb = 0.25;
        for &l in &c.links {
            let n = &ns[l];
            let (vx, vy) = ((n.x - c.x) as f64, (n.y - c.y) as f64);
            let len = vx.hypot(vy).max(1.0);
            let d = (vx * ux + vy * uy) / len;
            if d > lb {
                lb = d;
                best = Some(l);
            }
        }
    }
    best
}

/// Tooltip stat lines: (text, is-downside) — the keystone's price printed honestly.
pub fn stat_lines(n: &Node) -> Vec<(String, bool)> {
    let pct = |v: f64| format!("{}{}%", if v > 0.0 { "+" } else { "" }, js_round(v * 100.0));
    let flat = |v: f64| format!("{}{}", if v > 0.0 { "+" } else { "" }, js_round(v));
    n.stats
        .iter()
        .map(|&(k, v)| {
            let txt = match k {
                "melee" => format!("{} MELEE DMG", pct(v)),
                "crit" => format!("{} CRIT CHANCE", pct(v)),
                "critmult" => format!("{} CRIT DMG", pct(v)),
                "leech" => format!("{} LIFESTEAL", pct(v)),
                "haste" => format!("{} ATK SPEED", pct(v)),
                "move" => format!("{} SPEED", flat(v * 100.0)),
                "maxhp" => format!("{} MAX HP", flat(v)),
                "defense" => format!("{} ARMOR", flat(v)),
                "regen" => format!("{} REGEN", flat(v)),
                "luck" => format!("{} DROP RATE", pct(v)),
                "magnet" => format!("{} PICKUP RANGE", flat(v)),
                "knock" => format!("{} KNOCKBACK", pct(v)),
                "coin" => format!("{} COIN", pct(v)),
                "spell" => format!("{} SPELL DMG", pct(v)),
                "maxmana" => format!("{} MAX MP", flat(v)),
                "manaregen" => format!("{} MANA REGEN", flat(v)),
                "gather" => format!("{} HARVEST YIELD", pct(v)),
                "craft" => format!("{} MATERIAL SAVE", pct(v)),
                _ => format!("{} {}", flat(v), k.to_uppercase()),
            };
            (txt, v < 0.0)
        })
        .collect()
}

/// The branch identity colour (port of the RGB table; grey for start/unknown).
pub fn branch_color(id: &str) -> (u8, u8, u8) {
    match &id[..3.min(id.len())] {
        "war" => (224, 90, 72),
        "bld" => (216, 74, 120),
        "for" => (232, 192, 74),
        "blw" => (90, 168, 90),
        "wnd" => (90, 200, 216),
        "pre" => (160, 106, 224),
        "mag" => (110, 130, 245),
        "gth" => (176, 128, 70),
        "crf" => (235, 142, 55),
        _ => (138, 148, 168),
    }
}

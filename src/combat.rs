//! combat.rs — the hit-resolution core (port of the battle half of entities.js resolveCombat).
//!
//! Everything that can deal or take damage carries [`Combatant`] + [`Hitbox`]; the
//! [`resolve_combat`] pass walks attackers x targets exactly like the JS: team/hurt-team
//! gates, i-frames, one-shot attackers hit a target once, persistent (body-contact) attackers
//! get the +-3px reach expansion, defense floors damage at 1, and a landed hit knocks the
//! target back from the attacker's centre using the target's own hurt profile.
//!
//! Not ported yet (they arrive with their systems): shields/deflection, fire/flammables,
//! gather-tool gating, crits/lifesteal, statuses. Each is a marked branch in the JS original.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
    /// Gatherable resource nodes (trees, bushes, rocks, grass) — hit by player tools only.
    Object,
    /// Neutral blasts (js team 'hazard'): a bomb hurts foes AND the player caught in
    /// it, and shatters nodes past the tool gate (the js 'boom' rule).
    Hazard,
}

/// Which resource a swing gathers (port of the JS weapon `tool` field).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Sword,
    Axe,
    Pick,
}

/// On an attack: the tool it swings as + its head rank (js toolTier — base 1, metal 2..6).
#[derive(Component, Clone, Copy)]
pub struct AttackTool(pub Tool, pub i32);

/// A resource node only yields to its matching tool — and, for ore/wood, a head that
/// MEETS its req_tier (js reqTier). Anything weaker just tinks off. (0 = no tier gate.)
#[derive(Component, Clone, Copy)]
pub struct GatherTool(pub Tool, pub i32);

/// Combat identity: which side, who it may hurt, and what its touch deals.
/// `persistent` = body-contact damage every overlap (mobs); one-shots (swings, projectiles)
/// hit each target once via [`HitOnce`].
#[derive(Component)]
pub struct Combatant {
    pub team: Team,
    pub hurt_team: Option<Team>,
    pub damage: Option<i32>,
    pub persistent: bool,
    pub knock: f32,
}

/// Absolute room-pixel hitbox, updated by its owner's system every tick.
#[derive(Component, Clone, Copy)]
pub struct Hitbox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Hitbox {
    pub fn overlaps(&self, o: &Hitbox) -> bool {
        // Port of Entities.overlap.
        !(self.x + self.w <= o.x || o.x + o.w <= self.x || self.y + self.h <= o.y || o.y + o.h <= self.y)
    }
    fn expanded(&self, by: f32) -> Hitbox {
        Hitbox { x: self.x - by, y: self.y - by, w: self.w + by * 2.0, h: self.h + by * 2.0 }
    }
}

#[derive(Component)]
pub struct Health {
    pub hp: i32,
    pub max: i32,
    pub defense: i32,
    pub invuln: u32,
    pub flash: u32,
}

/// How this target reacts to a landed hit — the per-kind constants from the JS onHurt
/// handlers (player: 72 i-frames, kb 2.6/8; goblin: 10 i-frames, kb 2.2/11).
#[derive(Component)]
pub struct HurtProfile {
    pub invuln: u32,
    pub flash: u32,
    pub kb_base: f32,
    pub kb_frames: u32,
}

/// Knockback in flight; movement systems yield to it while `timer > 0`.
#[derive(Component, Default)]
pub struct Knockback {
    pub kx: f32,
    pub ky: f32,
    pub timer: u32,
}

/// One-shot attackers remember who they already hit (port of `atk.hits`).
#[derive(Component, Default)]
pub struct HitOnce(pub HashSet<Entity>);

/// Blood colour for hit sprays (port of `e.blood`).
#[derive(Component)]
pub struct Blood(pub u32);

/// An attack that can land critically (js atk.crit/critMult — swings, arrows,
/// spellbolts as their systems port). Rolled once per landed hit.
#[derive(Component)]
pub struct CritChance {
    pub chance: f64,
    pub mult: f64,
}

/// Fired for every landed hit so FX/audio systems can react without living inside the pass.
#[derive(Message)]
pub struct HitLanded {
    pub target: Entity,
    /// Who landed it (thorns bite back; proc swings mark their foe).
    pub attacker: Entity,
    pub at: Vec2,
    pub blood: Option<u32>,
    pub dealt: i32, // damage after defense — the stats ledger counts the player's losses
    /// The attacker's on-hit status (webs mire, venom clings) — players only.
    pub afflicts: Option<(&'static str, i32)>,
    /// The hit landed critically (gold sparkle in fx.rs).
    pub crit: bool,
}

/// On an attacker: the status its landed hits inflict on the PLAYER (js atk.slow /
/// atk.poison / atk.burn / atk.shock). An empty id is a no-op placeholder.
#[derive(Component)]
pub struct Afflicts(pub &'static str, pub i32);

/// A wrong-tool strike glancing off a resource node (spark + sound, no damage). `note`
/// carries a toast for the too-weak-tool case (js resistTool — "NEEDS A STRONGER PICK").
#[derive(Message)]
pub struct Tinked {
    pub at: Vec2,
    pub note: Option<&'static str>,
}

/// The resolve pass. Mirrors the JS loop shape: for each live attacker, test every target.
#[allow(clippy::type_complexity)] // ECS system queries are wide by nature
pub fn resolve_combat(
    mut attackers: Query<(Entity, &Combatant, &Hitbox, Option<&mut HitOnce>, Option<&AttackTool>, Option<&Afflicts>, Option<&CritChance>)>,
    mut targets: Query<(Entity, &Combatant, &Hitbox, &mut Health, &HurtProfile, Option<&mut Knockback>, Option<&Blood>, Option<&GatherTool>)>,
    mut rng: ResMut<crate::app::battle::GameRng>,
    mut hits: MessageWriter<HitLanded>,
    mut tinks: MessageWriter<Tinked>,
) {
    for (a_ent, atk, abox, mut once, atk_tool, afflicts, crit_chance) in &mut attackers {
        let Some(damage) = atk.damage else { continue };
        let ab = if atk.persistent { abox.expanded(3.0) } else { *abox };
        for (t_ent, tgt, tbox, mut health, profile, kb, blood, gather) in &mut targets {
            if t_ent == a_ent || tgt.team == atk.team {
                continue;
            }
            if let Some(hurt) = atk.hurt_team
                && tgt.team != hurt
            {
                continue;
            }
            if tgt.team == Team::Object && atk.team == Team::Enemy {
                continue; // enemies don't chip/gather resource nodes (no tink spam by a tree)
            }
            if health.invuln > 0 || health.hp <= 0 {
                continue; // target has i-frames (or is already down this tick)
            }
            if let Some(once) = &mut once
                && once.0.contains(&t_ent)
            {
                continue;
            }
            if !ab.overlaps(tbox) {
                continue;
            }
            if let Some(once) = &mut once {
                once.0.insert(t_ent);
            }
            // Gathering: a node only yields to its matching tool; the wrong tool tinks off
            // (still consumed the hits-set slot above, exactly like the JS order). A
            // HAZARD blast ignores the gate — js: nodes shatter to type 'boom'.
            if let Some(node_tool) = gather
                && atk.team != Team::Hazard
            {
                let wrong_tool = atk_tool.map(|t| t.0) != Some(node_tool.0);
                let too_weak = !wrong_tool && node_tool.1 > atk_tool.map_or(1, |t| t.1);
                if wrong_tool || too_weak {
                    // Wrong tool, OR the right tool but too weak a head for this ore/wood tier
                    // (js reqTier gate) — the tier miss surfaces a "needs a stronger tool" toast.
                    let note = too_weak.then_some(match node_tool.0 {
                        Tool::Axe => "NEEDS A STRONGER AXE",
                        _ => "NEEDS A STRONGER PICK",
                    });
                    tinks.write(Tinked {
                        at: Vec2::new(tbox.x + tbox.w / 2.0, tbox.y + tbox.h / 2.0),
                        note,
                    });
                    continue;
                }
            }
            // Defense reduces damage (min 1), then a crit multiplies what got through
            // (the js order: max(1, base - defense) -> round(dealt x critMult)).
            let mut dealt = (damage - health.defense).max(1);
            let crit = crit_chance.is_some_and(|c| c.chance > 0.0 && rng.0.next_f64() < c.chance);
            if crit {
                let mult = crit_chance.map_or(2.0, |c| c.mult);
                dealt = ((dealt as f64) * mult).round() as i32;
            }
            health.hp -= dealt;
            health.invuln = profile.invuln;
            health.flash = profile.flash;
            // Knockback away from the attacker's centre (the shared JS onHurt shape).
            if let Some(mut kb) = kb {
                let acx = abox.x + abox.w / 2.0;
                let acy = abox.y + abox.h / 2.0;
                let dx = (tbox.x + tbox.w / 2.0) - acx;
                let dy = (tbox.y + tbox.h / 2.0) - acy;
                let m = dx.hypot(dy).max(1e-6);
                let k = profile.kb_base + atk.knock;
                kb.kx = dx / m * k;
                kb.ky = dy / m * k;
                kb.timer = profile.kb_frames;
            }
            hits.write(HitLanded {
                target: t_ent,
                attacker: a_ent,
                at: Vec2::new(tbox.x + tbox.w / 2.0, tbox.y + tbox.h / 2.0),
                blood: blood.map(|b| b.0),
                dealt,
                afflicts: afflicts
                    .filter(|a| tgt.team == Team::Player && !a.0.is_empty())
                    .map(|a| (a.0, a.1)),
                crit,
            });
        }
    }
}

/// Tick i-frames + hurt flashes (every combatant shares this).
pub fn tick_health(mut q: Query<&mut Health>) {
    for mut h in &mut q {
        if h.invuln > 0 {
            h.invuln -= 1;
        }
        if h.flash > 0 {
            h.flash -= 1;
        }
    }
}

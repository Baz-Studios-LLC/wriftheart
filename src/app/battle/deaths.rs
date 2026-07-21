//! The fall: goblin + biome-mob deaths — burst, the js deathEffect drop recipes,
//! XP, the ledger + bestiary, slime splits and the zombie collapse (split from battle.rs).

use super::{fx::spawn_burst, GameRng, RoomActor};
use crate::actors::goblin::Goblin;
use crate::actors::mobs::{self, Mob};
use crate::combat::{Combatant, Health};
use crate::gfx::PIXEL_LAYER;
use bevy::prelude::*;

/// Fallen goblins: burst, the js goblin deathEffect drop table, XP for the kill, despawn.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
#[allow(clippy::type_complexity)] // the goblin corpse row: chassis + skin + bounty
pub(super) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    tstats: Res<crate::app::slideout::TreeStats>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut bestiary: ResMut<crate::app::codex::mobs_tab::Bestiary>,
    mut credits: MessageWriter<crate::app::quests::KillCredit>,
    q: Query<(Entity, &Goblin, &Health, Option<&crate::actors::goblin::HumanSkin>, Option<&crate::app::quests::BountyTag>)>,
) {
    for (e, g, h, skin, bounty) in &q {
        if h.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(g.x + 8.0, g.y + 8.0), 0xd82800, 10);
        // The ledger + the bestiary page (js bump('kills') / bump('kill_'+type) / bestiary.add).
        // A HumanSkin rider (bandit) credits as ITS kind — and a goblin-chassis bounty
        // elite reports its tag, or the bounty could never complete.
        let key = match skin {
            Some(s) => s.kind,
            None if g.kind == crate::actors::goblin::GoblinKind::Spear => "slinger",
            None => "goblin",
        };
        credits.write(crate::app::quests::KillCredit { kind: key, bounty: bounty.map(|b| b.0) }); // js onEnemyKilled
        stats.bump("kills", 1.0);
        stats.bump(
            match key {
                "slinger" => "kill_slinger",
                "bandit" => "kill_bandit",
                "cultist" => "kill_cultist",
                _ => "kill_goblin",
            },
            1.0,
        );
        bestiary.0.insert(key);
        // --- js goblin deathEffect (red/champion multipliers join with those variants) ---
        let luck = 1.0 + tstats.luck; // js Entities.luckMult
        let ranged = g.kind == crate::actors::goblin::GoblinKind::Spear;
        let (gx, gy) = (g.x + 4.0, g.y + 4.0);
        let cv = 1 + (rng.0.next_f64() * 5.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, cv, gx, gy);
        if !ranged && rng.0.next_f64() < 0.15 * luck {
            // The woodcutting gate: a common enough melee-goblin drop to bootstrap
            // harvesting (js: axe, NO magnet).
            crate::app::gather::spawn_pickup(&mut commands, &mut images, "axe", 1, gx, gy, false, None);
        }
        if ranged && rng.0.next_f64() < 0.15 * luck {
            // Slingshot goblins scrounge stones — a small chance to drop 1-2.
            let n = 1 + (rng.0.next_f64() * 2.0) as i32;
            crate::app::gather::spawn_pickup(&mut commands, &mut images, "stone", n, gx, gy, true, None);
        }
        // Any foe may scatter bow ammo (js: ~5% x luck — the shared cull-loop roll).
        if rng.0.next_f64() < 0.05 * luck {
            crate::app::gather::spawn_pickup(&mut commands, &mut images, "arrow", 1, gx + 4.0, gy, true, None);
        }
        // Loot scarcity: trash goblins cough up gear barely ever (js 0.8%; red 3% and
        // champions 20% when those variants port).
        if rng.0.next_f64() < 0.008 * luck {
            let (id, qty) = crate::items::roll_loot(0.0, luck - 1.0, || rng.0.next_f64());
            crate::app::gather::spawn_pickup(&mut commands, &mut images, id, qty, gx, gy, true, None);
        }
        // XP for the kill (js e.xp: spear 4, melee 3; red 8 when it ports).
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, if ranged { 4 } else { 3 });
        commands.entity(e).despawn();
    }
}

/// Fallen biome mobs: burst, the js mob deathEffect drop recipe, XP, the ledger + bestiary.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
/// One dying mob's row (the bounty tag rides along for quest credit).
type MobDeathRow = (Entity, &'static mut Mob, &'static mut Combatant, &'static mut Health, Option<&'static crate::app::quests::BountyTag>, Option<&'static crate::app::champions::Promoted>, Option<&'static crate::app::champions::AffixVolatile>);

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(super) fn mob_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    tstats: Res<crate::app::slideout::TreeStats>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut bestiary: ResMut<crate::app::codex::mobs_tab::Bestiary>,
    mut credits: MessageWriter<crate::app::quests::KillCredit>,
    inv: Res<crate::inventory::PlayerInv>,
    mut players: Query<&mut Health, (With<crate::app::play::Player>, Without<Mob>)>,
    mut q: Query<MobDeathRow, With<Mob>>,
) {
    for (e, mut m, mut cb, mut h, bounty, promoted, affix_volatile) in &mut q {
        if h.hp > 0 || m.downed {
            continue;
        }
        let d = &mobs::MOB_DEFS[m.def];
        credits.write(crate::app::quests::KillCredit { kind: d.kind, bounty: bounty.map(|b| b.0) }); // js onEnemyKilled
        // js surviveLethal: a downRevive kind COLLAPSES instead of dying (fire ends it
        // for good once fire exists).
        if d.down_revive > 0 {
            m.downed = true;
            m.down_t = d.down_revive;
            cb.damage = None;
            h.hp = 0; // stays down; the rise restores it
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(m.x + 8.0, m.y + 8.0), d.blood, 10);
        if d.volatile || affix_volatile.is_some() {
            // js blast(): a brief AoE that catches the player if they're hugging it.
            crate::app::caves::spawn_death_blast(&mut commands, &mut images, m.x + 8.0, m.y + 9.0, d.damage.max(2));
        }
        if let Some(promo) = promoted {
            // A fallen leader: the ledger line + it coughs up gear (js champion/elite
            // loot — elites drop double).
            stats.bump(if promo.elite { "elites" } else { "champions" }, 1.0);
            let rolls = if promo.elite { 2 } else { 1 };
            let boost = if promo.elite { 0.8 } else { 0.5 };
            for i in 0..rolls {
                let (id, qty) = crate::items::roll_loot(boost, tstats.luck, || rng.0.next_f64());
                crate::app::gather::spawn_pickup(&mut commands, &mut images, id, qty, m.x + 2.0 + i as f32 * 8.0, m.y + 6.0, true, None);
            }
        }
        // Kill procs (js cull-loop reward block): Midas coin bursts, Soul Locket mends —
        // chances scale with luck.
        let midas = crate::items::gear_stat(&inv, "midas");
        if midas > 0.0 && rng.0.next_f64() < midas * (1.0 + tstats.luck) {
            crate::app::gather::spawn_coin(&mut commands, &mut images, 3 + (rng.0.next_f64() * 6.0) as i32, m.x + 4.0, m.y + 4.0);
        }
        let soul = crate::items::gear_stat(&inv, "soul");
        if soul > 0.0
            && rng.0.next_f64() < soul * (1.0 + tstats.luck)
            && let Ok(mut ph) = players.single_mut()
            && ph.hp > 0
        {
            ph.hp = (ph.hp + 1).min(ph.max);
        }
        let luck = 1.0 + tstats.luck;
        let (gx, gy) = (m.x + 4.0, m.y + 4.0);
        // A big slime SPLITS into two small ones when slain (js slime drops).
        if d.splits && !m.small {
            commands.spawn((mobs::mob_bundle_small(m.def, m.x - 7.0, m.y - 2.0), RoomActor, PIXEL_LAYER));
            commands.spawn((mobs::mob_bundle_small(m.def, m.x + 7.0, m.y + 2.0), RoomActor, PIXEL_LAYER));
        }
        // Coins (js: (o.coin ? o.coin() : 1 + rand*4); smalls carry pocket change).
        let cv = if m.small {
            (rng.0.next_f64() * 2.0) as i32
        } else {
            match d.coin {
                mobs::Coin::Default => 1 + (rng.0.next_f64() * 4.0) as i32,
                mobs::Coin::None => 0,
                mobs::Coin::Range(base, spread) => base + (rng.0.next_f64() * spread as f64) as i32,
            }
        };
        if cv > 0 {
            crate::app::gather::spawn_coin(&mut commands, &mut images, cv, gx, gy);
        }
        // Per-kind materials (js matDrop: gate by chance*luck, then min + rand(spread));
        // split-children carry nothing but their coins.
        if let Some((mat, chance, min, spread)) = d.drops.filter(|_| !m.small)
            && rng.0.next_f64() < chance * luck
        {
            let qty = min + if spread > 0 { (rng.0.next_f64() * spread as f64) as i32 } else { 0 };
            crate::app::gather::spawn_pickup(&mut commands, &mut images, mat, qty, gx, gy, true, None);
        }
        // Any foe may scatter bow ammo (js: ~5% x luck, on top of its own drops).
        if !m.small && rng.0.next_f64() < 0.05 * luck {
            crate::app::gather::spawn_pickup(&mut commands, &mut images, "arrow", 1, gx + 4.0, gy, true, None);
        }
        // A healing drop for the bruisers (js o.potion).
        if !m.small && d.potion > 0.0 && rng.0.next_f64() < d.potion {
            crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, gx, gy + 2.0, true, None);
        }
        // Loot scarcity: trash mobs cough up gear barely ever (js 0.8% * luck).
        if !m.small && rng.0.next_f64() < 0.008 * luck {
            let (id, qty) = crate::items::roll_loot(0.0, luck - 1.0, || rng.0.next_f64());
            crate::app::gather::spawn_pickup(&mut commands, &mut images, id, qty, gx, gy, true, None);
        }
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, if m.small { 1 } else { d.xp });
        stats.bump("kills", 1.0);
        stats.bump_kill(d.kind);
        bestiary.0.insert(d.kind);
        commands.entity(e).despawn();
    }
}

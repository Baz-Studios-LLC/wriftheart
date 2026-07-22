//! Skill-tree INVARIANTS. The constellation DELIBERATELY deviates from the js
//! (Baz, 2026-07-24: every node bespoke + chunkier) — the old golden pin is
//! retired; what must still hold is the structure and the design rules:
//! the js layout skeleton, full reachability, keystone tradeoffs, and the
//! no-duplicate-nodes rule the redesign exists for.

use std::collections::HashSet;
use wriftheart::skilltree::{nodes, start};

#[test]
fn constellation_structure_holds() {
    let ns = nodes();
    assert_eq!(ns.len(), 1 + 8 * 10, "8 branches x 10 slots + the start");
    // Every node reachable from the start.
    let mut seen = HashSet::from([start()]);
    let mut queue = vec![start()];
    while let Some(i) = queue.pop() {
        for &l in &ns[i].links {
            if seen.insert(l) {
                queue.push(l);
            }
        }
    }
    assert_eq!(seen.len(), ns.len(), "unreachable nodes");
    for n in ns {
        // Costs stay in the js band; the start is free.
        assert!(n.cost <= 5, "{} cost", n.id);
        if n.kind == "keystone" {
            // A keystone always pays for its power (the tradeoff rule).
            assert!(n.stats.iter().any(|(_, v)| *v < 0.0), "{} lacks a downside", n.id);
        }
    }
}

#[test]
fn no_branch_repeats_a_node() {
    // The redesign's law (Baz): within a branch, every node is its own reward —
    // no two nodes share a name or an identical stat map.
    let ns = nodes();
    let mut by_branch: std::collections::HashMap<&str, Vec<&wriftheart::skilltree::Node>> = Default::default();
    for n in ns.iter().skip(1) {
        by_branch.entry(&n.id[..3]).or_default().push(n);
    }
    for (branch, list) in by_branch {
        let names: HashSet<_> = list.iter().map(|n| n.name).collect();
        assert_eq!(names.len(), list.len(), "duplicate node name in branch {branch}");
        let stats: HashSet<String> = list.iter().map(|n| format!("{:?}", n.stats)).collect();
        assert_eq!(stats.len(), list.len(), "duplicate stat map in branch {branch}");
    }
}

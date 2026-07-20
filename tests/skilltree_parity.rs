//! Skill-tree parity: the ported constellation must match the JS NODES table exactly —
//! ids, rounded positions, kinds, names, costs, stat maps, and the link graph.

use wriftheart::skilltree::nodes;

mod golden {
    include!("data/skilltree_golden.rs");
}

#[test]
fn constellation_matches_js() {
    let ours = nodes();
    assert_eq!(ours.len(), golden::NODES.len(), "node count");
    for (n, (id, x, y, kind, name, cost, stats, links)) in ours.iter().zip(golden::NODES) {
        assert_eq!(&n.id, id, "node order/id");
        assert_eq!((n.x, n.y), (*x, *y), "{id} position");
        assert_eq!(n.kind, *kind, "{id} kind");
        assert_eq!(n.name, *name, "{id} name");
        assert_eq!(n.cost, *cost, "{id} cost");
        let mut got: Vec<(&str, f64)> = n.stats.clone();
        let mut want: Vec<(&str, f64)> = stats.to_vec();
        got.sort_by(|a, b| a.0.cmp(b.0));
        want.sort_by(|a, b| a.0.cmp(b.0));
        assert_eq!(got, want, "{id} stats");
        let mut got_links: Vec<&str> = n.links.iter().map(|&l| ours[l].id.as_str()).collect();
        let mut want_links: Vec<&str> = links.to_vec();
        got_links.sort_unstable();
        want_links.sort_unstable();
        assert_eq!(got_links, want_links, "{id} links");
    }
}

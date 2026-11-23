//! The fast evaluator must agree with the simple O(n·m) reference on random games.
use poker_core::{Card, CardSet};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::Combo;
use solver_validation::{GameNode, PostflopGame, Rake, evaluate, evaluate_reference};

struct Gen {
    rng: ChaCha8Rng,
    nodes: Vec<GameNode>,
    sizes: [usize; 2],
}

impl Gen {
    /// Builds a random subtree; `board` is the number of board cards so far.
    fn node(&mut self, board: u64, put: [f64; 2], depth: usize, to_act: usize) -> usize {
        let me = self.nodes.len();
        self.nodes.push(GameNode::Fold { folder: 0, put });
        let cards = board.count_ones();
        let roll = self.rng.random_range(0..10);
        let node = if depth == 0 || roll < 2 {
            if cards == 5 { GameNode::Showdown { put } } else { self.chance(board, put, depth) }
        } else if roll < 3 {
            GameNode::Fold { folder: to_act, put: [put[0] + 1.0 * (to_act as f64), put[1] + 1.0 * (1 - to_act) as f64] }
        } else if roll < 4 && cards < 5 {
            self.chance(board, put, depth)
        } else {
            let k = self.rng.random_range(2..4);
            let n = self.sizes[to_act];
            let mut strategy = vec![0.0; k * n];
            for i in 0..n {
                let raw: Vec<f64> = (0..k).map(|_| self.rng.random::<f64>() + 1e-3).collect();
                let s: f64 = raw.iter().sum();
                for a in 0..k {
                    strategy[a * n + i] = raw[a] / s;
                }
            }
            let mut children = Vec::new();
            for a in 0..k {
                let bump = a as f64 * 2.0;
                let mut np = put;
                np[to_act] += bump;
                np[1 - to_act] += bump * self.rng.random_range(0..2) as f64;
                children.push(self.node(board, np, depth - 1, 1 - to_act));
            }
            GameNode::Decision { player: to_act, children, strategy }
        };
        self.nodes[me] = node;
        me
    }

    fn chance(&mut self, board: u64, put: [f64; 2], depth: usize) -> GameNode {
        let mut children = Vec::new();
        for c in Card::all() {
            let bit = 1u64 << c.index();
            if board & bit == 0 {
                let child = self.node(board | bit, put, depth.saturating_sub(1).min(1), 0);
                children.push((c, child));
            }
        }
        GameNode::Chance { children }
    }
}

fn random_game(seed: u64) -> PostflopGame {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let len = if rng.random_bool(0.5) { 4 } else { 5 };
    let mut deck: Vec<Card> = Card::all().collect();
    for i in 0..len {
        let j = rng.random_range(i..52);
        deck.swap(i, j);
    }
    let board: Vec<Card> = deck[..len].to_vec();
    let dead = CardSet::from_cards(board.iter().copied());
    let live: Vec<Combo> = Combo::all().filter(|c| !c.card_set().intersects(dead)).collect();
    // Overlapping ranges, so identical combos appear for both players.
    let mut pick = |p: f64| -> Vec<(Combo, f64)> {
        let mut out = Vec::new();
        for &c in &live {
            if rng.random_bool(p) {
                out.push((c, rng.random_range(0.05..1.0)));
            }
        }
        out
    };
    let r0 = pick(0.15);
    let r1 = pick(0.15);
    let sizes = [r0.len(), r1.len()];
    let mut g = Gen { rng: ChaCha8Rng::seed_from_u64(seed ^ 0xABCD), nodes: Vec::new(), sizes };
    let board_mask = dead.bits();
    g.node(board_mask, [5.0, 5.0], 3, 0);
    let rake = if seed % 2 == 0 { Rake::NONE } else { Rake { fraction: 0.05, cap: Some(3.0) } };
    PostflopGame { board, ranges: [r0, r1], rake, nodes: g.nodes }
}

#[test]
fn fast_and_reference_agree_on_random_games() {
    let mut checked = 0;
    for seed in 0..60 {
        let g = random_game(seed);
        if g.ranges[0].is_empty() || g.ranges[1].is_empty() {
            continue;
        }
        let (Ok(f), Ok(r)) = (evaluate(&g), evaluate_reference(&g)) else {
            panic!("seed {seed}: evaluation failed");
        };
        for p in 0..2 {
            for (a, b, what) in [(f.ev[p], r.ev[p], "ev"), (f.br[p], r.br[p], "br")] {
                assert!((a - b).abs() <= 1e-9 * (1.0 + b.abs()), "seed {seed} p{p} {what}: fast {a} vs reference {b}");
            }
        }
        checked += 1;
    }
    assert!(checked > 50);
}

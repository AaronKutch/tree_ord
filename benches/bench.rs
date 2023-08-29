#![feature(test)]
#![allow(clippy::needless_range_loop)]

extern crate test;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use test::Bencher;
use tree_ord::{Tracker, TreeOrd};

const M: u64 = 1 << 16;
const M1: u64 = 1 << 7;
type T = Vec<u64>;

fn gen_t() -> Vec<T> {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut res = vec![];
    for _ in 0..M {
        let len = (rng.next_u64() % M1) as usize;
        let mut t0: T = vec![0; len];
        if len != 0 {
            for i in 0..((rng.next_u64() as usize) % len) {
                t0[i] = u64::MAX / 2;
            }
            t0.rotate_left((rng.next_u64() as usize) % len);
            for i in 0..((rng.next_u64() as usize) % len) {
                t0[i] = u64::MAX;
            }
            t0.rotate_left((rng.next_u64() as usize) % len);
        }
        res.push(t0);
    }
    res.sort();
    res
}

#[bench]
fn t_tree(bencher: &mut Bencher) {
    type T = Vec<u64>;
    let space = gen_t();
    let inxs = space.clone();

    bencher.iter(|| {
        for rhs in &inxs {
            let mut tracker = <T as TreeOrd>::Tracker::new();
            space
                .binary_search_by(|lhs| lhs.tree_cmp(rhs, &mut tracker))
                .unwrap();
        }
    })
}

#[bench]
fn t_ord(bencher: &mut Bencher) {
    let space = gen_t();
    let inxs = space.clone();

    bencher.iter(|| {
        for rhs in &inxs {
            space.binary_search_by(|lhs| lhs.cmp(rhs)).unwrap();
        }
    })
}

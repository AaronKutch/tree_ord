#![allow(clippy::needless_range_loop)]

use std::{cell::RefCell, cmp::Ordering};

use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use tree_ord::{Tracker, TreeOrd};
use Ordering::*;

const N: u64 = 1 << 15; //1 << 16;
const N0: u64 = 4;
const N1: u64 = 1 << 5;
const M: u64 = 1 << 16;
const M1: u64 = 1 << 7;

thread_local! {
    pub static CMP_COUNT: RefCell<u64> = RefCell::new(0);
}

pub fn get_cmp_count() -> u64 {
    CMP_COUNT.with(|f| *f.borrow())
}

pub fn inc_cmp_count() {
    CMP_COUNT.with(|f| {
        let x = f.borrow().checked_add(1).unwrap();
        *f.borrow_mut() = x;
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct COrd(u64);

#[allow(clippy::incorrect_partial_ord_impl_on_ord_type)]
impl PartialOrd for COrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        inc_cmp_count();
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for COrd {
    fn cmp(&self, other: &Self) -> Ordering {
        inc_cmp_count();
        self.0.cmp(&other.0)
    }
}

impl TreeOrd for COrd {
    type Tracker = ();

    fn tree_cmp(&self, rhs: &Self, _: &mut Self::Tracker) -> Ordering {
        self.cmp(rhs)
    }
}

#[test]
fn tuples() {
    let init = get_cmp_count();
    type T2 = (COrd, COrd);
    let t2: T2 = (COrd(32), COrd(48));
    let mut tracker = <T2 as TreeOrd>::Tracker::new();
    assert_eq!(t2.tree_cmp(&(COrd(8), COrd(64)), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 1);
    assert_eq!(t2.tree_cmp(&(COrd(48), COrd(64)), &mut tracker), Less);
    assert_eq!(get_cmp_count(), init + 2);
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(64)), &mut tracker), Less);
    assert_eq!(get_cmp_count(), init + 4);
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(16)), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 6);
    // after being bounded on both sides with a prefix of 32, only 1 `Ord`
    // call should be incurred
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(24)), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 7);
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(50)), &mut tracker), Less);
    assert_eq!(get_cmp_count(), init + 8);
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(48)), &mut tracker), Equal);
    assert_eq!(get_cmp_count(), init + 9);
    // in nonhereditary tree settings we can still be going down a tree, and `Equal`
    // doesn't constrain bounds so we can't increase the known prefix length
    assert_eq!(t2.tree_cmp(&(COrd(32), COrd(47)), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 10);

    let init = get_cmp_count();
    type T3 = (COrd, COrd, COrd);
    let t3: T3 = (COrd(32), COrd(48), COrd(40));
    let mut tracker = <T3 as TreeOrd>::Tracker::new();
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(40)), &mut tracker),
        Equal
    );
    assert_eq!(get_cmp_count(), init + 3);
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(99)), &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 6);
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(16)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 9);
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(35)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 10);
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(45)), &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 11);
    assert_eq!(
        t3.tree_cmp(&(COrd(32), COrd(48), COrd(40)), &mut tracker),
        Equal
    );
    assert_eq!(get_cmp_count(), init + 12);
}

#[test]
fn result() {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct COrd2(COrd);
    impl TreeOrd for COrd2 {
        type Tracker = <COrd as TreeOrd>::Tracker;

        fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
            self.0.tree_cmp(&rhs.0, tracker)
        }
    }
    let init = get_cmp_count();
    type T = Result<COrd, COrd2>;
    let t: T = Err(COrd2(COrd(32)));
    let mut tracker = <T as TreeOrd>::Tracker::new();
    assert_eq!(t.tree_cmp(&Ok(COrd(32)), &mut tracker), Greater);
    assert_eq!(Err(0u8).cmp(&Ok(0u8)), Greater);
    assert_eq!(get_cmp_count(), init);
    assert_eq!(t.tree_cmp(&Err(COrd2(COrd(32))), &mut tracker), Equal);
    assert_eq!(get_cmp_count(), init + 1);
    assert_eq!(t.tree_cmp(&Ok(COrd(48)), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 1);
    assert_eq!(t.tree_cmp(&Err(COrd2(COrd(16))), &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 2);
}

#[test]
fn slices() {
    type T = Vec<COrd>;
    let t: T = vec![];
    let init = get_cmp_count();
    let mut tracker = <T as TreeOrd>::Tracker::new();
    assert_eq!(t.tree_cmp(&vec![], &mut tracker), Equal);
    assert_eq!(get_cmp_count(), init);
    assert_eq!(t.tree_cmp(&vec![COrd(0)], &mut tracker), Less);
    assert_eq!(t.tree_cmp(&vec![], &mut tracker), Equal);
    assert_eq!(get_cmp_count(), init);

    let t: T = vec![COrd(32), COrd(48), COrd(35)];
    let init = get_cmp_count();
    let mut tracker = <T as TreeOrd>::Tracker::new();
    assert_eq!(t.tree_cmp(&vec![COrd(32), COrd(0)], &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 2);
    assert_eq!(
        t.tree_cmp(&vec![COrd(32), COrd(64), COrd(0), COrd(0)], &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 4);
    assert_eq!(t.tree_cmp(&vec![COrd(32), COrd(16)], &mut tracker), Greater);
    assert_eq!(get_cmp_count(), init + 5);
    assert_eq!(t.tree_cmp(&vec![COrd(32), COrd(64)], &mut tracker), Less);
    assert_eq!(get_cmp_count(), init + 6);
    assert_eq!(t.tree_cmp(&vec![COrd(32), COrd(49)], &mut tracker), Less);
    assert_eq!(get_cmp_count(), init + 7);
    assert_eq!(
        t.tree_cmp(&vec![COrd(32), COrd(47), COrd(35)], &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 8);
    assert_eq!(
        t.tree_cmp(&vec![COrd(32), COrd(48), COrd(35)], &mut tracker),
        Equal
    );
    assert_eq!(get_cmp_count(), init + 10);
    assert_eq!(
        t.tree_cmp(&vec![COrd(32), COrd(48), COrd(40)], &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 12);
    assert_eq!(
        t.tree_cmp(&vec![COrd(32), COrd(48), COrd(30)], &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 14);
}

#[test]
fn nested_tuple() {
    type T = (COrd, Vec<COrd>, COrd);
    let t: T = (COrd(32), vec![COrd(16), COrd(16)], COrd(64));
    let init = get_cmp_count();
    let mut tracker = <T as TreeOrd>::Tracker::new();
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(32)], COrd(0)), &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 2);
    assert_eq!(
        t.tree_cmp(&(COrd(16), vec![COrd(16)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 3);
    assert_eq!(
        t.tree_cmp(&(COrd(24), vec![COrd(16)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 4);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 6);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16), COrd(20)], COrd(0)), &mut tracker),
        Less
    );
    assert_eq!(get_cmp_count(), init + 8);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16), COrd(10)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 10);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16), COrd(11)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 11);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(99)),
            &mut tracker
        ),
        Less
    );
    assert_eq!(get_cmp_count(), init + 13);
    // t.1 is not locked in, need to keep B::Tracker until it is
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(12)], COrd(99)),
            &mut tracker
        ),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 14);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16), COrd(16)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 16);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(60)),
            &mut tracker
        ),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 17);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(64)),
            &mut tracker
        ),
        Equal
    );
    assert_eq!(get_cmp_count(), init + 18);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(63)),
            &mut tracker
        ),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 19);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(64)),
            &mut tracker
        ),
        Equal
    );
    assert_eq!(get_cmp_count(), init + 20);

    // test multiple convergences at same time
    let init = get_cmp_count();
    let mut tracker = <T as TreeOrd>::Tracker::new();
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(99)),
            &mut tracker
        ),
        Less
    );
    assert_eq!(get_cmp_count(), init + 4);
    assert_eq!(
        t.tree_cmp(&(COrd(32), vec![COrd(16), COrd(16)], COrd(0)), &mut tracker),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 8);
    assert_eq!(
        t.tree_cmp(
            &(COrd(32), vec![COrd(16), COrd(16)], COrd(50)),
            &mut tracker
        ),
        Greater
    );
    assert_eq!(get_cmp_count(), init + 9);
}

fn gen_nested_vecs() -> Vec<Vec<Vec<COrd>>> {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    type T = Vec<Vec<COrd>>;
    let mut res: Vec<Vec<Vec<COrd>>> = vec![];
    for _ in 0..N {
        let mut t0: T = vec![];
        for _ in 0..N0 {
            let len = (rng.next_u64() % N1) as usize;
            let mut t1 = vec![];
            for _ in 0..len {
                t1.push(COrd(0));
            }
            if len != 0 {
                for i in 0..((rng.next_u64() as usize) % len) {
                    t1[i] = COrd(4);
                }
                t1.rotate_left((rng.next_u64() as usize) % len);
                for i in 0..((rng.next_u64() as usize) % len) {
                    t1[i] = COrd(8);
                }
                t1.rotate_left((rng.next_u64() as usize) % len);
            }
            t0.push(t1);
        }
        res.push(t0);
    }
    res.sort();
    res
}

#[test]
fn nested_slices() {
    type T = Vec<Vec<COrd>>;
    let mut tree_comparisons = 0;
    let mut comparisons = 0;
    let space = gen_nested_vecs();
    let inxs = space.clone();
    for rhs in &inxs {
        let init = get_cmp_count();
        let mut tracker = <T as TreeOrd>::Tracker::new();
        let found = space
            .binary_search_by(|lhs| lhs.tree_cmp(rhs, &mut tracker))
            .unwrap();
        tree_comparisons += get_cmp_count() - init;

        let init = get_cmp_count();
        let expected = space.binary_search_by(|lhs| lhs.cmp(rhs)).unwrap();
        comparisons += get_cmp_count() - init;
        assert_eq!(found, expected);
    }
    assert_eq!((tree_comparisons, comparisons), (3396610, 5301800));
}

fn gen_bytes() -> Vec<Vec<u8>> {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    type T = Vec<u8>;
    let mut res: Vec<Vec<u8>> = vec![];
    for _ in 0..M {
        let len = (rng.next_u64() % M1) as usize;
        let mut t0: T = vec![0; len];
        if len != 0 {
            for i in 0..((rng.next_u64() as usize) % len) {
                t0[i] = 128;
            }
            t0.rotate_left((rng.next_u64() as usize) % len);
            for i in 0..((rng.next_u64() as usize) % len) {
                t0[i] = 255;
            }
            t0.rotate_left((rng.next_u64() as usize) % len);
        }
        res.push(t0);
    }
    res.sort();
    res
}

#[test]
fn bytes() {
    type T = Vec<u8>;
    let space = gen_bytes();
    let inxs = space.clone();
    for rhs in &inxs {
        let mut tracker = <T as TreeOrd>::Tracker::new();
        space
            .binary_search_by(|lhs| lhs.tree_cmp(rhs, &mut tracker))
            .unwrap();
    }
}

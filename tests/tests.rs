use std::{cell::RefCell, cmp::Ordering};

use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};
use tree_ord::{Tracker, TreeOrd};
use Ordering::*;

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

#[derive(Debug, PartialEq, Eq)]
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

/*
#[test]
fn nested_slices() {
    type T = Vec<Vec<COrd>>;
    let t: T = vec![];
    let init = get_cmp_count();
    let mut tracker = <T as TreeOrd>::Tracker::new();

    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
}
*/

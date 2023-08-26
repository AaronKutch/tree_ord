use core::cmp::{min, Ordering};

use Ordering::*;

use crate::{Tracker, TreeOrd};

/// Minimize debug info for enum matching branches that should be impossible no
/// matter what
#[cold]
pub fn tree_cmp_unreachable() -> ! {
    unreachable!()
}

/// Used by the trackers of things such as slices
pub struct LexicographicTracker<T: TreeOrd> {
    /// Stores a `Tracker` for individual elements
    pub subtracker: T::Tracker,
    /// Element to which `subtracker` corresponds
    pub subtracker_i: usize,
    /// Length of lower bounding prefix
    pub min_eq_len: usize,
    /// Length of upper bounding prefix
    pub max_eq_len: usize,
}

impl<T: TreeOrd> Tracker for LexicographicTracker<T> {
    const IS_NOOP: bool = <T as TreeOrd>::Tracker::IS_NOOP;

    fn new() -> Self {
        LexicographicTracker {
            subtracker: <T as TreeOrd>::Tracker::new(),
            subtracker_i: 0,
            min_eq_len: 0,
            max_eq_len: 0,
        }
    }
}

pub enum ResultTracker<T: TreeOrd, E: TreeOrd> {
    T(<T as TreeOrd>::Tracker),
    E(<E as TreeOrd>::Tracker),
}

impl<T: TreeOrd, E: TreeOrd> Tracker for ResultTracker<T, E> {
    const IS_NOOP: bool = <T as TreeOrd>::Tracker::IS_NOOP && <E as TreeOrd>::Tracker::IS_NOOP;

    fn new() -> Self {
        ResultTracker::T(<T as TreeOrd>::Tracker::new())
    }
}

macro_rules! tuple_recast {
    ($tuple_name:ident, $tracker_name:ident, $i_len:expr, $($i:tt $t:tt),+) => {
        pub enum $tuple_name<$($t: TreeOrd,)+> {
            $($t($t::Tracker),)+
        }

        pub struct $tracker_name<$($t: TreeOrd,)+> {
            pub subtracker: $tuple_name<$($t,)+>,
            pub min_eq_len: u8,
            pub max_eq_len: u8,
        }

        impl<$($t: TreeOrd,)+> Tracker for $tracker_name<$($t,)+> {
            const IS_NOOP: bool = false;

            fn new() -> Self {
                Self {
                    subtracker: $tuple_name::A(<A as TreeOrd>::Tracker::new()),
                    min_eq_len: 0,
                    max_eq_len: 0,
                }
            }
        }

        impl<$($t: TreeOrd,)+> TreeOrd<Self> for ($($t,)+) {
            type Tracker = $tracker_name<$($t,)+>;

            fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
                let mut start = min(tracker.min_eq_len, tracker.max_eq_len);
                loop {
                    match start {
                        $(
                            $i => {
                                if !matches!(tracker.subtracker, $tuple_name::$t(_)) {
                                    tracker.subtracker =
                                        $tuple_name::$t(<$t as TreeOrd>::Tracker::new());
                                }
                                let res = if let $tuple_name::$t(ref mut subtracker) =
                                    tracker.subtracker
                                {
                                    self.$i.tree_cmp(&rhs.$i, subtracker)
                                } else {
                                    tree_cmp_unreachable()
                                };
                                match res {
                                    Less => {
                                        tracker.max_eq_len = $i;
                                        return Less
                                    }
                                    Equal => (),
                                    Greater => {
                                        tracker.min_eq_len = $i;
                                        return Greater
                                    }
                                }
                            }
                        )+
                        $i_len => return Equal,
                        _ => tree_cmp_unreachable(),
                    }
                    start = start.wrapping_add(1);
                }
            }
        }
    };
}

tuple_recast!(Tuple2, TupleTracker2, 2, 0 A, 1 B);
tuple_recast!(Tuple3, TupleTracker3, 3, 0 A, 1 B, 2 C);
tuple_recast!(Tuple4, TupleTracker4, 4, 0 A, 1 B, 2 C, 3 D);
tuple_recast!(Tuple5, TupleTracker5, 5, 0 A, 1 B, 2 C, 3 D, 4 E);
tuple_recast!(Tuple6, TupleTracker6, 6, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F);
tuple_recast!(Tuple7, TupleTracker7, 7, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G);
tuple_recast!(Tuple8, TupleTracker8, 8, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H);
tuple_recast!(Tuple9, TupleTracker9, 9, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H, 8 I);
tuple_recast!(Tuple10, TupleTracker10, 10, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H, 8 I, 9 J);
tuple_recast!(Tuple11, TupleTracker11, 11, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H, 8 I, 9 J, 10 K);
tuple_recast!(Tuple12, TupleTracker12, 12, 0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H, 8 I, 9 J, 10 K, 11 L);

// Tuple1 case
impl<A: TreeOrd> TreeOrd<Self> for (A,) {
    type Tracker = A::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.0.tree_cmp(&rhs.0, tracker)
    }
}

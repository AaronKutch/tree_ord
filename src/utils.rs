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

// for small enums we store the subtrackers in parallel
pub struct ResultTracker<T: TreeOrd, E: TreeOrd> {
    pub t: <T as TreeOrd>::Tracker,
    pub e: <E as TreeOrd>::Tracker,
}

impl<T: TreeOrd, E: TreeOrd> Tracker for ResultTracker<T, E> {
    const IS_NOOP: bool = <T as TreeOrd>::Tracker::IS_NOOP && <E as TreeOrd>::Tracker::IS_NOOP;

    fn new() -> Self {
        Self {
            t: <T as TreeOrd>::Tracker::new(),
            e: <E as TreeOrd>::Tracker::new(),
        }
    }
}

macro_rules! tuple_recast {
    ($tuple_name:ident, $tracker_name:ident, $i_len:expr, $($i:tt $s:tt $t:tt),+) => {
        pub struct $tracker_name<$($t: TreeOrd,)+> {
            pub min_eq_len: u8,
            pub max_eq_len: u8,
            $($s: <$t as TreeOrd>::Tracker,)+
        }

        impl<$($t: TreeOrd,)+> Tracker for $tracker_name<$($t,)+> {
            const IS_NOOP: bool = false;

            fn new() -> Self {
                Self {
                    min_eq_len: 0,
                    max_eq_len: 0,
                    $($s: <$t as TreeOrd>::Tracker::new(),)+
                }
            }
        }

        impl<$($t: TreeOrd,)+> TreeOrd<Self> for ($($t,)+) {
            type Tracker = $tracker_name<$($t,)+>;

            fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
                let mut start = min(tracker.min_eq_len, tracker.max_eq_len);
                match start {
                    $(
                        $i => {
                            match self.$i.tree_cmp(&rhs.$i, &mut tracker.$s) {
                                Less => {
                                    return Less
                                }
                                Equal => (),
                                Greater => {
                                    return Greater
                                }
                            }
                        }
                    )+
                    $i_len => return Equal,
                    _ => tree_cmp_unreachable(),
                }
                loop {
                    start = start.wrapping_add(1);
                    match start {
                        $(
                            $i => {
                                // the performance assumption we make is that if the
                                // first match is encountering `Equal`s, it most scenarios
                                // will usually be locking in on the next `tree_cmp` call
                                // or two (in contrast to using just `cmp` here which would
                                // miss a bound improvement for the next initial `start`)
                                tracker.$s = <$t as TreeOrd>::Tracker::new();
                                match self.$i.tree_cmp(&rhs.$i, &mut tracker.$s) {
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
                }
            }
        }
    };
}

tuple_recast!(Tuple2, TupleTracker2, 2, 0 a A, 1 b B);
tuple_recast!(Tuple3, TupleTracker3, 3, 0 a A, 1 b B, 2 c C);
tuple_recast!(Tuple4, TupleTracker4, 4, 0 a A, 1 b B, 2 c C, 3 d D);
tuple_recast!(Tuple5, TupleTracker5, 5, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E);
tuple_recast!(Tuple6, TupleTracker6, 6, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F);
tuple_recast!(Tuple7, TupleTracker7, 7, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G);
tuple_recast!(Tuple8, TupleTracker8, 8, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G, 7 h H);
tuple_recast!(Tuple9, TupleTracker9, 9, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G, 7 h H, 8 i I);
tuple_recast!(Tuple10, TupleTracker10, 10, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G, 7 h H, 8 i I, 9 j J);
tuple_recast!(Tuple11, TupleTracker11, 11, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G, 7 h H, 8 i I, 9 j J, 10 k K);
tuple_recast!(Tuple12, TupleTracker12, 12, 0 a A, 1 b B, 2 c C, 3 d D, 4 e E, 5 f F, 6 g G, 7 h H, 8 i I, 9 j J, 10 k K, 11 l L);

// Tuple1 case
impl<A: TreeOrd> TreeOrd<Self> for (A,) {
    type Tracker = A::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.0.tree_cmp(&rhs.0, tracker)
    }
}

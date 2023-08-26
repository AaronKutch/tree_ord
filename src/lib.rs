//! Note that there are "alloc" and "std" feature flags that can be turned off

#![cfg_attr(not(feature = "std"), no_std)]

use core::{
    any::TypeId,
    cell::{Cell, RefCell},
    cmp::min,
    marker::PhantomData,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
};
#[cfg(feature = "alloc")]
extern crate alloc;
use core::{cmp::Ordering, time::Duration};
use Ordering::*;

use utils::{tree_cmp_unreachable, LexicographicTracker, ResultTracker};
pub mod utils;

pub trait Tracker {
    /// Indicates if the `Tracker` is a no-op that does no prefix tracking or
    /// anything to help `TreeOrd` with. This is used by `TreeOrd` impls to
    /// avoid some branches. Generic types can use things like `const IS_NOOP:
    /// bool = <T as TreeOrd>::Tracker::IS_NOOP;` depending on if they have any
    /// tracking state themselves or not.
    const IS_NOOP: bool;

    /// Creates a new `Tracker` that starts with no known prefix
    fn new() -> Self;
}

impl Tracker for () {
    const IS_NOOP: bool = true;

    fn new() -> Self {}
}

/// Note: after first comparison with root node, the minimum or maximum node
/// should be `tree_cmp`ed with in order to handle certain close to min or close
/// to max cases
pub trait TreeOrd<Rhs = Self>
where
    Rhs: ?Sized,
{
    type Tracker: Tracker;

    fn tree_cmp(&self, rhs: &Rhs, tracker: &mut Self::Tracker) -> Ordering;
}

impl TreeOrd<Self> for () {
    type Tracker = ();

    #[inline]
    fn tree_cmp(&self, rhs: &Self, _: &mut Self::Tracker) -> Ordering {
        self.cmp(rhs)
    }
}

macro_rules! impl_simple_tree_ord {
    ($($t:ident)*) => {
        $(
            impl TreeOrd<Self> for $t {
                type Tracker = ();

                #[inline]
                fn tree_cmp(&self, rhs: &Self, _: &mut Self::Tracker) -> Ordering {
                    self.cmp(rhs)
                }
            }
        )*
    };
}

impl_simple_tree_ord!(
    usize u8 u16 u32 u64 u128 NonZeroUsize NonZeroU8 NonZeroU16 NonZeroU32 NonZeroU64 NonZeroU128
    isize i8 i16 i32 i64 i128 NonZeroIsize NonZeroI8 NonZeroI16 NonZeroI32 NonZeroI64 NonZeroI128
    bool char
    Ordering TypeId Duration
);
// TODO when stabilized in core
//IpAddr SocketAddr Ipv4Addr Ipv6Addr SocketAddrV4 SocketAddrV6

/// Wrapper that implements `TreeOrd` for any `T: Ord`. However, no `Tracker`
/// optimizations specific to the `T` are applied, which may be important to do
/// manually
#[repr(transparent)]
pub struct OrdToTreeOrd<T: Ord>(pub T);

impl<T: Ord> TreeOrd<Self> for OrdToTreeOrd<T> {
    type Tracker = ();

    #[inline]
    fn tree_cmp(&self, rhs: &Self, _: &mut Self::Tracker) -> Ordering {
        self.0.cmp(&rhs.0)
    }
}

/// Like [core::cmp::Reverse] except for `TreeOrd`
#[repr(transparent)]
pub struct TreeOrdReverse<T: TreeOrd>(pub T);

impl<T: TreeOrd> TreeOrd<Self> for TreeOrdReverse<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.0.tree_cmp(&rhs.0, tracker).reverse()
    }
}

impl<T: TreeOrd> TreeOrd<Self> for &T {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(*self, *rhs, tracker)
    }
}

impl<T: TreeOrd> TreeOrd<Self> for &mut T {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(*self, *rhs, tracker)
    }
}

impl<T: TreeOrd + Copy> TreeOrd<Self> for Cell<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(&self.get(), &rhs.get(), tracker)
    }
}

impl<T: TreeOrd + ?Sized> TreeOrd<Self> for RefCell<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.borrow().tree_cmp(&*rhs.borrow(), tracker)
    }
}

impl<T: ?Sized> TreeOrd<Self> for PhantomData<T> {
    type Tracker = ();

    #[inline]
    fn tree_cmp(&self, _: &Self, _: &mut Self::Tracker) -> Ordering {
        Ordering::Equal
    }
}

impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::boxed::Box<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::rc::Rc<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::sync::Arc<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

// TODO for `Saturating` and `Wrapping` when impls become stable

impl<T: TreeOrd> TreeOrd<Self> for Option<T> {
    type Tracker = T::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        match (self, rhs) {
            (None, None) => Equal,
            (None, Some(_)) => Less,
            (Some(_), None) => Greater,
            (Some(lhs), Some(rhs)) => lhs.tree_cmp(rhs, tracker),
        }
    }
}

impl<T: TreeOrd, E: TreeOrd> TreeOrd<Self> for Result<T, E> {
    type Tracker = ResultTracker<T, E>;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        match (self, rhs) {
            (Ok(lhs), Ok(rhs)) => {
                if !matches!(tracker, ResultTracker::T(_)) {
                    *tracker = ResultTracker::T(<T as TreeOrd>::Tracker::new());
                }
                if let ResultTracker::T(subtracker) = tracker {
                    lhs.tree_cmp(rhs, subtracker)
                } else {
                    tree_cmp_unreachable()
                }
            },
            (Ok(_), Err(_)) => Less,
            (Err(_), Ok(_)) => Greater,
            (Err(lhs), Err(rhs)) => {
                if !matches!(tracker, ResultTracker::E(_)) {
                    *tracker = ResultTracker::E(<E as TreeOrd>::Tracker::new());
                }
                if let ResultTracker::E(subtracker) = tracker {
                    lhs.tree_cmp(rhs, subtracker)
                } else {
                    tree_cmp_unreachable()
                }
            },
        }
    }
}

impl<T: TreeOrd> TreeOrd<Self> for [T] {
    type Tracker = LexicographicTracker<T>;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        // use this, because otherwise the compiler would not optimize stuff away since
        // `tracker` is an external thing to the function
        let not_noop = !Self::Tracker::IS_NOOP;
        let start = min(tracker.min_eq_len, tracker.max_eq_len);
        let end = min(self.len(), rhs.len());
        if start >= end {
            return self.len().cmp(&rhs.len())
        }
        let len = end.wrapping_sub(start);
        // enable bound check elmination in the compiler
        let lhs = &self[start..end];
        let rhs = &rhs[start..end];
        for i in 0..len {
            if not_noop && (i != tracker.subtracker_i) {
                tracker.subtracker = <T as TreeOrd>::Tracker::new();
                tracker.subtracker_i = i;
            }
            match lhs[i].tree_cmp(&rhs[i], &mut tracker.subtracker) {
                Less => {
                    if not_noop {
                        tracker.max_eq_len = i;
                    }
                    return Less
                }
                Equal => (),
                Greater => {
                    if not_noop {
                        tracker.min_eq_len = i;
                    }
                    return Greater
                }
            }
        }
        self.len().cmp(&rhs.len())
    }
}

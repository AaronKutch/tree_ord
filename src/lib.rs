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
    ops::Deref,
    pin::Pin,
};
#[cfg(feature = "alloc")]
extern crate alloc;
use core::{cmp::Ordering, time::Duration};

use utils::{LexicographicTracker, ResultTracker};
use Ordering::*;
pub mod utils;

/// A trait for structs used in `TreeOrd` impls to store prefix information
pub trait Tracker {
    /// Indicates if the `Tracker` is a no-op that does no prefix tracking or
    /// anything to help `TreeOrd` with. This can be used by `TreeOrd` impls to
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

/// An ordering trait for faster comparisons in binary tree searches
///
/// Consider doing a tree search over something like `Vec<u64>` or long byte
/// arrays. In some algorithmic contexts, there is a lot of commonality between
/// long key prefixes.
///
/// ```
/// use core::cmp::Ordering;
///
/// use Ordering::*;
///
/// let v: &[u8] = &[42, 64, 8, 0, 32];
///
/// // Suppose we are searching down a binary tree with `v`. We encounter the following
///
/// assert_eq!(v.cmp(&[50, 50, 50, 50, 50]), Less);
/// assert_eq!(v.cmp(&[42, 64, 0, 0, 0]), Greater);
/// assert_eq!(v.cmp(&[42, 64, 99, 99, 99]), Less);
/// assert_eq!(v.cmp(&[42, 64, 8, 50, 50]), Less);
/// assert_eq!(v.cmp(&[42, 64, 8, 0, 16]), Greater);
/// assert_eq!(v.cmp(&[42, 64, 8, 0, 32]), Equal);
/// ```
///
/// Everytime `v` is compared with, it starts from the very beginning to find
/// where the prefix commonality diverges. However, because we are in an ordered
/// binary tree, every time we compare `v` with a node on the right hand side
/// and find `v` to be `Greater` than the node, we know that we will not
/// encounter nodes lesser than the node we just compared with. Similarly, when
/// we encounter a `Less` case, we know that we will not encounter nodes greater
/// than the node we just encountered. After `v.cmp(&[42, 64, 0, 0, 0]) ==
/// Greater` and `v.cmp(&[42, 64, 99, 99, 99]) == Less`, we know that all the
/// subtrees we can encounter will only have prefixes starting with `[42, 64]`,
/// and thus we can skip checking that prefix for all future comparisons within
/// the current search.
///
/// We need some kind of state that tracks the minimum equal prefix and maximum
/// equal prefix, and a special comparison function that can skip the minimum of
/// the two. This is where the `Tracker` and `TreeOrd` traits come in. The
/// primitives and small fixed width types have no use for trackers, so their
/// `TreeOrd` impls use `type Tracker = ();`, which is a no-operation tracker
/// that takes up no memory. `[T]` has `type Tracker =
/// tree_ord::utils::LexicographicTracker<T>`, which has the state to track
/// prefixes and the tracker of a single `T` type.
///
/// ```
/// use core::cmp::Ordering;
///
/// use tree_ord::{Tracker, TreeOrd};
/// use Ordering::*;
///
/// let v: &[u8] = &[42, 64, 8, 0, 32];
///
/// // upon starting a new tree search, always create a new tracker
/// let mut tracker = <[u8] as TreeOrd>::Tracker::new();
/// assert_eq!(v.tree_cmp(&[50, 50, 50, 50, 50], &mut tracker), Less);
/// assert_eq!(v.tree_cmp(&[42, 64, 0, 0, 0], &mut tracker), Greater);
/// assert_eq!(v.tree_cmp(&[42, 64, 99, 99, 99], &mut tracker), Less);
/// assert_eq!(v.tree_cmp(&[42, 64, 8, 50, 50], &mut tracker), Less);
/// assert_eq!(v.tree_cmp(&[42, 64, 8, 0, 16], &mut tracker), Greater);
/// assert_eq!(v.tree_cmp(&[42, 64, 8, 0, 32], &mut tracker), Equal);
/// ```
///
/// In the small example above, only 7 comparisons get skipped and we added some
/// over head, so the performance would not have actually increased, but this
/// trait does become efficient for large equivalence graphs with large keys.
/// Depending on your use case, you can simply use the `OrdToTreeOrd<T>` wrapper
/// which opts out of all overhead.
///
/// `TreeOrd` is implemented for tuples up to length 12 that have all `TreeOrd`
/// fields, and it has subtrackers for each type of field while applying the
/// prefix optimization to itself. For example, if we have `(A, B, C)`, it will
/// start with `<A as TreeOrd>::Tracker`. If the entire `A` prefix is
/// determined, then it will only compare starting from `B` and `<B as
/// TreeOrd>::Tracker`, etc.
///
/// Note: in `tree_cmp` implementations, they should not treat `Equal` as
/// strengthening any bounds. This is because we want to handle certain
/// nonhereditary trees and other cases where the search may continue after
/// encountering an `Equal`. The search may find its way outside of the group of
/// equal keys for one move and would need to be redirected, which couldn't
/// happen if `Equal` made all future returns `Equal` or something like that.
/// Additionally, the last comparison of any valid series of comparisons is
/// allowed to be repeated any number of times.
///
/// Note: When using `TreeOrd` for a datastructure with fast access to the
/// minimum and maximum keys of a tree, the minimum or maximum keys should be
/// `tree_cmp`ed with after the root node, because if there is some bias where
/// insertions are happening close to one edge of the tree, then the tracker
/// can't optimize things like lots of leading zero bytes early because it needs
/// both `Less` and `Greater` cases. In the future we may have better
/// specializations that are aware of the absolute minimum and maximum values of
/// `T`.
pub trait TreeOrd<Rhs = Self>
where
    Self: Ord,
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

/// Wrapper that implements `TreeOrd` with a no-op `Tracker` for any `T: Ord`.
/// It may be important to implement `TreeOrd` manually for large and
/// complicated `T`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(PartialEq, Eq, PartialOrd, Ord)]
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

#[cfg(feature = "alloc")]
impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::boxed::Box<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

#[cfg(feature = "alloc")]
impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::rc::Rc<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

#[cfg(feature = "alloc")]
impl<T: TreeOrd + ?Sized> TreeOrd<Self> for alloc::sync::Arc<T> {
    type Tracker = T::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrd::tree_cmp(self.as_ref(), rhs.as_ref(), tracker)
    }
}

// TODO for `Saturating` and `Wrapping` when impls become stable

impl<P> TreeOrd<Self> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: TreeOrd,
{
    type Tracker = <<P as Deref>::Target as TreeOrd>::Tracker;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.deref().tree_cmp(rhs.deref(), tracker)
    }
}

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
            (Ok(lhs), Ok(rhs)) => lhs.tree_cmp(rhs, &mut tracker.t),
            (Ok(_), Err(_)) => Less,
            (Err(_), Ok(_)) => Greater,
            (Err(lhs), Err(rhs)) => lhs.tree_cmp(rhs, &mut tracker.e),
        }
    }
}

impl<T: TreeOrd> TreeOrd<Self> for [T] {
    type Tracker = LexicographicTracker<T>;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        // use this, because otherwise the compiler would not optimize stuff away since
        // `tracker` is an external thing to the function
        let not_noop = !<T as TreeOrd>::Tracker::IS_NOOP;
        let start = min(tracker.min_eq_len, tracker.max_eq_len);
        let end = min(self.len(), rhs.len());
        if start >= end {
            return self.len().cmp(&rhs.len())
        }
        let len = end.wrapping_sub(start);
        // enable bound check elmination in the compiler
        let x = &self[start..end];
        let y = &rhs[start..end];
        // unroll first iter to handle subtracker which tracks only the `start` element
        let i = start;
        if not_noop && (i != tracker.subtracker_i) {
            tracker.subtracker = <T as TreeOrd>::Tracker::new();
            tracker.subtracker_i = i;
        }
        match x[0].tree_cmp(&y[0], &mut tracker.subtracker) {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
        for j in 1..len {
            let i = j.wrapping_add(start);
            match x[j].cmp(&y[j]) {
                Less => {
                    tracker.max_eq_len = i;
                    return Less
                }
                Equal => (),
                Greater => {
                    tracker.min_eq_len = i;
                    return Greater
                }
            }
        }
        self.len().cmp(&rhs.len())
    }
}

impl TreeOrd<Self> for str {
    type Tracker = <[u8] as TreeOrd>::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.as_bytes().tree_cmp(rhs.as_bytes(), tracker)
    }
}

impl<T: TreeOrd, const N: usize> TreeOrd<Self> for [T; N] {
    type Tracker = <[T] as TreeOrd>::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.as_slice().tree_cmp(rhs.as_slice(), tracker)
    }
}

#[cfg(feature = "alloc")]
impl<T: TreeOrd> TreeOrd<Self> for alloc::vec::Vec<T> {
    type Tracker = <[T] as TreeOrd>::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.as_slice().tree_cmp(rhs.as_slice(), tracker)
    }
}

#[cfg(feature = "alloc")]
impl TreeOrd<Self> for alloc::string::String {
    type Tracker = <[u8] as TreeOrd>::Tracker;

    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        self.as_bytes().tree_cmp(rhs.as_bytes(), tracker)
    }
}

/// The generic `[T]` impl is not performant for `[u8]`. We can't specialize the
/// `[T]` impl on stable, so this exists to compare bytes in chunks of bytes.
/// However, it seems this is only more performant for very long slices and deep
/// trees, you should benchmark to see if this is faster for your usecase.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeOrdBytes<'a>(pub &'a [u8]);

impl<'a> TreeOrd<Self> for TreeOrdBytes<'a> {
    type Tracker = LexicographicTracker<u8>;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        // byte comparison is greatly sped up internally by
        // `core::intrinsics::compare_bytes` when we do it in chunks
        const CHUNK_LEN: usize = 32;
        let start_chunks = min(tracker.min_eq_len, tracker.max_eq_len);
        let start_bytes = start_chunks.wrapping_mul(CHUNK_LEN);
        let end_bytes = min(self.0.len(), rhs.0.len());
        let end_chunks = end_bytes.wrapping_div(CHUNK_LEN);
        if start_chunks >= end_chunks {
            if start_bytes >= end_bytes {
                return self.0.len().cmp(&rhs.0.len())
            } else {
                let x = &self.0[start_bytes..];
                let y = &rhs.0[start_bytes..];
                return x.cmp(y)
            }
        }
        let len_chunks = end_chunks.wrapping_sub(start_chunks);
        for i in 0..len_chunks {
            let start = start_chunks.wrapping_add(i).wrapping_mul(CHUNK_LEN);
            let end = start.wrapping_add(CHUNK_LEN);
            let x = &self.0[start..end];
            let y = &rhs.0[start..end];
            match x.cmp(y) {
                Less => {
                    tracker.max_eq_len = i;
                    return Less
                }
                Equal => (),
                Greater => {
                    tracker.min_eq_len = i;
                    return Greater
                }
            }
        }
        let extra_start = end_chunks.wrapping_mul(CHUNK_LEN);
        self.0[extra_start..].cmp(&rhs.0[extra_start..])
    }
}

/// The same as `TreeOrdBytes` but for an owned `Vec<u8>`
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeOrdVec(pub alloc::vec::Vec<u8>);

#[cfg(feature = "alloc")]
impl TreeOrd<Self> for TreeOrdVec {
    type Tracker = LexicographicTracker<u8>;

    #[inline]
    fn tree_cmp(&self, rhs: &Self, tracker: &mut Self::Tracker) -> Ordering {
        TreeOrdBytes(&self.0).tree_cmp(&TreeOrdBytes(&rhs.0), tracker)
    }
}

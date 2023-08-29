# Tree Ordering

This is an experiment to determine if we can improve on `Ord` in the context of binary tree
searches, where we can skip comparing the same prefixes in some cases. Note that it turns out that
this is not faster than `Ord` in most cases, although very complex and long keys can be faster.

Provides the `TreeOrd` trait, similar to `Ord` but with the ability to optimize binary tree searches.

There are "alloc" and "std" features enabled by default that can be turned off.

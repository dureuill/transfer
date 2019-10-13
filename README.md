Transfer is to move what clone is to copy
=========================================

Note: This crate, as well as [`stackpin`](https://github.com/dureuill/stackpin/) is very much a work in progress, and is published in the hope that it will be of interest for further work.

The `Transfer` trait executes user code to take a value from an unmovable instance of a struct to another instance.

In this way, it is similar to the `Clone` trait, that allows to execute user code to clone a value that is not copiable.

The `Transfer` trait is also comparable to the move constructor of C++.

Hold on, what is an unmovable struct?
-------------------------------------

Rust does not natively expose the concept of "unmovable types". However, thanks to [`Pin`](std::pin::Pin) and `unsafe`, it is possible to express this concept in the type system.
`Transfer` leverages the [`stackpin`](https://github.com/dureuill/stackpin/) crate (by the same author) to build type safe abstractions for Unmovable types.

Examples
--------

* The unit tests for `Transfer` demonstrate a `SecretU64` type, that attempt to erase itself securely when it gets out of scope.
* An example for `Transfer` is `DynRef`, a type of reference that uses an external `Lifetime` struct to represent the lifetime of `DynRef`.

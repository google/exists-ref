// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Provides "existential references", a middle ground between raw pointers and references.
//!
//! An existential reference, written as `&Exists<T>` or `&mut Exists<T>`, indicates that an
//! object of type `T` is valid and exists at a location with read or write access, but does not
//! assert the aliasing or uniqueness of the data like references do. This allows you to use
//! reference-based code and traits while preserving many of the valuable properties of raw pointers.
//!
//! This is done by taking advantage of a [quirk of ZSTs][zst]: references to them are incapable
//! of aliasing. [`Exists<T>`] provides this ZST marker type, with safe reads and writes available
//! by asserting the validity of the data ahead of time.
//!
//! [`SliceExists<T>`] extends this concept to slices, allowing for reference-like code that
//! essentially operates on raw pointers.
//!
//! WARNING: this crate is still under development and has not been rigorously reviewed for soundness
//!
//! # Examples
//! Creating and using an existential reference from a reference is safe.
//! The reference preserves the lifetime, unlike a raw pointer.
//! ```
//! # use exists_ref::Exists;
//! let mut x = 10;
//!
//! let r: &mut Exists<i32> = Exists::from_mut(&mut x);
//!
//! // Reading/writing is safe.
//! r.set(20);
//! assert_eq!(r.get(), 20);
//!
//! // Going _back_ to a mut ref is unsafe, since this could invoke UB.
//! let r: &mut i32 = unsafe { r.as_mut_unchecked() };
//! *r = 30;
//! assert_eq!(x, 30);
//! ```
//!
//! A single type can safely refer to and use a `&T` and `&Cell<T>`.
//! ```
//! # use exists_ref::Exists;
//! # use core::cell::Cell;
//! fn read_double(x: &Exists<i32>) -> i32 {
//!   x.get() * 2
//! }
//! let x = 10;
//! let y = Cell::new(20);
//! assert_eq!(read_double(Exists::from_ref(&x)), 20);
//! assert_eq!(read_double(Exists::from_cell(&y)), 40);
//! ```
//!
//! A single type can safely refer to and use a `&mut T` and `&Cell<T>`.
//! ```
//! # use exists_ref::Exists;
//! # use core::cell::Cell;
//! fn double_value(x: &mut Exists<i32>) {
//!   x.set(x.get() * 2)
//! }
//! let mut x = 10;
//! let y = Cell::new(20);
//! double_value(Exists::from_mut(&mut x));
//! double_value(Exists::from_cell(&y));
//! assert_eq!(x, 20);
//! assert_eq!(y.get(), 40);
//! ```
//!
//! `&mut Exists<T>` can be copied and "alias" each other.
//! ```
//! # use exists_ref::Exists;
//! fn double_value(x: &mut Exists<i32>) {
//!   x.set(x.get() * 2)
//! }
//! let mut x = 10;
//! let r: &mut Exists<i32> = Exists::from_mut(&mut x);
//! let [r1, r2] = r.copy_mut();
//! double_value(r1);
//! double_value(r2);
//! assert_eq!(x, 40);
//! ```
//!
//! TODO: examples using raw pointers
//!
//! # Interaction with Stacked Borrows
//!
//! This crate is tested under [Miri][miri] with the default flags.
//! However, that does not mean it will always pass under Miri as the memory model evolves.
//! It also does not guarantee that this crate cannot cause Undefined Behavior.
//!
//! This crate is entirely incompatible with the current implementation of `-Zmiri-tag-raw-pointers`.
//! This is because the cast to a ZST reference will perform a retag,
//! so a round trip cast of `*const T` to `&Exists<T>` back to `*const T` results in a pointer
//! that has a different tag than what is on the borrow stack.
//! Without the flag, the borrow stack will have an untagged entry that the `*const T` can use,
//! and the round trip succeeds.
//!
//! The below is enough to trigger Miri with `-Zmiri-tag-raw-pointers`:
//!
//! ```
//! assert_eq!(
//! unsafe {
//!     (
//!         &*(&10 as *const i32 as *const ())
//!         as *const () as *const i32
//!     ).read()
//! }, 10);
//! ```
//!
//! [miri]: https://github.com/rust-lang/miri
//! [zst]: https://github.com/rust-lang/rust-memory-model/issues/44

#![no_std]

mod exists;
pub mod slice;

pub use exists::Exists;
pub use slice::SliceExists;

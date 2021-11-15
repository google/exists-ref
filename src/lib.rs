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
//! `&mut Exists<T>` can be copied and "alias" each other
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
//! [zst]: https://github.com/rust-lang/rust-memory-model/issues/44

#![no_std]

mod exists;
pub mod slice;

pub use exists::Exists;
pub use slice::SliceExists;

use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::ptr;

/// A ZST marker that indicates a valid `T` is accessible at this location.
///
/// A valid `&Exists<T>` can safely read a `T` at its address.
/// This is distinct from being able to soundly create a `&T` at this address.
///
/// A valid `&mut Exists<T>` can safely write a `T` at its address.
/// This is distinct from being able to soundly create a `&mut T` at this address.
///
/// Since `Exists<T>` is a ZST, any reference to it is incapable of aliasing, and reading
/// and writing are done by first casting `&self` to `*const T` and `&mut self` to `*mut T`.
/// This means references to an `Exists<T>` retain a critical property of raw pointers:
/// the optimizer may assume the pointee can change between individual reads and writes,
/// but _not_ by asserting the aliasing of memory like `&T`, `&mut T`, and `&UnsafeCell<T>` do.
///
/// # Safety
/// - It is *unsound* to refer to a `Exists<T>` by value.
/// - The address of a `&Exists<T>` or `&mut Exists<T>` must be:
///   - Pointing to a properly initialized value of type `T`
///   - Non-null
///   - Aligned for `T`
///
/// # Quirks
/// Even though it's only exposed via reference, this type is a ZST, so it cannot be used
/// to modify the `T` via the reference directly:
/// ```
/// # use exists_ref::Exists;
/// # use core::mem;
/// let mut x: u64 = 10;
/// let mut y: u64 = 20;
/// let xe: &mut Exists<u64> = (&mut x).into();
/// let ye: &mut Exists<u64> = (&mut y).into();
/// assert_eq!(mem::size_of_val(xe), 0);
/// mem::swap(xe, ye);
/// assert_eq!((x, y), (10, 20));
/// mem::swap(&mut x, &mut y);
/// assert_eq!((x, y), (20, 10));
/// ```
///
/// # TODO
///
/// - Can `&'a mut Exists<T>` and `&'b mut T` coexist soundly if the former isn't used during `'b`?
/// - Can interior mutability screw up the invariants of `as_ref` and ilk? May need more precise wording.
pub struct Exists<T>(PhantomData<(UnsafeCell<T>, *const T)>);

impl<T> Exists<T> {
    /// "Casts" a shared const reference to a const existential reference.
    ///
    /// ```
    /// # use exists_ref::Exists;
    /// let x: i64 = 10;
    /// let y: &Exists<i64> = Exists::from_ref(&x);
    /// assert_eq!(y.get(), 10);
    /// ```
    #[inline]
    pub fn from_ref(val: &T) -> &Self {
        val.into()
    }

    /// "Casts" a shared mutable reference to a mut existential reference.
    ///
    /// ```
    /// # use exists_ref::Exists;
    /// # use core::cell::Cell;
    /// let x = Cell::new(10i32);
    /// let y: &mut Exists<i32> = Exists::from_cell(&x);
    /// y.set(20);
    /// assert_eq!(x.get(), 20);
    /// ```
    #[inline]
    pub fn from_cell(val: &Cell<T>) -> &mut Self {
        val.into()
    }

    /// "Casts" a unique mutable reference to a mut existential reference.
    ///
    /// ```
    /// # use exists_ref::Exists;
    /// let mut x: u64 = 10;
    /// let y: &mut Exists<u64> = Exists::from_mut(&mut x);
    /// y.set(20);
    /// assert_eq!(x, 20);
    /// ```
    #[inline]
    pub fn from_mut(val: &mut T) -> &mut Self {
        val.into()
    }

    /// Constructs an existential reference from a raw pointer.
    ///
    /// This does not create any intermediate references to `T`.
    ///
    /// # Safety
    /// For the duration of lifetime `'a`, `data` must be:
    /// - Pointing to a properly initialized value of type `T`
    /// - [Valid][valid] for reads the size of `T`
    /// - Properly aligned
    /// - Not aliasing a `&mut T`, since that would disallow safe reads
    ///
    /// If the result is unused, the only requirement is that `data` be a
    /// pointer to allocated memory.
    ///
    /// # Example
    /// ```
    /// # use exists_ref::Exists;
    /// let x: [u64; 2] = [10, 20];
    /// let y: *const u64 = x.as_ptr();
    /// let z: &Exists<u64> = unsafe { Exists::from_ptr(y.add(1)) };
    /// assert_eq!(z.get(), 20);
    /// ```
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    pub unsafe fn from_ptr<'a>(data: *const T) -> &'a Self {
        &*(data as *const Self)
    }

    /// Constructs a mutable existential reference from a raw pointer.
    ///
    /// This does not create any intermediate references to `T`.
    ///
    /// # Safety
    /// For the duration of lifetime `'a`, `data` must be:
    /// - Pointing to a properly initialized value of type `T`
    /// - [Valid][valid] for both reads and writes the size of `T`
    /// - Properly aligned
    /// - Not aliasing a `&T` or `&mut T`, since that would disallow safe writes
    ///
    /// If the result is unused, the only requirement is that `data` be a pointer
    /// to allocated memory.
    ///
    /// # Example
    /// ```
    /// # use exists_ref::Exists;
    /// let mut x: [u64; 2] = [10, 20];
    /// let y: *mut u64 = x.as_mut_ptr();
    /// let z: &mut Exists<u64> = unsafe { Exists::from_mut_ptr(y.add(1)) };
    ///
    /// // This would be UB if this `z` were `&mut u64`!
    /// z.set(30);
    /// assert_eq!(x, [10, 30]);
    /// z.set(50);
    /// assert_eq!(x, [10, 50]);
    /// ```
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    pub unsafe fn from_mut_ptr<'a>(data: *mut T) -> &'a mut Self {
        &mut *(data as *mut Self)
    }

    /// Asserts that the memory pointed to by `&self` can be written to, enabling
    /// safe mutating operations.
    ///
    /// # Safety
    /// In addition to the safety requirements for `&self`, the `size_of::<T>()` bytes
    /// of memory pointed to by `&'a self` must be:
    /// - [Valid][valid] for writes the size of `T`
    /// - Not aliasing a `&T` or `&mut T`, since that would disallow safe writes
    ///
    /// If the resulting `&mut Exists<T>` doesn't perform any writes,
    /// this function will not invoke UB on its own.
    ///
    /// # Example
    /// ```
    /// # use exists_ref::Exists;
    /// let get_writeable = true;
    /// let x = 0;
    /// let mut y = 1;
    /// let e: &Exists<i32> = if get_writeable {
    ///   Exists::from_mut(&mut y)
    ///   // This would invoke UB, since it would write to a `&T`!
    ///   // Exists::from_ref(&y)
    /// } else {
    ///   Exists::from_ref(&x)
    /// };
    /// // ...
    /// if get_writeable {
    ///   // Safety: `e` was definitely derived from a `&mut i32`.
    ///   unsafe { e.assume_mut() }.set(5);
    ///   assert_eq!(y, 5);
    /// }
    /// assert_eq!(x, 0);
    /// ```
    pub unsafe fn assume_mut(&self) -> &mut Self {
        &mut *(self as *const Self as *mut Self)
    }

    /// Safely copies this mutable existential reference into multiple identical references.
    ///
    /// Since this type does not assert aliasing of pointed memory, this can be done
    /// safely.
    ///
    /// # Examples
    /// ```
    /// # use exists_ref::Exists;
    /// let mut x = 10u32;
    /// let a: &mut Exists<u32> = (&mut x).into();
    /// let [a, b] = a.copy_mut();
    /// a.set(20);
    /// b.set(30);
    /// assert_eq!(a.get(), 30);
    /// ```
    pub fn copy_mut<const N: usize>(&mut self) -> [&mut Self; N] {
        [self as *mut Self; N].map(|x| unsafe { &mut *x })
    }

    /// Returns a raw pointer to the underlying data being referenced by this `Exists<T>`.
    pub fn as_ptr(&self) -> *const T {
        self as *const Self as *const T
    }

    /// Returns a raw pointer to the underlying data being referenced by this `Exists<T>`.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut Self as *mut T
    }

    /// Returns a shared reference that this `Exists<T>` points to.
    ///
    /// # Safety
    /// For the duration of lifetime `'a`, `data` must be:
    /// - Pointing to a properly initialized value of type `T`
    /// - [Valid][valid] for reads the size of `T`
    /// - Properly aligned
    /// - Pointing to memory that will not mutate unless behind an `UnsafeCell`.
    ///   This *includes* writing via [`Exists::set`].
    /// - Not aliasing a `&UnsafeCell<T>` or `&mut T`
    ///
    /// You must enforce Rust's aliasing rules regarding `&T`.
    /// In particular, for the duration of this lifetime, the
    /// pointee must not get mutated (except inside `UnsafeCell`).
    /// This applies even if the result is unused.
    ///
    /// TODO: is a better safety rule "this `&Exists<T>` was originally derived from a `&T`"?
    ///
    /// # Examples
    /// ```
    /// # use exists_ref::Exists;
    /// let mut x = 10u32;
    /// let a: &mut Exists<u32> = (&mut x).into();
    /// let b: &u32 = unsafe { a.as_ref_unchecked() };
    /// assert_eq!(*b, 10);
    /// ```
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    pub unsafe fn as_ref_unchecked(&self) -> &T {
        &*self.as_ptr()
    }

    /// # Safety
    /// For the duration of lifetime `'a`, `data` must be:
    /// - Pointing to a properly initialized value of type `T`
    /// - [Valid][valid] for both reads and writes the size of `T`
    /// - Properly aligned
    /// - Not aliasing a `&T` or `&mut T`
    ///
    /// TODO: is a better safety rule "this `&mut Exists<T>` was originally derived from a `&Cell<T>`"?
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    pub unsafe fn as_cell_unchecked(&mut self) -> &Cell<T> {
        // Safety: `Cell<T>` is transparent over `T`
        &*(self.as_mut_ptr() as *mut Cell<T>)
    }

    /// # Safety
    /// For the duration of lifetime `'a`, `data` must be:
    /// - Pointing to a properly initialized value of type `T`
    /// - [Valid][valid] for both reads and writes the size of `T`
    /// - Properly aligned
    /// - Not read or written to via any methods other than the return value.
    ///   This *includes* writing via [`Exists::set`].
    /// - Not aliasing a `&T`, `&UnsafeCell<T>` or `&mut T`
    ///
    /// TODO: test if a `&Cell<T>` into `&mut T` via `&Exists<T>` is UB if no other `&Cell<T>` exist.
    ///
    /// You must enforce Rust's aliasing rules regarding `&mut T`.
    /// In particular, for the duration of this lifetime, the
    /// pointee must not get accessed (read or written) through any
    /// other pointer. This applies even if the result is unused.
    ///
    /// TODO: is a better safety rule "this `&mut Exists<T>` was originally derived from a `&mut T`"?
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    pub unsafe fn as_mut_unchecked(&mut self) -> &mut T {
        &mut *self.as_mut_ptr()
    }

    /// Swaps the values of two mutable locations of the same type,
    /// without deinitializing either one.
    ///
    /// This is semantically similar to [`core::mem::swap`], but
    /// with the notable exception that the two pointed-to-values may overlap.
    ///
    /// If they do overlap, then the overlapping region of memory from
    /// `&mut self` will be used.
    pub fn swap(&mut self, other: &mut Exists<T>) {
        // Safety: the two raw pointers are guaranteed to be valid for reads/writes
        // and aligned as an invariant of the type.
        unsafe { ptr::swap(self.as_mut_ptr(), other.as_mut_ptr()) }
    }

    /// Replaces the contained value with `val`, and returns the old pointed value.
    pub fn replace(&mut self, val: T) -> T {
        // Safety: the two raw pointers are guaranteed to be valid for reads/writes
        // aligned, and initialized as an invariant of the type.
        unsafe { ptr::replace(self.as_mut_ptr(), val) }
    }
}

impl<T: Copy> Exists<T> {
    /// Gets the value at the address of `&self`. Equivalent to a raw pointer read.
    pub fn get(&self) -> T {
        unsafe { self.as_ptr().read() }
    }

    /// Sets a value at the address of `&mut self`. Equivalent to a raw pointer write.
    pub fn set(&mut self, src: T) {
        unsafe { self.as_mut_ptr().write(src) }
    }
}

impl<T: Default> Exists<T> {
    /// Takes the value out of this location, leaving `Default::default()` in its place.
    pub fn take(&mut self) -> T {
        self.replace(Default::default())
    }
}

impl<'a, T: 'a> From<&'a T> for &'a Exists<T> {
    /// Constructs an existential reference from a shared reference.
    fn from(item: &'a T) -> &'a Exists<T> {
        // Safety: the raw pointer is derived from a valid reference.
        unsafe { Exists::from_ptr(item) }
    }
}

impl<'a, T: 'a> From<&'a Cell<T>> for &'a mut Exists<T> {
    /// Constructs an existential reference from a shared mutable reference.
    fn from(item: &'a Cell<T>) -> &'a mut Exists<T> {
        // Safety: the raw pointer is derived from a valid shared reference to mutable data.
        unsafe { Exists::from_mut_ptr(item.as_ptr()) }
    }
}

impl<'a, T: 'a> From<&'a mut T> for &'a mut Exists<T> {
    /// Constructs an existential reference from a unique mutable reference.
    fn from(item: &'a mut T) -> &'a mut Exists<T> {
        // Safety: the raw pointer is derived from a valid mut reference.
        unsafe { Exists::from_mut_ptr(item) }
    }
}

impl<T> AsRef<Exists<T>> for Exists<Cell<T>> {
    fn as_ref(&self) -> &Exists<T> {
        // Safety: `Cell` is transparent over `T`
        unsafe { Exists::from_ptr(self.as_ptr() as *const T) }
    }
}

impl<T> AsMut<Exists<T>> for Exists<Cell<T>> {
    fn as_mut(&mut self) -> &mut Exists<T> {
        // Safety: `Cell` is transparent over `T`
        unsafe { Exists::from_mut_ptr(self.as_ptr() as *mut T) }
    }
}

impl<'a, T: 'a> From<&'a Exists<Cell<T>>> for &'a mut Exists<T> {
    fn from(x: &'a Exists<Cell<T>>) -> &'a mut Exists<T> {
        // Safety: `Cell` is transparent over `T`, and `&Cell<T>` indicates that the `T` is mutable.
        unsafe { Exists::from_mut_ptr(x.as_ptr() as *mut T) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_read() {
        let x = 10;
        assert_eq!(Exists::from_ref(&x).get(), 10);
    }

    #[test]
    fn trivial_write() {
        let mut x = 10;
        assert_eq!(Exists::from_mut(&mut x).replace(20), 10);
        assert_eq!(x, 20);
    }

    #[test]
    fn mut_roundtrip() {
        let mut x: u64 = 10;
        let xe: &mut Exists<u64> = (&mut x).into();

        let [xe, xe2] = xe.copy_mut();

        let xer: &Exists<u64> = xe;

        let xem = unsafe {
            let xem: &mut Exists<u64> = xer.assume_mut();
            xem.set(20);
            assert_eq!(xer.get(), 20);
            xem
        };
        assert_eq!(xem.replace(10), 20);
        assert_eq!(xe.replace(30), 10);
        assert_eq!(xe2.replace(40), 30);
    }

    #[test]
    fn immut_roundtrip() {
        let x: u64 = 10;
        let xe: &Exists<u64> = Exists::from_ref(&x);

        // If `xe` is not used to read, no UB is invoked
        let xe = unsafe { xe.assume_mut() };
        assert_eq!(xe.get(), 10);
        let xe = unsafe { xe.as_ref_unchecked() };
        assert_eq!(*xe, 10);
    }

    #[test]
    fn cell_roundtrip() {
        let x: Cell<u64> = Cell::new(10);
        let xe: &Exists<u64> = Exists::from_cell(&x);
        assert_eq!(xe.get(), 10);
        let xe = unsafe { xe.assume_mut() };
        xe.set(20);
        let xe = unsafe { xe.as_cell_unchecked() };
        assert_eq!(xe.get(), 20);
    }

    #[test]
    fn test_box() {
        extern crate alloc;
        use alloc::boxed::Box;
        let x: Box<Cell<u64>> = Box::new(Cell::new(10));
        let _xe: &mut Exists<u64> = Exists::from_cell(&x);
    }

    #[test]
    fn copy_mut_ref() {
        let mut x = 10;
        let e = Exists::from_mut(&mut x);
        let [e1, e2] = e.copy_mut();

        // TODO: this violates the invariants of as_mut_unchecked() technically
        let e1: &mut i32 = unsafe { e1.as_mut_unchecked() };
        *e1 = 20;
        let e2: &mut i32 = unsafe { e2.as_mut_unchecked() };
        *e2 = 30;
        assert_eq!(x, 30);
    }

    #[test]
    fn copy_mut_ref_via_assume_mut() {
        let mut x = 10;
        let e1: &Exists<_> = Exists::from_mut(&mut x);
        let e2 = e1;

        let e1: &mut i32 = unsafe { e1.assume_mut().as_mut_unchecked() };
        *e1 = 20;
        let e2: &mut i32 = unsafe { e2.assume_mut().as_mut_unchecked() };
        *e2 = 30;
        assert_eq!(x, 30);
    }

    // TODO: more rigorous testing
}

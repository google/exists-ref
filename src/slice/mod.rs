//! [`Exists<T>`] but for `[T]`, and operations on existential slice references.
//!
//! ## Why not `Exists<[T]>`?
//! Because `Exists<T>` should be a ZST, while `Exists<[T]>` must be a
//! zero-sized DST. This can't be done without specialization.
//!

use core::cell::Cell;
use core::ops::Index;
use core::ops::IndexMut;

use crate::Exists;

mod index;
mod iter;
pub use index::SliceExistsIndex;

/// A DST marker that indicates a `[T]` is accessible at this location.
///
/// This operates similarly to [`Exists<T>`], but for a slice of `T`'s.
/// It is implemented as a slice of ZSTs that similarly cannot alias, but
/// still requires a wide pointer to reference which stores length info.
///
/// The address of `&SliceExists<T>` or `&mut SliceExists<T>` must be:
/// - Non-null
/// - Aligned to `T`
/// - Pointing to [`SliceExists::len()`] properly initialized values of type `T`
///
/// A valid `&SliceExists<T>` can safely read `len()` `T`'s in its address.
/// This is distinct from being able to soundly create a `&[T]` at this address.
///
/// A valid `&mut SliceExists<T>` can safely write `len()` `T`'s in its address.
/// This is distinct from being able to soundly create a `&mut [T]` at this address.
///
/// ## Quirks
/// - Even though it's only exposed via wide reference, this type is always zero-sized,
///   so [`core::mem::size_of_val`] always returns 0.
#[repr(transparent)]
pub struct SliceExists<T>([Exists<T>]);

impl<T> SliceExists<T> {
    /// "Casts" a shared const slice reference to a const existential slice reference.
    #[inline]
    pub fn from_ref(val: &[T]) -> &Self {
        val.into()
    }

    /// "Casts" a shared mutable slice reference to a mut existential slice reference.
    #[inline]
    pub fn from_cell(val: &Cell<[T]>) -> &mut Self {
        val.into()
    }

    /// "Casts" a shared mutable slice reference to a mut existential slice reference.
    #[inline]
    pub fn from_cell_slice(val: &[Cell<T>]) -> &mut Self {
        val.into()
    }

    /// "Casts" a unique mutable slice reference to a mut existential slice reference.
    #[inline]
    pub fn from_mut(val: &mut [T]) -> &mut Self {
        val.into()
    }

    /// Constructs an existential slice reference from a raw slice pointer.
    ///
    /// This does not create any intermediate references to `T`.
    ///
    /// # Safety
    /// For the duration of lifetime `'a`, `data` with length metadata `len` must be:
    /// - [Valid][valid] for reads for `len * mem::size_of::<T>()` many bytes.
    /// - Pointing to `len` contiguous properly initialized values of type `T`.
    /// - Properly aligned.
    /// - Not aliasing a `&mut T`, since that would disallow safe reads.
    ///
    /// In addition:
    /// - The entire memory range must be contained within a single allocated object.
    /// - The total size `len * mem::size_of::<T>()` of the slice must be no larger than `isize::MAX`.
    ///
    /// If the result is unused, the only requirement is that `data` point to allocated memory.
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    #[inline]
    pub unsafe fn from_ptr<'a>(data: *const [T]) -> &'a Self {
        &*(data as *const Self)
    }

    /// Constructs a mutable existential slice reference from a raw slice pointer.
    ///
    /// This does not create any intermediate references to `T`.
    ///
    /// # Safety
    /// For the duration of lifetime `'a`, `data` with length metadata `len` must be:
    /// - [Valid][valid] for reads and writes for `len * mem::size_of::<T>()` many bytes.
    /// - Pointing to `len` contiguous properly initialized values of type `T`.
    /// - Properly aligned.
    /// - Not aliasing a `&T` or `&mut T`, since that would disallow safe writes.
    ///
    /// In addition:
    /// - The entire memory range must be contained within a single allocated object.
    /// - The total size `len * mem::size_of::<T>()` of the slice must be no larger than `isize::MAX`.
    ///
    /// If the result is unused, the only requirement is that `data` point to allocated memory.
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    #[inline]
    pub unsafe fn from_mut_ptr<'a>(data: *mut [T]) -> &'a mut Self {
        &mut *(data as *mut Self)
    }

    /// Asserts that the memory pointed to by `&self` can be written to, enabling
    /// safe mutating operations.
    ///
    /// # Safety
    /// In addition to the safety requirements for `&self`, the `len * size_of::<T>()` bytes
    /// of memory pointed to by `&'a self` must be:
    /// - [Valid][valid] for writes for `len * size_of::<T>()` bytes
    /// - Not aliasing a `&T` or `&mut T`, since that would disallow safe writes
    ///
    /// If the result does perform any writes, this function will not cause UB.
    #[inline]
    pub unsafe fn assume_mutable(&self) -> &mut Self {
        &mut *(self as *const Self as *mut Self)
    }

    /// Safely copies this mutable existential reference into multiple identical references.
    ///
    /// Since this type does not assert aliasing of pointed memory, this can be done safely.
    ///
    /// TODO: confirm if I'm cuckoo bananas here
    ///
    /// # Examples
    /// ```
    /// # use exists_ref::SliceExists;
    /// let mut x = [10u32; 10];
    /// let a: &mut SliceExists<u32> = (&mut x[..]).into();
    /// let [a, b] = a.copy_mut();
    /// a[2].set(20);
    /// b[2].set(30);
    /// assert_eq!(a[2].get(), 30);
    /// ```
    #[inline]
    pub fn copy_mut<const N: usize>(&mut self) -> [&mut Self; N] {
        [self as *mut Self; N].map(|x| unsafe { &mut *x })
    }

    /// Returns the number of elements in the slice.
    ///
    /// This does not perform any reads on the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns a raw pointer to the existing slice's buffer.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    /// Returns a raw mutable pointer to the existing slice's buffer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr() as *mut T
    }

    /// Returns an existential reference to an element or subslice depending on the type of index.
    ///
    /// - If given a position, returns a `&Exists<T>` at that position or `None` if out of bounds.
    /// - If given a range, returns a `&SliceExists<T>` corresponding to that range, or `None` if
    ///   out of bounds.
    ///
    /// This does not create an intermediate `&T` or `&[T]`.
    #[inline]
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceExistsIndex<SliceExists<T>>,
    {
        index.get(self)
    }

    /// Returns a mutable existential reference to an element or subslice depending on the type of
    /// index (see [`get`]) or `None` if the index is out of bounds.
    ///
    /// This does not create an intermediate `&mut T` or `&mut [T]`.
    ///
    /// [`get`]: SliceExists::get
    #[inline]
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceExistsIndex<SliceExists<T>>,
    {
        index.get_mut(self)
    }

    /// Returns an existential reference to an element or subslice, without doing bounds checking.
    ///
    /// For a safe alternative, see [`get`].
    /// This does not create an intermediate `&T` or `&[T]`.
    ///
    /// # Safety
    /// Calling this method with an out-of-bounds index is undefined behavior even if the result is unused.
    ///
    /// [`get`]: SliceExists::get
    #[inline]
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &I::Output
    where
        I: SliceExistsIndex<SliceExists<T>>,
    {
        index.get_unchecked(self)
    }

    /// Returns a mutable existential reference to an element or subslice, without doing bounds checking.
    ///
    /// For a safe alternative, see [`get_mut`].
    /// This does not create an intermediate `&mut T` or `&mut [T]`.
    ///
    /// # Safety
    /// Calling this method with an out-of-bounds index is undefined behavior even if the result is unused.
    ///
    /// [`get_mut`]: SliceExists::get_mut
    #[inline]
    pub unsafe fn get_unchecked_mut<I>(&mut self, index: I) -> &mut I::Output
    where
        I: SliceExistsIndex<SliceExists<T>>,
    {
        index.get_unchecked_mut(self)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Exists<T>> {
        self.into_iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Exists<T>> {
        self.into_iter()
    }

    #[inline]
    pub fn chunks(&self, chunk_size: usize) -> impl Iterator<Item = &SliceExists<T>> {
        iter::Chunks {
            v: self,
            chunk_size,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn split_at(&self, index: usize) -> (&Self, &Self) {
        // todo: maybe optimize?
        (&self[..index], &self[index..])
    }
}

impl<'a, T: 'a> From<&'a [T]> for &'a SliceExists<T> {
    /// Constructs an existential slice reference from a shared reference.
    #[inline]
    fn from(item: &'a [T]) -> &'a SliceExists<T> {
        // Safety: the raw pointer is derived from a valid reference.
        unsafe { SliceExists::from_ptr(item) }
    }
}

impl<'a, T: 'a> From<&'a Cell<[T]>> for &'a mut SliceExists<T> {
    /// Constructs an existential slice reference from a shared mutable reference.
    #[inline]
    fn from(item: &'a Cell<[T]>) -> &'a mut SliceExists<T> {
        item.as_slice_of_cells().into()
    }
}

impl<'a, T: 'a> From<&'a [Cell<T>]> for &'a mut SliceExists<T> {
    /// Constructs an existential slice reference from a shared reference of cells.
    #[inline]
    fn from(item: &'a [Cell<T>]) -> &'a mut SliceExists<T> {
        // Safety: the raw pointer is derived from a valid shared reference to mutable data.
        unsafe { SliceExists::from_mut_ptr(item as *const [Cell<T>] as *const [T] as *mut [T]) }
    }
}

impl<'a, T: 'a> From<&'a mut [T]> for &'a mut SliceExists<T> {
    /// Constructs an existential slice reference from a unique mutable reference.
    #[inline]
    fn from(item: &'a mut [T]) -> &'a mut SliceExists<T> {
        // Safety: the raw pointer is derived from a valid mut reference.
        unsafe { SliceExists::from_mut_ptr(item) }
    }
}

impl<T, I> Index<I> for SliceExists<T>
where
    I: SliceExistsIndex<SliceExists<T>>,
{
    type Output = <I as SliceExistsIndex<SliceExists<T>>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        index.index(self)
    }
}

impl<T, I> IndexMut<I> for SliceExists<T>
where
    I: SliceExistsIndex<SliceExists<T>>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.index_mut(self)
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn
}

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

use crate::slice::SliceExists;
use crate::Exists;
use core::{ops, ptr};

mod sealed {
    use core::ops;

    pub trait Sealed {}

    impl Sealed for usize {}
    impl Sealed for ops::Range<usize> {}
    impl Sealed for ops::RangeTo<usize> {}
    impl Sealed for ops::RangeFrom<usize> {}
    impl Sealed for ops::RangeFull {}
    impl Sealed for ops::RangeInclusive<usize> {}
    impl Sealed for ops::RangeToInclusive<usize> {}
    impl Sealed for (ops::Bound<usize>, ops::Bound<usize>) {}
}

/// A valid index on an existential slice reference.
pub trait SliceExistsIndex<T: ?Sized>: sealed::Sealed + Sized {
    /// The output type returned by methods.
    type Output: ?Sized;

    /// Returns a shared reference to the output at this location, if in bounds.
    fn get(self, slice: &T) -> Option<&Self::Output>;

    /// Returns a mutable reference to the output at this location, if in bounds.
    fn get_mut(self, slice: &mut T) -> Option<&mut Self::Output>;

    /// Returns a shared reference to the output at this location, without
    /// performing any bounds checking.
    ///
    /// Calling this method with an out-of-bounds index or invalid `slice` is
    /// undefined behavior even if the resulting reference is not used.
    unsafe fn get_unchecked(self, slice: &T) -> &Self::Output;

    /// Returns a mutable reference to the output at this location, without
    /// performing any bounds checking.
    ///
    /// Calling this method with an out-of-bounds index or invalid `slice` is
    /// undefined behavior even if the resulting reference is not used.
    unsafe fn get_unchecked_mut(self, slice: &mut T) -> &mut Self::Output;

    /// Returns a shared reference to the output at this location,
    /// panicking if out of bounds.
    #[track_caller]
    fn index(self, slice: &T) -> &Self::Output;

    /// Returns a mutable reference to the output at this location,
    /// panicking if out of bounds.
    #[track_caller]
    fn index_mut(self, slice: &mut T) -> &mut Self::Output;
}

impl<T> SliceExistsIndex<SliceExists<T>> for usize {
    type Output = Exists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Exists<T>> {
        (self < slice.len()).then(|| unsafe { self.get_unchecked(slice) })
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Exists<T>> {
        (self < slice.len()).then(|| unsafe { self.get_unchecked_mut(slice) })
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Exists<T> {
        Exists::from_ptr(slice.as_ptr().add(self))
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Exists<T> {
        Exists::from_mut_ptr(slice.as_mut_ptr().add(self))
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        self.get(slice)
            .unwrap_or_else(|| slice_index_past_end(self, slice.len()))
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        let len = slice.len();
        self.get_mut(slice)
            .unwrap_or_else(|| slice_index_past_end(self, len))
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::Range<usize> {
    type Output = SliceExists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Self::Output> {
        (self.start > self.end || self.end > slice.len())
            .then(|| unsafe { self.get_unchecked(slice) })
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        (self.start > self.end || self.end > slice.len())
            .then(|| unsafe { self.get_unchecked_mut(slice) })
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Self::Output {
        SliceExists::from_ptr(ptr::slice_from_raw_parts(
            slice.as_ptr().add(self.start),
            self.end - self.start,
        ))
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        SliceExists::from_mut_ptr(ptr::slice_from_raw_parts_mut(
            slice.as_mut_ptr().add(self.start),
            self.end - self.start,
        ))
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end)
        } else if self.end > slice.len() {
            slice_end_index_len_fail(self.end, slice.len())
        }
        unsafe { self.get_unchecked(slice) }
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end)
        } else if self.end > slice.len() {
            slice_end_index_len_fail(self.end, slice.len())
        }
        unsafe { self.get_unchecked_mut(slice) }
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::RangeTo<usize> {
    type Output = SliceExists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Self::Output> {
        (0..self.end).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        (0..self.end).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Self::Output {
        (0..self.end).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        (0..self.end).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        (0..self.end).index(slice)
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        (0..self.end).index_mut(slice)
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::RangeFrom<usize> {
    type Output = SliceExists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Self::Output> {
        (self.start..slice.len()).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        (self.start..slice.len()).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Self::Output {
        (self.start..slice.len()).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        (self.start..slice.len()).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        if self.start > slice.len() {
            slice_start_index_len_fail(self.start, slice.len());
        }
        unsafe { self.get_unchecked(slice) }
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        if self.start > slice.len() {
            slice_start_index_len_fail(self.start, slice.len());
        }
        unsafe { self.get_unchecked_mut(slice) }
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::RangeFull {
    type Output = SliceExists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Self::Output> {
        Some(slice)
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        Some(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Self::Output {
        slice
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        slice
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        slice
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        slice
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::RangeInclusive<usize> {
    type Output = SliceExists<T>;

    fn get(self, _slice: &SliceExists<T>) -> Option<&Self::Output> {
        todo!()
    }

    fn get_mut(self, _slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        todo!()
    }

    unsafe fn get_unchecked(self, _slice: &SliceExists<T>) -> &Self::Output {
        todo!()
    }

    unsafe fn get_unchecked_mut(self, _slice: &mut SliceExists<T>) -> &mut Self::Output {
        todo!()
    }

    fn index(self, _slice: &SliceExists<T>) -> &Self::Output {
        todo!()
    }

    fn index_mut(self, _slice: &mut SliceExists<T>) -> &mut Self::Output {
        todo!()
    }
}

impl<T> SliceExistsIndex<SliceExists<T>> for ops::RangeToInclusive<usize> {
    type Output = SliceExists<T>;

    #[inline]
    fn get(self, slice: &SliceExists<T>) -> Option<&Self::Output> {
        (0..=self.end).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: &mut SliceExists<T>) -> Option<&mut Self::Output> {
        (0..=self.end).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: &SliceExists<T>) -> &Self::Output {
        (0..=self.end).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        (0..=self.end).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: &SliceExists<T>) -> &Self::Output {
        (0..=self.end).index(slice)
    }

    #[inline]
    fn index_mut(self, slice: &mut SliceExists<T>) -> &mut Self::Output {
        (0..=self.end).index_mut(slice)
    }
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_index_past_end(index: usize, len: usize) -> ! {
    panic!("index {} out of range for slice of length {}", index, len)
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_index_order_fail(index: usize, end: usize) -> ! {
    panic!("slice index starts at {} but ends at {}", index, end)
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_start_index_len_fail(index: usize, len: usize) -> ! {
    panic!(
        "range start index {} out of range for slice of length {}",
        index, len
    )
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
    panic!(
        "range end index {} out of range for slice of length {}",
        index, len
    )
}

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

use core::{marker::PhantomData, ptr::NonNull};

use crate::{slice::SliceExists, Exists};

impl<'a, T> IntoIterator for &'a SliceExists<T> {
    type Item = &'a Exists<T>;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a, T> IntoIterator for &'a mut SliceExists<T> {
    type Item = &'a mut Exists<T>;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut::new(self)
    }
}

pub struct Iter<'a, T> {
    ptr: NonNull<T>,
    end: *const T,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> Iter<'a, T> {
    fn new(slice: &SliceExists<T>) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(slice.as_ptr() as *mut T) },
            end: unsafe { slice.as_ptr().add(slice.len()) },
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a Exists<T>;

    // TODO: better implementations
    fn next(&mut self) -> Option<Self::Item> {
        let p: *const T = self.ptr.as_ptr();
        (p < self.end).then(|| unsafe {
            self.ptr = NonNull::new_unchecked(p.add(1) as *mut T);
            Exists::from_ptr(p)
        })
    }
}

pub struct IterMut<'a, T> {
    ptr: NonNull<T>,
    end: *mut T,
    _phantom: PhantomData<&'a mut T>,
}

impl<'a, T> IterMut<'a, T> {
    fn new(slice: &mut SliceExists<T>) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(slice.as_mut_ptr()) },
            end: unsafe { slice.as_mut_ptr().add(slice.len()) },
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut Exists<T>;

    // TODO: better implementations
    fn next(&mut self) -> Option<Self::Item> {
        let p: *mut T = self.ptr.as_ptr();
        (p < self.end).then(|| unsafe {
            self.ptr = NonNull::new_unchecked(p.add(1));
            Exists::from_mut_ptr(p)
        })
    }
}

pub struct Chunks<'a, T> {
    pub v: &'a SliceExists<T>,
    pub chunk_size: usize,
}

impl<'a, T: 'a> Iterator for Chunks<'a, T> {
    type Item = &'a SliceExists<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.v.is_empty() {
            None
        } else {
            let len = core::cmp::min(self.v.len(), self.chunk_size);
            let (before, after) = self.v.split_at(len);
            self.v = after;
            Some(before)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::SliceExists;

    #[test]
    fn test_iteration() {
        extern crate alloc;
        use alloc::vec::Vec;
        let x = [1, 2, 3, 4, 5];
        let y: Vec<i32> = SliceExists::from_ref(&x)
            .iter()
            .map(|x| x.get() * 2)
            .collect();
        assert_eq!(&y[..], &[2, 4, 6, 8, 10]);
    }
}

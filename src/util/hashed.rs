// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! A wrapper that overrides the [`Debug`][std::fmt::Debug] trait
//!
//! The seminal use case for this was to avoid printing lengthy strings. Suppose
//! you have the input to the compilation process stored in a [`String`][]. The
//! data may be very long, but you'd like to be able to reference it succinctly
//! in a log message. Thus, we refer to it using its 16 digit hexadecimal hash.
//! Instead of rewriting many custom [`Debug`][std::fmt::Debug] implementations,
//! you can simply wrap a type in a [`Hashed`][] and then derive
//! [`Debug`][std::fmt::Debug] on the enclosing data structure.
//!
//! Internally, this uses the standard library's
//! [DefaultHasher][std::collections::hash_map::DefaultHasher] with the same
//! initial state every time. Thus, the result of the hash will be stable.
//!
//! The `Debug` output follows the following format:
//! ```text
//! Hashed(0xba245e48b8b24873)
//! ```
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

/// A wrapper that overrides the [`Debug`][std::fmt::Debug] trait
///
/// See the [module documentation][crate::util::hashed] for details.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Hashed<T: Hash> {
    pub value: T,
}

impl<T: Hash> Hashed<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.value.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash> Deref for Hashed<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash> DerefMut for Hashed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Hash> std::fmt::Debug for Hashed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hashed({:#x})", self.get_hash())
    }
}

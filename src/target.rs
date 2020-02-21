// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

/// Code generation target
pub mod linux;

use crate::front::types::IntegerType;

#[doc(hidden)]
#[macro_export]
macro_rules! impl_data_representation {
    ($target: ident {$($nominal:ident = $real:ident),+}) => (
        impl DataRepresentation for $target {
            fn integer_sizeof(&self, t: IntegerType) -> usize {
                match t {
                    $(
                        IntegerType::$nominal => std::mem::size_of::<$real>(),
                    )+
                }
            }

            fn integer_max_value(&self, t: IntegerType) -> u64 {
                match t {
                    $(
                        IntegerType::$nominal => $real::max_value() as u64,
                    )+
                }
            }

            fn integer_min_value(&self, t: IntegerType) -> i64 {
                match t {
                    $(
                        IntegerType::$nominal => $real::min_value() as i64,
                    )+
                }
            }
        }
    )
}

pub trait DataRepresentation {
    fn integer_sizeof(&self, t: IntegerType) -> usize;
    fn integer_max_value(&self, t: IntegerType) -> u64;
    fn integer_min_value(&self, t: IntegerType) -> i64;
}

pub trait Target: std::fmt::Debug + DataRepresentation {}

// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use crate::front::types::IntegerType;
use crate::impl_data_representation;
use crate::target::{DataRepresentation, Target};

#[derive(Clone, Debug)]
pub struct X64Linux;

impl_data_representation! {
    X64Linux {
        UnsignedInt = u32,
        UnsignedShort = u16,
        UnsignedLong = u64,
        UnsignedLongLong = u64,

        SignedInt = i32,
        SignedShort = i16,
        SignedLong = i64,
        SignedLongLong = i64
    }
}

impl Target for X64Linux {}

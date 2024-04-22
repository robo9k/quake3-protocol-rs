use std::ffi::c_int;

pub const MAX_PACKETLEN: usize = 1400;

pub const FRAGMENT_SIZE: usize = MAX_PACKETLEN - 100;

pub const FRAGMENT_BIT: c_int = 1 << 31;

#![feature(generic_const_exprs)]
#![feature(rustc_attrs)]
#![feature(allocator_api)]

pub mod bitset;
pub mod bytetable;
pub mod pact;
pub mod trible;
pub mod ufoid;
pub mod fucid;
pub mod query;

#[cfg(test)]
mod tests {}

pub mod imtribledb;
pub mod imtribledb2;
pub mod imtribledb3;
pub mod imtribledb4;
pub mod query;

use super::trible::Trible;
use query::*;

pub trait TribleDB {
    fn with<'a, T>(&self, tribles: T) -> Self
    where
        T: Iterator<Item = &'a Trible> + Clone;
    /*
    fn empty(&self) -> Self;
    fn isEmpty(&self) -> bool;
    fn isEqual(&self, other: &Self) -> bool;å
    fn isSubsetOf(&self, other: &Self) -> bool;
    fn isProperSubsetOf(&self, other: &Self) -> bool;
    fn isIntersecting(&self, other: &Self) -> bool;
    fn union(&self, other: &Self) -> Self;
    fn subtract(&self, other: &Self) -> Self;
    fn difference(&self, other: &Self) -> Self;
    fn intersect(&self, other: &Self) -> Self;

    fn inner_constraint(
        &self,
        variable: Variable,
        e: bool,
        a: bool,
        v1: bool,
    ) -> Box<dyn Constraint>;
    fn trible_constraint(
        &self,
        e: Variable,
        a: Variable,
        v1: Variable,
        v2: Variable,
    ) -> Box<dyn Constraint>;
    */
}

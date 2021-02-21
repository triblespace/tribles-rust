pub type Segment = u128;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct E(pub Segment);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct A(pub Segment);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct V1(pub Segment);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct V2(pub Segment);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Trible {
    pub e: E,
    pub a: A,
    pub v1: V1,
    pub v2: V2,
}

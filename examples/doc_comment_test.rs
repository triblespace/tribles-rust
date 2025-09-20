use tribles::prelude::*;

pub mod testmod {
    #![allow(unused)]
    use super::*;
    use tribles::prelude::valueschemas::*;
    use tribles::prelude::*;

    attributes! {
        /// First doc line
        /// Second doc line
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: valueschemas::GenId;
    }
}

fn main() {}

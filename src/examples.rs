//! This module contains an example namespace for use in the documentation.
//! It is not intended to be used in practice.

use crate::prelude::blobschemas::*;
use crate::prelude::valueschemas::*;
use crate::NS;

NS! {
    /// The `literature` namespace contains attributes describing authors and their works.
    /// It is used to demonstrate the capabilities of the `tribles` crate.
    /// The namespace is not intended to be used in practice.
    pub namespace literature {
        /// The title of a work.
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
        /// A quote from a work.
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
        /// The author of a work.
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        /// The first name of an author.
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        /// The last name of an author.
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        /// The number of pages in the work.
        "FCCE870BECA333D059D5CD68C43B98F0" as page_count: R256;
    }
}

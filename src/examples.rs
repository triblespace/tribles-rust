//! This module contains an example namespace for use in the documentation.
//! It is not intended to be used in practice.

use crate::prelude::blobschemas::*;
use crate::prelude::valueschemas::*;
use crate::NS;

NS! {
    pub namespace literature {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
        "76AE5012877E09FF0EE0868FE9AA0343" as height: R256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

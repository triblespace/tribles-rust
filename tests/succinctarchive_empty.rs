#![cfg(feature = "succinct-archive")]

use tribles::blob::schemas::succinctarchive::OrderedUniverse;
use tribles::blob::schemas::succinctarchive::SuccinctArchive;
use tribles::prelude::*;

#[test]
fn build_from_empty_set() {
    let set = TribleSet::new();
    let archive: SuccinctArchive<OrderedUniverse> = (&set).into();
    assert_eq!(archive.domain.len(), 0);
    assert_eq!(archive.entity_count, 0);
    assert_eq!(archive.attribute_count, 0);
    assert_eq!(archive.value_count, 0);
}

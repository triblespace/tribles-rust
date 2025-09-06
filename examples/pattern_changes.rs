use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::Repository;
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


pub mod literature {
    #![allow(unused)]
    use crate::prelude::*;
    pub const author: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("8F180883F9FD5F787E9E0AF0DF5866B9"));
    pub const firstname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("0DBB530B37B966D137C50B943700EDB2"));
    pub const lastname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("6BAA463FD4EAF45F6A103DB9433E4545"));
    pub const title: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("A74AA63539354CDA47F387A4C3A8D54C"));
}

fn main() {
    // ANCHOR: pattern_changes_example
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws = repo.branch("main").expect("branch");

    // Commit initial data
    let shakespeare = ufoid();
    let hamlet = ufoid();
    let mut base = TribleSet::new();
    base += entity!(&shakespeare, { literature::firstname: "William", literature::lastname: "Shakespeare" });
    base += entity!(&hamlet, { literature::title: "Hamlet", literature::author: &shakespeare });
    ws.commit(base.clone(), None);
    let c1 = ws.head().unwrap();

    // Commit a new book
    let macbeth = ufoid();
    let mut change = TribleSet::new();
    change += entity!(&macbeth, { literature::title: "Macbeth", literature::author: &shakespeare });
    ws.commit(change.clone(), None);
    let c2 = ws.head().unwrap();

    // Compute updated state and delta between commits
    let base_state = ws.checkout(c1).expect("base");
    let updated = ws.checkout(c2).expect("updated");
    let delta = updated.difference(&base_state);

    // Find new titles by Shakespeare
    let results: Vec<_> = find!(
        (author: Value<_>, book: Value<_>, title: Value<_>),
        pattern_changes!(&updated, &delta, [
            { author @ literature::firstname: ("William"), literature::lastname: ("Shakespeare") },
            { book @ literature::author: author, literature::title: title }
        ])
    )
    .map(|(_, b, t)| (b, t))
    .collect();

    println!("{results:?}");
    // ANCHOR_END: pattern_changes_example
}

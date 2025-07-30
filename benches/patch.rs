use fake::faker::lorem::en::Sentence;
use fake::Fake;
use tribles::patch::{Entry, IdentityOrder, SingleSegmentation, PATCH};

fn main() {
    let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();

    for _ in 0..2_000_000 {
        let text: String = Sentence(3..8).fake();
        let mut key = [0u8; 64];
        let bytes = text.as_bytes();
        let len = bytes.len().min(64);
        key[..len].copy_from_slice(&bytes[..len]);
        let entry = Entry::new(&key);
        patch.insert(&entry);
    }

    #[cfg(debug_assertions)]
    {
        let avg = patch.debug_branch_fill();
        println!("Average fill: {:?}", avg);
    }
    #[cfg(not(debug_assertions))]
    {
        println!("Recompile with debug assertions to compute branch fill");
    }
}

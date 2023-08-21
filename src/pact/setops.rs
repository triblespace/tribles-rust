
use super::*;
/*
fn recursive_union<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
    at_depth: usize,
    unioned_nodes: &mut [Head<KEY_LEN, O, S>],
    prefix: &mut [u8; KEY_LEN],
) -> Head<KEY_LEN, O, S> {
    if 0 == unioned_nodes.len() {
        return Head::empty();
    }
    let first_node = &unioned_nodes[0];
    let first_node_hash = first_node.hash();

    let rest_nodes = &unioned_nodes[1..];
    
    if rest_nodes.iter().all(|node| node.hash() == first_node_hash) {
        return first_node.with_start(at_depth);
    }

    let branch_depth = unioned_nodes.iter()
                                           .map(|node| node.end_depth())
                                           .min().unwrap();

    for depth in at_depth..branch_depth {
        let first_peek = first_node.peek(depth);
        if !rest_nodes.iter().all(|node| node.peek(depth) == first_peek) {
            branch_with
        }
    }
    
    let mut depth = at_depth;

    loop {
        let mut union_childbits = ByteBitset::new_empty();

        for node in &mut *unioned_nodes {
            match node.peek(depth) {
                Peek::Fragment(byte) => {
                    union_childbits.set(byte);
                }
                Peek::Branch(children_set) if !children_set.is_empty() => {
                    union_childbits = union_childbits.union(children_set);
                    *node = node.child(depth, children_set.find_first_set().expect("child exists"));
                }
                Peek::Branch(_no_children) => {}
            }
        }
        match union_childbits.count() {
            0 => return Head::from(Empty::new()),
            1 => {
                prefix[depth] = union_childbits.find_first_set().expect("bitcount is one");
                if depth == KEY_LEN - 1 {
                    return Leaf::new(at_depth, &Arc::new(*prefix)).into();
                }
                depth += 1;
            }
            n => {
                let mut branch_node: Head<KEY_LEN, O, S> = match n {
                    1..=2 => unsafe {Head::new(HeadTag::Branch2, 0, Branch2::new(depth))},
                    2..=4 => unsafe {Head::new(HeadTag::Branch4, 0, Branch4::new(depth))},
                    5..=8 => unsafe {Head::new(HeadTag::Branch8, 0, Branch8::new(depth))},
                    9..=16 => unsafe {Head::new(HeadTag::Branch16, 0, Branch16::new(depth))},
                    17..=32 => unsafe {Head::new(HeadTag::Branch32, 0, Branch32::new(depth))},
                    33..=64 => unsafe {Head::new(HeadTag::Branch64, 0, Branch64::new(depth))},
                    65..=128 => unsafe {Head::new(HeadTag::Branch128, 0, Branch128::new(depth))},
                    129..=256 => unsafe {Head::new(HeadTag::Branch256, 0, Branch256::new(depth))},
                    _ => panic!("bad child count"),
                };

                while let Some(byte) = union_childbits.drain_next_ascending() {
                    let mut children = Vec::new();
                    for node in &mut *unioned_nodes {
                        if(node.end_depth() == depth) {
                            children.push(node.clone());
                        } else {
                            if let Some(child) = node.branch(byte) {
                                children.push(child);
                            }
                        }
                    }

                    let union_node = if depth == KEY_LEN - 1 {
                        Leaf::new(depth, &Arc::new(*prefix)).into()
                    } else {
                        recursive_union(depth, &mut children[..])
                    };

                    let mut displaced = branch_node.insert(union_node);
                    while None != displaced.key() {
                        branch_node = branch_node.grow();
                        displaced = branch_node.reinsert(displaced);
                    }
                }

                return Head::from(branch_node);
            }
        }
    }
}

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>
    PACT<KEY_LEN, O, S>
{
    pub fn union<I>(trees: I) -> PACT<KEY_LEN, O, S>
    where
        I: IntoIterator<Item = PACT<KEY_LEN, O, S>>,
    {
        let mut children = Vec::new();

        for tree in trees {
            children.push(tree.root)
        }

        let mut prefix = [0u8; KEY_LEN];

        return PACT {
            root: recursive_union(0, &mut children[..], &mut prefix),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    proptest! {
        /*
        #[test]
        fn tree_union(entriess in prop::collection::vec(prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1), 2)) {
            let mut set = HashSet::new();

            let mut trees = Vec::new();
            for entries in entriess {
                let mut tree = PACT::<64, IdentityOrder>::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    tree.put(&Arc::new(key));
                    set.insert(key);
                }
                trees.push(tree);
            }
            let union_tree = PACT::union(trees);
            let union_set = HashSet::from_iter(union_tree.cursor().into_iter());
            prop_assert_eq!(set, union_set);
        }
        */
    }
}
*/
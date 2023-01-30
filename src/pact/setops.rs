use super::*;

fn recursiveUnion<const KEY_LEN: usize>(
    at_depth: usize,
    unioned_nodes: &[Head<KEY_LEN>],
    prefix: &mut [u8; KEY_LEN],
) -> Head<KEY_LEN> {
    if 0 == dbg!(unioned_nodes.len()) {
        return Head::from(Empty::new());
    }
    let first_node = &unioned_nodes[0];
    let first_node_hash = first_node.hash(prefix);

    let mut all_equal = true;
    for other_node in &unioned_nodes[1..] {
        if first_node_hash != other_node.hash(prefix) {
            all_equal = false;
            break;
        }
    }
    if all_equal {
        return first_node.clone();
    }

    let mut depth = at_depth;
    /*
    outer: while (depth < max_depth):(depth += 1) {
        const first_peek = first_node.peek(depth).?;
        for (other_nodes) |other_node| {
            const other_peek = other_node.peek(depth).?;
            if (first_peek != other_peek) break :outer;
        }
        prefix[depth] = first_peek;
    }
    */
    loop {
        dbg!(at_depth, depth);
        let mut union_childbits = ByteBitset::new_empty();

        for node in unioned_nodes {
            union_childbits = union_childbits.union(node.propose(depth));
        }

        match union_childbits.count() {
            0 => return Head::from(Empty::new()),
            1 => {
                prefix[depth] = union_childbits.find_first_set().expect("bitcount is one");
                depth += 1;
            }
            n => {
                let mut branch_node: Head<KEY_LEN> = match n {
                    1..=4 => Branch4::new(at_depth, depth, prefix).into(),
                    5..=8 => Branch8::new(at_depth, depth, prefix).into(),
                    9..=16 => Branch16::new(at_depth, depth, prefix).into(),
                    17..=32 => Branch32::new(at_depth, depth, prefix).into(),
                    33..=64 => Branch64::new(at_depth, depth, prefix).into(),
                    65..=128 => Branch128::new(at_depth, depth, prefix).into(),
                    129..=256 => Branch256::new(at_depth, depth, prefix).into(),
                    _ => panic!("bad child count"),
                };

                while let Some(byte) = union_childbits.drain_next_ascending() {
                    prefix[depth] = byte;

                    let mut children = Vec::new();
                    for node in unioned_nodes {
                        //TODO filter empty
                        children.push(node.get(depth, byte));
                    }

                    let union_node = recursiveUnion(depth + 1, &children[..], prefix);

                    let mut displaced = branch_node.insert(prefix, union_node);
                    while None != displaced.key() {
                        branch_node = branch_node.grow();
                        displaced = branch_node.reinsert(displaced);
                    }
                }

                return branch_node.wrap_path(at_depth, prefix);
            }
        }
    }
}

impl<const KEY_LEN: usize> PACT<KEY_LEN> {
    pub fn union<I>(trees: I) -> PACT<KEY_LEN>
    where
        I: IntoIterator<Item = PACT<KEY_LEN>>,
    {
        let mut children = Vec::new();

        for tree in trees {
            children.push(tree.root)
        }

        let mut prefix = [0u8; KEY_LEN];

        return PACT {
            root: recursiveUnion(0, &children[..], &mut prefix),
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
        #[test]
        fn tree_union(entriess in prop::collection::vec(prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1), 2)) {
            let mut set = HashSet::new();

            let mut trees = Vec::new();
            for entries in entriess {
                let mut tree = PACT::<64>::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    tree.put(key);
                    set.insert(key);
                }
                trees.push(tree);
            }
            let union_tree = PACT::union(trees);
            let union_set = HashSet::from_iter(union_tree.cursor().into_iter());
            prop_assert_eq!(set, union_set);
        }
    }
}

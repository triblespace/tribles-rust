use super::*;

fn recursive_union<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
    at_depth: usize,
    unioned_nodes: &mut [Head<KEY_LEN, O, S>],
    prefix: &mut [u8; KEY_LEN],
) -> Head<KEY_LEN, O, S> {
    if 0 == unioned_nodes.len() {
        return Head::from(Empty::new());
    }
    let first_node = &unioned_nodes[0];
    let first_node_hash = first_node.hash();

    let mut all_equal = true;
    for other_node in &unioned_nodes[1..] {
        if first_node_hash != other_node.hash() {
            all_equal = false;
            break;
        }
    }
    if all_equal {
        return first_node.with_start(at_depth);
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
                    for node in &mut *unioned_nodes {
                        //TODO filter empty
                        children.push(node.child(depth, byte));
                    }

                    let union_node = if depth == KEY_LEN - 1 {
                        Leaf::new(depth, &Arc::new(*prefix)).into()
                    } else {
                        recursive_union(depth, &mut children[..], prefix)
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

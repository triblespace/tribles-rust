use super::*;

fn recursive_union<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
    at_depth: usize,
    unioned_nodes: Vec<&Head<KEY_LEN, O, S>>,
) -> Head<KEY_LEN, O, S> {
    if 0 == unioned_nodes.len() {
        return Head::empty();
    }

    if 1 == unioned_nodes.len() {
        return unioned_nodes[0].with_start(at_depth);
    }

    let first_node = unioned_nodes[0];
    let first_node_hash = first_node.hash();

    let rest_nodes = &unioned_nodes[1..];

    if rest_nodes.iter().all(|node| node.hash() == first_node_hash) {
        return first_node.with_start(at_depth);
    }

    let branch_depth = unioned_nodes
        .iter()
        .map(|node| node.end_depth())
        .min()
        .unwrap();

    for depth in at_depth..branch_depth {
        let mut byte_keys = ByteBitset::new_empty();
        unioned_nodes
            .iter()
            .for_each(|node| byte_keys.set(node.peek(depth)));
        if byte_keys.count() != 1 {
            let mut branch = branch_for_size(byte_keys.count() as usize, depth);
            for byte_key in byte_keys {
                let byte_nodes: Vec<_> = unioned_nodes
                    .iter()
                    .copied()
                    .filter(|&node| node.peek(depth) == byte_key)
                    .collect();
                let byte_union = recursive_union(depth, byte_nodes);
                branch.insert(byte_union);
            }
            return branch.with_start(at_depth);
        }
    }

    let mut byte_keys = ByteBitset::new_empty();
    unioned_nodes
        .iter()
        .for_each(|node| byte_keys = byte_keys.union(node.keys(branch_depth)));

    let mut branch = branch_for_size(byte_keys.count() as usize, branch_depth);
    for byte_key in byte_keys {
        let byte_nodes: Vec<_> = unioned_nodes
            .iter()
            .filter_map(|&node| node.child(branch_depth, byte_key))
            .collect();
        let byte_union = recursive_union(branch_depth, byte_nodes);
        branch.insert(byte_union);
    }
    return branch.with_start(at_depth);
}

impl<
        const KEY_LEN: usize,
        O: KeyOrdering<KEY_LEN> + 'static,
        S: KeySegmentation<KEY_LEN> + 'static,
    > PATCH<KEY_LEN, O, S>
{
    pub fn union<'a, I>(trees: I) -> PATCH<KEY_LEN, O, S>
    where
        I: IntoIterator<Item = &'a PATCH<KEY_LEN, O, S>>,
    {
        let mut children = Vec::new();

        for tree in trees {
            children.push(&tree.root)
        }

        return PATCH {
            root: recursive_union(0, children),
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
        fn tree_union(entriess in prop::collection::vec(prop::collection::vec(prop::collection::vec(0u8..=255, 64), 100), 10)) {
            let mut set = HashSet::new();

            let mut trees = Vec::new();
            for entries in entriess {
                let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    let entry = Entry::new(&key);
                    tree.put(&entry);
                    set.insert(key);
                }
                trees.push(tree);
            }
            let union_tree = PATCH::union(trees.iter());

            let mut set_vec = Vec::from_iter(set.into_iter());
            let mut tree_vec = union_tree.infixes([0; 64], 0, 63, |x| x);

            set_vec.sort();
            tree_vec.sort();

            prop_assert_eq!(set_vec, tree_vec);
        }
    }

    #[test]
    fn tree_union_r0() {
        let entriess = [
            [
                [
                    2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ],
            ],
            [
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            ],
        ];
        let mut set = HashSet::new();

        let mut trees = Vec::new();
        for entries in entriess {
            let mut tree = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
            for entry in entries {
                let mut key = [0; 64];
                key.iter_mut().set_from(entry.iter().cloned());
                let entry = Entry::new(&key);
                tree.put(&entry);
                set.insert(key);
            }
            trees.push(tree);
        }
        let union_tree = PATCH::union(trees.iter());

        let mut set_vec = Vec::from_iter(set.into_iter());
        let mut tree_vec = union_tree.infixes([0; 64], 0, 63, |x| x);

        set_vec.sort();
        tree_vec.sort();

        assert_eq!(set_vec, tree_vec);
    }
}

use super::*;
/*
fn recursive_union<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN>, S: KeySegmentation<KEY_LEN>>(
    at_depth: usize,
    unioned_nodes: Vec<&Head<KEY_LEN, O, S>>
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
        let mut byte_keys = ByteBitset::new_empty();
        unioned_nodes.iter().for_each(|node| byte_keys.set(node.peek(depth)));
        if byte_keys.count() != 1 {
            let branch = branch_for_size(byte_keys.count());
            for byte_key in byte_keys {
                let byte_nodes: Vec<_> = unioned_nodes.iter().copied().filter(|&node| node.peek(depth) == byte_key).collect();
                let byte_union = recursive_union(depth, byte_nodes);
                branch.insert(byte_union);
            }
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
                if depth == KEY_LEN - 1 {
                    return Leaf::new(at_depth).into();
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
                        Leaf::new(depth).into()
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

impl<const KEY_LEN: usize, O: KeyOrdering<KEY_LEN> + 'static, S: KeySegmentation<KEY_LEN> + 'static>
    PACT<KEY_LEN, O, S>
{
    pub fn union<'a, I>(trees: I) -> PACT<KEY_LEN, O, S>
    where
        I: IntoIterator<Item = &'a PACT<KEY_LEN, O, S>>,
    {
        let mut children = Vec::new();

        for tree in trees {
            children.push(&tree.root)
        }

        return PACT {
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
        fn tree_union(entriess in prop::collection::vec(prop::collection::vec(prop::collection::vec(0u8..=255, 64), 1), 2)) {
            let mut set = HashSet::new();

            let mut trees = Vec::new();
            for entries in entriess {
                let mut tree = PACT::<64, IdentityOrder, SingleSegmentation>::new();
                for entry in entries {
                    let mut key = [0; 64];
                    key.iter_mut().set_from(entry.iter().cloned());
                    let entry = Entry::new(&key);
                    tree.put(&entry);
                    set.insert(key);
                }
                trees.push(tree);
            }
            let union_tree = PACT::union(trees.iter());

            let mut set_vec = Vec::from_iter(set.into_iter());
            let mut tree_vec = union_tree.infixes([0; 64], 0, 63, |x| x);

            set_vec.sort();
            tree_vec.sort();

            prop_assert_eq!(set_vec, tree_vec);
        }
    }
}
*/

use std::fmt;

/// A fixed size bitset over the possible values of a byte.
#[repr(transparent)]
pub struct ByteBitset {
    bits: [u64; 4]
}

impl ByteBitset {
    /// Create a new empty set.
    pub const fn new_empty() -> Self {
        ByteBitset {
            bits: [0; 4]
        }
    }
    /// Create a new set with every value of the domain set.
    pub const fn new_full() -> Self {
        ByteBitset {
            bits: [!0; 4]
        }
    }
    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        (self.bits[0] == 0) &&
        (self.bits[1] == 0) &&
        (self.bits[2] == 0) &&
        (self.bits[3] == 0)
    }
    /// Count the number of elements in the set.
    pub fn count(&self) -> u32 {
        self.bits[0].count_ones() +
        self.bits[1].count_ones() +
        self.bits[2].count_ones() +
        self.bits[3].count_ones()
    }
    /// Set the given value in the set.
    pub fn set(&mut self, index:u8) {
        self.bits[usize::from(index >> 6)] |= 1 << (index & 0b111111);
    }
    /// Remove the given value from the set.
    pub fn unset(&mut self, index:u8) {
        self.bits[usize::from(index >> 6)] &= !(1 << (index & 0b111111));
    }
    /// Sets or removes the given element into or from the set
    /// depending on the passed value.
    pub fn set_value(&mut self, index: u8, value: bool) {
        if value {
            self.set(index);
        } else {
            self.unset(index);
        }
    }
    /// Include every value in the domain in the set.
    pub fn set_all(&mut self) {
        self.bits = [!0; 4];
    }
    /// Remove all values from the set.
    pub fn unset_all(&mut self) {
        self.bits = [0; 4];
    }
    /// Check if the given value is in the set.
    pub fn is_set(&self, index: u8) -> bool {
        0 != (self.bits[usize::from(index >> 6)] & (1 << (index & 0b111111)))
    }
    /// Finds the index of the first set bit.
    /// If no bits are set, returns `None`.
    pub fn find_first_set(&self) -> Option<u8> {
        if self.bits[0] != 0 {return Some(self.bits[0].trailing_zeros() as u8);}
        if self.bits[1] != 0 {return Some((1 << 6) + (self.bits[1].trailing_zeros() as u8));}
        if self.bits[2] != 0 {return Some((2 << 6) + (self.bits[2].trailing_zeros() as u8));}
        if self.bits[3] != 0 {return Some((3 << 6) + (self.bits[3].trailing_zeros() as u8));}
        return None;
    }
    /// Finds the index of the last set bit.
    /// If no bits are set, returns `None`.
    pub fn find_last_set(&self) -> Option<u8> {
        if self.bits[3] != 0 {return Some((3 << 6) + (63 - (self.bits[3].leading_zeros() as u8)));}
        if self.bits[2] != 0 {return Some((2 << 6) + (63 - (self.bits[2].leading_zeros() as u8)));}
        if self.bits[1] != 0 {return Some((1 << 6) + (63 - (self.bits[1].leading_zeros() as u8)));}
        if self.bits[0] != 0 {return Some(63 - (self.bits[0].leading_zeros() as u8));}
        return None;
    }
    /// Returns the index of the next set bit
    /// in the bit set, in ascending order, while unseting it.
    pub fn drain_next_ascending(&mut self) -> Option<u8> {
        if let Some(next_index) = self.find_first_set() {
            self.unset(next_index);
            Some(next_index)
        } else {
            None
        }
    }
    /// Returns the index of the next set bit
    /// in the bit set, in descending order, while unseting it.
    pub fn drain_next_descending(&mut self) -> Option<u8> {
        if let Some(next_index) = self.find_last_set() {
            self.unset(next_index);
            Some(next_index)
        } else {
            None
        }
    }
    /// Checks if the set is a superset of the passed set.
    pub fn is_superset_of(&self, other: &Self) -> bool {
        ((self.bits[0] & other.bits[0]) ^ other.bits[0]) == 0 &&
        ((self.bits[1] & other.bits[1]) ^ other.bits[1]) == 0 &&
        ((self.bits[2] & other.bits[2]) ^ other.bits[2]) == 0 &&
        ((self.bits[3] & other.bits[3]) ^ other.bits[3]) == 0
    }
    /// Checks if the set is a subset of the passed set.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        ((self.bits[0] & other.bits[0]) ^ self.bits[0]) == 0 &&
        ((self.bits[1] & other.bits[1]) ^ self.bits[1]) == 0 &&
        ((self.bits[2] & other.bits[2]) ^ self.bits[2]) == 0 &&
        ((self.bits[3] & other.bits[3]) ^ self.bits[3]) == 0
    }
    /// Store the set intersection between the two given sets in the set.
    pub fn set_intersect(&mut self, left: &Self, right: &Self) {
        self.bits[0] = left.bits[0] & right.bits[0];
        self.bits[1] = left.bits[1] & right.bits[1];
        self.bits[2] = left.bits[2] & right.bits[2];
        self.bits[3] = left.bits[3] & right.bits[3];
    }
    /// Store the set union between the two given sets in the set.
    pub fn set_union(&mut self, left: &Self, right: &Self) {
        self.bits[0] = left.bits[0] | right.bits[0];
        self.bits[1] = left.bits[1] | right.bits[1];
        self.bits[2] = left.bits[2] | right.bits[2];
        self.bits[3] = left.bits[3] | right.bits[3];
    }
    /// Store the set subtraction between the two given sets in the set.
    pub fn set_subtract(&mut self, left: &Self, right: &Self) {
        self.bits[1] = left.bits[1] & !right.bits[1];
        self.bits[2] = left.bits[2] & !right.bits[2];
        self.bits[0] = left.bits[0] & !right.bits[0];
        self.bits[3] = left.bits[3] & !right.bits[3];
    }
    /// Store the set difference between the two given sets in the set.
    pub fn set_difference(&mut self, left: &Self, right: &Self) {
        self.bits[0] = left.bits[0] ^ right.bits[0];
        self.bits[1] = left.bits[1] ^ right.bits[1];
        self.bits[2] = left.bits[2] ^ right.bits[2];
        self.bits[3] = left.bits[3] ^ right.bits[3];
    }
    /// Perform a set complement, removing every element that was in the set
    /// and inserting every element from the domain that wasn't in the set.
    pub fn set_complement(&mut self, input: &Self) {
        self.bits[0] = !input.bits[0];
        self.bits[1] = !input.bits[1];
        self.bits[2] = !input.bits[2];
        self.bits[3] = !input.bits[3];
    }
    /// Remove all elements from the set except the one passed.
    /// Equal to an intersection with a set containing only the passed element.
    pub fn keep_single(&mut self, index: u8) {
        let had_bit = self.is_set(index);
        self.unset_all();
        if had_bit {
            self.set(index);
        }
    }

    /// Similar to keep_single, except that only values in the
    /// specified range are kept. Both range ends are inclusive.
    pub fn keep_range(&mut self, from_index: u8, to_index: u8) {
        let from_word_index = (from_index >> 6) as usize;
        let to_word_index = (to_index >> 6) as usize;

        for word_index in 0..from_word_index {
            self.bits[word_index] = 0;
        }

        self.bits[from_word_index] &= !0 << (from_index & 0b111111);
        self.bits[to_word_index] &= !(!1 << ((to_index & 0b111111)));

        for word_index in (to_word_index + 1)..4 {
            self.bits[word_index] = 0;
        }
    }
}

impl fmt::Debug for ByteBitset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteBitset: {{")?;
        let mut needs_comma = false;
        for byte in 0u8..255 {
            if self.is_set(byte) {
                if needs_comma {
                    write!(f, ", ",)?;
                }
                write!(f, "{}", byte)?;
                needs_comma = true;
            }
        }
        write!(f, "}}\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn new_empty_is_empty() {
        let set = ByteBitset::new_empty();
        assert!(set.is_empty());
    }
    #[test]
    fn new_full_is_not_empty() {
        let set = ByteBitset::new_full();
        assert!(!set.is_empty());
    }
    #[test]
    fn after_set_is_not_empty() {
        let mut set = ByteBitset::new_empty();
        set.set(5);
        assert!(!set.is_empty());
    }
    #[test]
    fn after_set_unset_is_empty() {
        let mut set = ByteBitset::new_empty();
        set.set(5);
        set.unset(5);
        assert!(set.is_empty());
    }
    #[test]
    fn after_set_is_set() {
        let mut set = ByteBitset::new_empty();
        set.set(5);
        assert!(set.is_set(5));
    }
    #[test]
    fn after_unset_is_not_set() {
        let mut set = ByteBitset::new_full();
        set.unset(5);
        assert!(!set.is_set(5));
    }
    #[test]
    fn find_first_set_none() {
        let set = ByteBitset::new_empty();
        assert_eq!(None, set.find_first_set());
    }
    #[test]
    fn find_last_set_none() {
        let set = ByteBitset::new_empty();
        assert_eq!(None, set.find_last_set());
    }
    #[test]
    fn superset_full_of_empty() {
        let empty = ByteBitset::new_empty();
        let full = ByteBitset::new_full();
        assert!(full.is_superset_of(&empty));
    }
    #[test]
    fn superset_not_empty_of_full() {
        let empty = ByteBitset::new_empty();
        let full = ByteBitset::new_full();
        assert!(!empty.is_superset_of(&full));
    }
    #[test]
    fn subset_empty_of_full() {
        let empty = ByteBitset::new_empty();
        let full = ByteBitset::new_full();
        assert!(empty.is_subset_of(&full));
    }
    #[test]
    fn subset_not_full_of_empty() {
        let empty = ByteBitset::new_empty();
        let full = ByteBitset::new_full();
        assert!(!full.is_subset_of(&empty));
    }
    proptest! {
        #[test]
        fn find_first_set(n in 0u8..255) {
            let mut set = ByteBitset::new_empty();
            set.set(n);
            prop_assert_eq!(Some(n), set.find_first_set());
        }
        #[test]
        fn find_last_set(n in 0u8..255) {
            let mut set = ByteBitset::new_empty();
            set.set(n);
            prop_assert_eq!(Some(n), set.find_last_set());
        }
        #[test]
        fn drain_ascending_drains(n in 0u8..255) {
            let mut set = ByteBitset::new_empty();
            set.set(n);
            prop_assert_eq!(Some(n), set.drain_next_ascending());
            prop_assert!(!set.is_set(n));
        }
        #[test]
        fn drain_descending_drains(n in 0u8..255) {
            let mut set = ByteBitset::new_empty();
            set.set(n);
            prop_assert_eq!(Some(n), set.drain_next_descending());
            prop_assert!(!set.is_set(n));
        }
        #[test]
        fn intersect(n in 0u8..255, m in 0u8..255) {
            let mut out = ByteBitset::new_empty();
            let mut left = ByteBitset::new_empty();
            let mut right = ByteBitset::new_empty();
            left.set(n);
            right.set(m);
            out.set_intersect(&left, &right);
            prop_assert_eq!(n == m, out.is_set(n));
        }
        #[test]
        fn union(n in 0u8..255, m in 0u8..255) {
            let mut out = ByteBitset::new_empty();
            let mut left = ByteBitset::new_empty();
            let mut right = ByteBitset::new_empty();
            left.set(n);
            right.set(m);
            out.set_union(&left, &right);
            prop_assert!(out.is_set(n));
            prop_assert!(out.is_set(m));
        }
        #[test]
        fn subtract(n in 0u8..255, m in 0u8..255) {
            let mut out = ByteBitset::new_empty();
            let mut left = ByteBitset::new_empty();
            let mut right = ByteBitset::new_empty();
            left.set(n);
            right.set(m);
            out.set_subtract(&left, &right);
            prop_assert_eq!(n != m, out.is_set(n));
        }
        #[test]
        fn difference(n in 0u8..255, m in 0u8..255) {
            let mut out = ByteBitset::new_empty();
            let mut left = ByteBitset::new_empty();
            let mut right = ByteBitset::new_empty();
            left.set(n);
            right.set(m);
            out.set_difference(&left, &right);
            prop_assert_eq!(n != m, out.is_set(n));
            prop_assert_eq!(n != m, out.is_set(m));
        }
        #[test]
        fn complement(n in 0u8..255, m in 0u8..255) {
            let mut out = ByteBitset::new_empty();
            let mut input = ByteBitset::new_empty();
            input.set(n);
            out.set_complement(&input);
            prop_assert!(!out.is_set(n));
            if n != m {
                prop_assert!(out.is_set(m));
            }
        }
        #[test]
        fn keep_single(n in 0u8..255, m in 0u8..255) {
            let mut set = ByteBitset::new_full();
            set.keep_single(n);
            prop_assert!(set.is_set(n));
            if n != m {
                prop_assert!(!set.is_set(m));
            }
        }
        #[test]
        fn keep_range(from in 0u8..255, to in 0u8..255) {
            let mut set = ByteBitset::new_full();
            set.keep_range(from, to);
            
            for n in 0u8..255 {
                prop_assert_eq!(from <= n && n <= to, set.is_set(n));
            }
        }
    }
}

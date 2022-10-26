struct ByteBitset {
    bits: [u64; 4]
}

impl ByteBitset {
    pub fn new_empty() -> Self {
        ByteBitset {
            bits: [0; 4]
        }
    }
    pub fn new_full() -> Self {
        ByteBitset {
            bits: [!0; 4]
        }
    }
    pub fn is_empty(&self) -> bool {
        (self.bits[0] == 0) &&
        (self.bits[1] == 0) &&
        (self.bits[2] == 0) &&
        (self.bits[3] == 0)
    }
    pub fn count(&self) -> u32 {
        self.bits[0].count_ones() +
        self.bits[1].count_ones() +
        self.bits[2].count_ones() +
        self.bits[3].count_ones()
    }
    pub fn set(&mut self, index:u8) {
        self.bits[usize::from(index >> 6)] |= 1 << (index & 0b111111);
    }
    pub fn unset(&mut self, index:u8) {
        self.bits[usize::from(index >> 6)] &= !(1 << (index & 0b111111));
    }
    pub fn set_value(&mut self, index: u8, value: bool) {
        if value {
            self.set(index);
        } else {
            self.unset(index);
        }
    }
    pub fn set_all(&mut self) {
        self.bits = [!0; 4];
    }
    pub fn unset_all(&mut self) {
        self.bits = [0; 4];
    }
    pub fn is_set(&self, index: u8) -> bool {
        0 != (self.bits[usize::from(index >> 6)] & (1 << (index & 0b111111)))
    }
    pub fn find_first_set(&self) -> Option<u8> {
        if self.bits[0] != 0 {return Some(self.bits[0].trailing_zeros() as u8);}
        if self.bits[1] != 0 {return Some((1 << 6) + (self.bits[1].trailing_zeros() as u8));}
        if self.bits[2] != 0 {return Some((2 << 6) + (self.bits[2].trailing_zeros() as u8));}
        if self.bits[3] != 0 {return Some((3 << 6) + (self.bits[3].trailing_zeros() as u8));}
        return None;
    }
    pub fn find_last_set(&self) -> Option<u8> {
        if self.bits[3] != 0 {return Some((3 << 6) + (63 - (self.bits[3].leading_zeros() as u8)));}
        if self.bits[2] != 0 {return Some((2 << 6) + (63 - (self.bits[2].leading_zeros() as u8)));}
        if self.bits[1] != 0 {return Some((1 << 6) + (63 - (self.bits[1].leading_zeros() as u8)));}
        if self.bits[0] != 0 {return Some(63 - (self.bits[0].leading_zeros() as u8));}
        return None;
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
        let mut set = ByteBitset::new_empty();
        assert_eq!(None, set.find_first_set());
    }
    #[test]
    fn find_last_set_none() {
        let mut set = ByteBitset::new_empty();
        assert_eq!(None, set.find_last_set());
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
    }
}

/*
    /// Returns the index of the next set bit
    /// in the bit set, in ascending order, while unseting it.
    pub fn drainNextAscending(self: *ByteBitset) ?u8 {
        if (self.isEmpty()) return null;
        const next_index = self.findFirstSet() orelse unreachable;
        self.unset(next_index);
        return next_index;
    }

    /// Returns the index of the next set bit
    /// in the bit set, in descending order, while unseting it.
    pub fn drainNextDescending(self: *ByteBitset) ?u8 {
        if (self.isEmpty()) return null;
        const next_index = self.findLastSet() orelse unreachable;
        self.unset(next_index);
        return next_index;
    }

    pub fn isSupersetOf(left: *ByteBitset, right: *ByteBitset) bool {
        return ((left.bits[0] & right.bits[0]) ^ right.bits[0]) == 0 and
            ((left.bits[1] & right.bits[1]) ^ right.bits[1]) == 0 and
            ((left.bits[2] & right.bits[2]) ^ right.bits[2]) == 0 and
            ((left.bits[3] & right.bits[3]) ^ right.bits[3]) == 0;
    }

    pub fn isSubsetOf(left: *ByteBitset, right: *ByteBitset) bool {
        return ((left.bits[0] & right.bits[0]) ^ left.bits[0]) == 0 and
            ((left.bits[1] & right.bits[1]) ^ left.bits[1]) == 0 and
            ((left.bits[2] & right.bits[2]) ^ left.bits[2]) == 0 and
            ((left.bits[3] & right.bits[3]) ^ left.bits[3]) == 0;
    }

    pub fn setIntersect(self: *ByteBitset, left: *ByteBitset, right: *ByteBitset) void {
        self.bits[0] = left.bits[0] & right.bits[0];
        self.bits[1] = left.bits[1] & right.bits[1];
        self.bits[2] = left.bits[2] & right.bits[2];
        self.bits[3] = left.bits[3] & right.bits[3];
    }

    pub fn setUnion(self: *ByteBitset, left: *ByteBitset, right: *ByteBitset) void {
        self.bits[0] = left.bits[0] | right.bits[0];
        self.bits[1] = left.bits[1] | right.bits[1];
        self.bits[2] = left.bits[2] | right.bits[2];
        self.bits[3] = left.bits[3] | right.bits[3];
    }

    pub fn setSubtract(self: *ByteBitset, left: *ByteBitset, right: *ByteBitset) void {
        self.bits[0] = left.bits[0] & ~right.bits[0];
        self.bits[1] = left.bits[1] & ~right.bits[1];
        self.bits[2] = left.bits[2] & ~right.bits[2];
        self.bits[3] = left.bits[3] & ~right.bits[3];
    }

    pub fn setDiff(self: *ByteBitset, left: *ByteBitset, right: *ByteBitset) void {
        self.bits[0] = left.bits[0] ^ right.bits[0];
        self.bits[1] = left.bits[1] ^ right.bits[1];
        self.bits[2] = left.bits[2] ^ right.bits[2];
        self.bits[3] = left.bits[3] ^ right.bits[3];
    }

    pub fn bitComplement(self: *ByteBitset, in: *ByteBitset) void {
        self.bits[0] = ~in.bits[0];
        self.bits[1] = ~in.bits[1];
        self.bits[2] = ~in.bits[2];
        self.bits[3] = ~in.bits[3];
    }

    pub fn singleIntersect(self: *ByteBitset, index: u8) void {
        const had_bit = self.isSet(index);
        self.unsetAll();
        if (had_bit) {
            self.set(index);
        }
    }

    pub fn intersectRange(self: *ByteBitset, from_index: u8, to_index: u8) void {
        const from_word_index = from_index >> 6;
        const to_word_index = to_index >> 6;

        var word_index = 0;
        while (word_index < from_word_index) : (word_index += 1) {
            self.bits[word_index] = 0;
        }

        self.bits[from_word_index] &= (~0) >> @truncate(u8, from_index);
        self.bits[to_word_index] &= ~(~(1 << 63) >> @truncate(u6, to_index));

        word_index = to_word_index;
        while (word_index < 4) : (word_index += 1) {
            self.bits[word_index] = 0;
        }
    }
};

*/
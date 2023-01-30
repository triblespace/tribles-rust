/*
pub struct TribleSet {
    eav: EAVIndex,
    aev: AEVIndex,
    ave: AVEIndex,
}

impl TribleSet {
    pub fn new() -> TribleSet {
        TribleSet{
            eav: EAVIndex.init(),
            aev: AEVIndex.init(),
            ave: AVEIndex.init(),
        }
    }

    pub fn len(&self) u64 {
        return self.eav.len();
    }

    pub fn put(self: *TribleSet, trible: *const Trible) allocError!void {
        try self.eav.put(trible.ordered(.eav));
        try self.aev.put(trible.ordered(.aev));
        try self.ave.put(trible.ordered(.ave));
    }
};

*/
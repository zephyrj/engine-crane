use std::cmp::{max, Ordering};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct RatioEntry {
    pub idx: usize,
    pub name: String,
    pub ratio: f64
}

impl RatioEntry {
    fn new(idx: usize, name: String, ratio: f64) -> RatioEntry {
        RatioEntry {idx, name, ratio}
    }

    pub fn total_cmp(&self, other: &RatioEntry) -> Ordering {
        self.ratio.total_cmp(&other.ratio)
    }
}

impl Eq for RatioEntry {}

impl PartialEq<Self> for RatioEntry {
    fn eq(&self, other: &Self) -> bool {
        self.ratio.eq(&other.ratio)
    }
}

impl PartialOrd<Self> for RatioEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.ratio.partial_cmp(&other.ratio)
    }
}

impl Ord for RatioEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_cmp(other)
    }
}

#[derive(Debug, Default)]
pub(crate) struct RatioSet {
    entries: BTreeMap<usize, RatioEntry>,
    max_name_length: usize,
    next_idx: usize,
    default_idx: Option<usize>
}

impl RatioSet {
    pub fn new() -> RatioSet {
        RatioSet {
            entries: BTreeMap::new(),
            max_name_length: 0,
            next_idx: 0,
            default_idx: None
        }
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn max_name_len(&self) -> usize {
        self.max_name_length
    }

    pub fn entries(&self) -> Vec<&RatioEntry> {
        let mut v = Vec::from_iter(self.entries.values());
        v.sort();
        v
    }

    pub fn mut_entries(&mut self) -> Vec<&mut RatioEntry> {
        let mut v = Vec::from_iter(self.entries.values_mut());
        v.sort();
        v
    }

    pub fn insert(&mut self, ratio_name: String, ratio: f64) -> usize {
        self.max_name_length = max(self.max_name_length , ratio_name.len());
        let idx = self.next_idx;
        self.next_idx += 1;
        self.entries.insert(idx, RatioEntry::new(idx, ratio_name, ratio));
        idx
    }

    pub fn remove(&mut self, idx: usize) -> bool {
        return match self.entries.remove(&idx) {
            None => { false }
            Some(removed) => {
                if let Some(default_idx) = self.default_idx {
                    if idx == default_idx {
                        self.default_idx = None;
                    }
                }
                if removed.name.len() == self.max_name_length {
                    self.max_name_length = 0;
                    for entry in self.entries.values() {
                        self.max_name_length = max(self.max_name_length, entry.name.len());
                    }
                }
                true
            }
        }
    }

    pub fn remove_entry(&mut self, entry: &RatioEntry) -> bool {
        self.remove(entry.idx)
    }

    pub fn update_ratio_name(&mut self, idx: usize, new_name: String) {
        match self.entries.get_mut(&idx) {
            None => {}
            Some(entry) => { entry.name = new_name }
        }
    }

    pub fn update_ratio_value(&mut self, idx: usize, new_value: f64) {
        match self.entries.get_mut(&idx) {
            None => {}
            Some(entry) => { entry.ratio = new_value }
        }
    }

    pub fn default_idx(&self) -> Option<usize> {
        self.default_idx
    }

    pub fn default_ratio(&self) -> Option<&RatioEntry> {
        match self.default_idx {
            None => None,
            Some(idx) => self.entries.get(&idx)
        }
    }

    pub fn set_default(&mut self, idx: usize) -> Result<(), String> {
        if !self.entries.contains_key(&idx) {
            return Err(format!("Index {} doesn't exist", idx));
        }
        self.default_idx = Some(idx);
        Ok(())
    }
}

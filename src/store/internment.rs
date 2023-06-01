use std::{collections::BTreeSet};

use super::RcStr;

pub trait Internable {
    fn intern(self, i: &mut Interner) -> Self;
}

impl Internable for RcStr {
    fn intern(self, i: &mut Interner) -> Self {
        i.intern(self)
    }
}

impl<T: Internable> Internable for Option<T> {
    fn intern(self, i: &mut Interner) -> Self {
        self.map(|s| s.intern(i))
    }
}

impl<T: Internable> Internable for Vec<T> {
    fn intern(self, i: &mut Interner) -> Self {
      self.into_iter().map(|v| v.intern(i)).collect()
    }
}

#[derive(Default, Clone)]
pub struct Interner {
    intern_cache: BTreeSet<RcStr>,
}

impl Interner {
    fn intern(&mut self, k: RcStr) -> RcStr {
        self.intern_cache
            .get(&k)
            .map(Clone::clone)
            .unwrap_or_else(|| {
                self.intern_cache.insert(k.clone());
                k
            })
    }
}

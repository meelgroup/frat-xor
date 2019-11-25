use std::hash::{Hash, Hasher};
use std::num::Wrapping;

pub fn is_perm(v: &Vec<i64>, w: &Vec<i64>) -> bool {
  v.len() == w.len() && v.iter().all(|i| w.contains(i))
}

#[derive(Copy, Clone)]
pub struct PermClauseRef<'a>(pub &'a Vec<i64>);

impl<'a> Hash for PermClauseRef<'a> {
  fn hash<H: Hasher>(&self, h: &mut H) {
    // Permutation-stable hash function from drat-trim.c
    let (mut sum, mut prod, mut xor) = (Wrapping(0u64), Wrapping(1u64), Wrapping(0u64));
    for &i in self.0 { let i = Wrapping(i as u64); prod *= i; sum += i; xor ^= i; }
    (Wrapping(1023) * sum + prod ^ (Wrapping(31) * xor)).hash(h)
  }
}

impl<'a> PartialEq for PermClauseRef<'a> {
  fn eq(&self, other: &Self) -> bool { is_perm(self.0, other.0) }
}
impl<'a> Eq for PermClauseRef<'a> {}

pub struct PermClause(pub Vec<i64>);

impl PermClause {
  pub fn as_ref(&self) -> PermClauseRef { PermClauseRef(&self.0) }
}

impl Hash for PermClause {
  fn hash<H: Hasher>(&self, h: &mut H) {
    self.as_ref().hash(h)
  }
}

impl PartialEq for PermClause {
  fn eq(&self, other: &Self) -> bool { is_perm(&self.0, &other.0) }
}
impl Eq for PermClause {}
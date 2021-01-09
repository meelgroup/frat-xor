use std::io::{self, Read, BufReader, Write, BufWriter};
use std::fs::{File, read_to_string};
use std::convert::TryInto;
use std::collections::VecDeque;
use std::mem;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use hashbrown::hash_map::{HashMap, Entry};
use super::dimacs::parse_dimacs;
use super::serialize::Serialize;
use super::parser::{detect_binary, Step, StepRef, ElabStep,
  Proof, ProofRef, Mode, Ascii, Bin, LRATParser, LRATStep};
use super::backparser::{StepParser, ElabStepParser};
use super::perm_clause::*;

struct VAssign {
  values: Vec<Option<bool>>
}

fn var(l: i64) -> usize { l.abs() as usize }

impl VAssign {

  fn new() -> VAssign {
    VAssign { values: Vec::new() }
  }

  fn val(&self, l: i64) -> Option<bool> {
    self.values.get(var(l)).unwrap_or(&None).map(|b| (l < 0) ^ b)
  }

  // Attempt to update the variable assignment and make l true under it.
  // If the update is impossible because l is already false under it, return false.
  // Otherwise, update and return true.
  fn set(&mut self, l: i64) -> bool {
    if let Some(v) = self.val(l) { return v }
    let i = var(l);
    if self.values.len() <= i {
      self.values.resize(i + 1, None);
    }
    self.values[i] = Some(0 < l);
    true
  }
}

#[derive(Debug)]
struct Clause {
  marked: bool,
  lits: Vec<i64>
}

impl<'a> IntoIterator for &'a Clause {
  type Item = i64;
  type IntoIter = std::iter::Cloned<std::slice::Iter<'a, Self::Item>>;
  fn into_iter(self) -> Self::IntoIter { self.lits.iter().cloned() }
}

impl Deref for Clause {
  type Target = [i64];
  fn deref(&self) -> &[i64] { self.lits.deref() }
}
impl DerefMut for Clause {
  fn deref_mut(&mut self) -> &mut [i64] { self.lits.deref_mut() }
}
impl Index<usize> for Clause {
  type Output = i64;
  fn index(&self, i: usize) -> &i64 { self.lits.index(i) }
}
impl IndexMut<usize> for Clause {
  fn index_mut(&mut self, i: usize) -> &mut i64 { self.lits.index_mut(i) }
}

#[derive(Default)]
struct Context {
  clauses: HashMap<u64, Clause>,
  units: HashMap<u64, i64>,
  watch: Option<HashMap<i64, HashMap<u64, ()>>>,
  step: Option<u64>
}

fn del_watch(watch: &mut HashMap<i64, HashMap<u64, ()>>, l: i64, i: u64) {
  // eprintln!("remove watch: {:?} for {:?}", l, i);

  assert!(watch.get_mut(&l).unwrap().remove(&i).is_some(),
    "Literal {} not watched in clause {:?}", l, i);
}

fn add_watch(watch: &mut HashMap<i64, HashMap<u64, ()>>, l: i64, id: u64) {
  assert!(watch.entry(l).or_insert_with(HashMap::new).insert(id, ()).is_none(),
    "Clause already watched");
}

impl Context {

  fn new() -> Context { Default::default() }

  fn marked(&self, i: u64) -> bool {
    self.clauses[&i].marked
  }

  fn mark(&mut self, i: u64) {
    self.clauses.get_mut(&i).unwrap().marked = true;
  }

  fn watch(&mut self) -> &mut HashMap<i64, HashMap<u64, ()>> {
    self.watch.get_or_insert_with({ let cl = &self.clauses; move || {
      let mut watch = HashMap::new();
      for (&i, c) in cl {
        if c.len() >= 2 {
          add_watch(&mut watch, c[0], i);
          add_watch(&mut watch, c[1], i);
        }
      }
      watch
    }})
  }

  fn insert(&mut self, i: u64, marked: bool, lits: Vec<i64>) {
    let c = Clause {marked, lits};
    match c.len() {
      0 => {}
      1 => { self.units.insert(i, c[0]); }
      _ => if let Some(w) = &mut self.watch {
        add_watch(w, c[0], i);
        add_watch(w, c[1], i);
      }
    }

    assert!(self.clauses.insert(i, c).is_none(),
      "at {:?}: Clause {} to be inserted already exists", self.step, i);
  }

  fn remove(&mut self, i: u64) -> Clause {

    let c: Clause = self.clauses.remove(&i).unwrap_or_else(
      || panic!("at {:?}: Clause {} to be removed does not exist", self.step, i));

    match c.len() {
      0 => {}
      1 => { self.units.remove(&i); }
      _ => if let Some(w) = &mut self.watch {
        del_watch(w, c[0], i);
        del_watch(w, c[1], i);
      }
    }

    c
  }

  fn reloc(&mut self, relocs: &mut Vec<(u64, u64)>) {
    let mut m = HashMap::new();
    let mut removed = Vec::new();
    relocs.retain(|&(from, to)| {
      if let Some(c) = self.clauses.remove(&to) {
        m.insert(from, to);
        removed.push((from, c));
        true
      } else {false}
    });
    for (from, c) in removed {
      assert!(self.clauses.insert(from, c).is_none(),
        "at {:?}: Clause {} to be inserted already exists", self.step, from);
    }
    if let Some(watch) = &mut self.watch {
      for (_, ws) in watch.iter_mut() {
        for (n, _) in mem::replace(ws, HashMap::new()) {
          ws.insert(m.get(&n).cloned().unwrap_or(n), ());
        }
      }
    }
  }

  fn get(&self, i: u64) -> &Clause {
    self.clauses.get(&i).unwrap_or_else(
      || panic!("at {:?}: Clause {} to be accessed does not exist", self.step, i))
  }

  #[inline] fn watch_idx(&mut self, idx: usize, i: u64, va: &VAssign) -> bool {

    let c = self.clauses.get_mut(&i).unwrap();
    let l = c[idx];

    if va.val(l).is_none() { return true }
    if let Some(j) = find_new_watch(c, va) {
      // eprintln!("Working on clause {}: {:?} at {}", i, c, j);
      let k = c[j];
      c[idx] = k;
      c[j] = l;
      let w = self.watch();
      del_watch(w, l, i);
      add_watch(w, k, i);
      true
    } else {false}
  }

  // m indicates propagation targets (m == true for marked clauses),
  // va is the current variable assignment, and i is the ID of a clause
  // which may potentially be unit under va. If c is verified under va,
  // do nothing and return None. If c is not verified but contains two
  // or more undecided literals, watch them and return none. Otherwise,
  // return Some(k), where k is a new unit literal.
  fn propagate(&mut self, m: bool, i: u64, va: &VAssign) -> Option<i64> {
    if m != self.marked(i) {return None}
    if self.get(i).iter().any(|&l| va.val(l) == Some(true)) {return None}
    if !self.watch_idx(0, i, va) {return Some(self.get(i)[1])}
    if !self.watch_idx(1, i, va) {return Some(self.get(i)[0])}
    None
  }
}

#[derive(Debug, Default)]
struct Hint {
  reasons: HashMap<i64, Option<usize>>,
  steps: Vec<u64>
}

impl Hint {
  fn new() -> Hint { Default::default() }

  fn add(&mut self, l: i64, rs: Option<u64>) {
    match rs {
      None => {
        self.reasons.insert(l, None);
      },
      Some(id) => {
        self.reasons.insert(l, Some(self.steps.len()));
        self.steps.push(id);
      }
    }
  }

  fn minimize(&mut self, ctx: &Context) {
    let mut need = vec![false; self.steps.len()];
    *need.last_mut().unwrap_or_else(
      || panic!("at {:?}: minimizing empty hint", ctx.step)) = true;

    for (i, &s) in self.steps.iter().enumerate().rev() {
      if need[i] {
        for l in ctx.get(s) {
          if let Some(&Some(j)) = self.reasons.get(&-l) {need[j] = true}
        }
      }
    }
    self.steps.retain({ let mut i = 0; move |_| (need[i], i += 1).0 });
  }
}

// Return the next literal to be propagated, and indicate whether
// its propagation should be limited to marked/unmarked clauses
fn next_prop_lit(ls0: &mut VecDeque<i64>, ls1: &mut VecDeque<i64>) -> Option<(bool, i64)> {
  if let Some(l) = ls0.pop_front() { return Some((true, l)); }
  if let Some(l) = ls1.pop_front() { return Some((false, l)); }
  None
}

fn propagate(c: &[i64], ctx: &mut Context) -> Option<Hint> {

  let mut ls0: VecDeque<i64> = VecDeque::new();
  let mut ls1: VecDeque<i64> = VecDeque::new();
  let mut va = VAssign::new();
  let mut ht = Hint::new();

  for l in c {
    ls0.push_back(-l);
    ls1.push_back(-l);
    ht.add(-l, None);
    if !va.set(-l) { return Some(ht) }
  }

  for (&i, &l) in &ctx.units {
    ls0.push_back(l);
    ls1.push_back(l);
    ht.add(l, Some(i));
    if !va.set(l) { return Some(ht) }
  }

  // Main unit propagation loop
  while let Some((m, l)) = next_prop_lit(&mut ls0, &mut ls1) {
    // If l is not watched at all, no new information can be obtained by propagating l
    if let Some(is) = ctx.watch().get(&-l) {
      // 'is' contains (as keys) the IDs of all clauses containing -l
      let js: Vec<u64> = is.keys().cloned().collect();
      for j in js {
        if let Some(k) = ctx.propagate(m, j, &va) {
          ls0.push_back(k);
          ls1.push_back(k);
          ht.add(k, Some(j));
          if !va.set(k) { return Some(ht) }
        }
      }
    }
  }

  // If there are no more literals to propagate, unit propagation has failed
  // let _ = propagate_stuck(ctx, &ht, &ls, c);
  // panic!("Unit propagation stuck, cannot add clause {:?}", c)
  None
}

// fn propagate_stuck(ctx: &Context, ht: &Hint, ls: &VecDeque<i64>, c: &Vec<i64>) -> io::Result<()> {
//   // If unit propagation is stuck, write an error log
//   let mut log = File::create("unit_prop_error.log")?;
//   writeln!(log, "Clauses available at failure:\n")?;
//   for ac in &ctx.clauses {
//     writeln!(log, "{:?}: {:?}", ac.0, ac.1.lits)?;
//   }
//   writeln!(log, "\nDiscovered reasons at failure: {:?}", ht.reasons)?;
//   writeln!(log, "\nRecorded steps at failure: {:?}", ht.steps)?;
//   writeln!(log, "\nObtained unit literals at failure: {:?}", ls)?;
//   writeln!(log, "\nFailed to add clause: {:?}", c)
// }

fn propagate_hint(ls: &[i64], ctx: &Context, is: &[i64], strict: bool) -> (Hint, bool) {
  let mut ht: Hint = Hint { reasons: ls.iter().map(|&x| (-x, None)).collect(), steps: vec![] };

  let mut is: Vec<u64> = is.iter().map(|&i| i as u64).collect();
  let mut queue = vec![];
  loop {
    let len = is.len();
    'a: for c in is.drain(..) {
      let mut uf: Option<i64> = None;
      let cl = ctx.get(c);
      for l in cl {
        if !ht.reasons.contains_key(&-l) {
          if uf.replace(l).is_some() {
            assert!(!strict, "at {:?}: clause {:?} is not unit", ctx.step, c);
            queue.push(c);
            continue 'a
          }
        }
      }
      match uf {
        None => {
          ht.steps.push(c);
          return (ht, true)
        },
        Some(l) => if let Entry::Vacant(v) = ht.reasons.entry(l) {
          v.insert(Some(ht.steps.len()));
          ht.steps.push(c);
        }
      }
    }
    if queue.len() >= len {return (ht, false)}
    mem::swap(&mut is, &mut queue);
  }
}

fn build_step(ls: &[i64], ctx: &mut Context, hint: Option<&[i64]>, strict: bool) -> Option<Vec<i64>> {
  let mut ht = hint.and_then(|is| {
    let (ht, success) = propagate_hint(&ls, &ctx, is, strict);
    if success {Some(Some(ht))} else {None}
  }).unwrap_or_else(|| propagate(ls, ctx))?;
  ht.minimize(&ctx);
  Some(ht.steps.iter().map(|&i| i as i64).collect())
}

fn run_rat_step<'a>(ls: &[i64], ctx: &mut Context, init: &[i64],
    mut rats: Option<(&'a i64, &'a [i64])>, strict: bool) -> Vec<i64> {
  if rats.is_none() {
    if let Some(res) = build_step(ls, ctx, if init.is_empty() {None} else {Some(init)}, strict) {
      return res
    }
  }
  let Hint {mut reasons, ..} = propagate_hint(&ls, &ctx, init, strict).0;
  let pivot = ls[0];
  reasons.remove(&-pivot);
  let ls2: Vec<i64> = reasons.into_iter().map(|(i, _)| -i).collect();
  let mut proofs = HashMap::new();
  let mut order = vec![];
  while let Some((&s, rest)) = rats {
    let step = -s as u64;
    order.push(step);
    match rest.iter().position(|&i| i < 0) {
      None => {
        proofs.insert(step, rest);
        break
      }
      Some(i) => {
        let (chain, r) = rest.split_at(i);
        proofs.insert(step, chain);
        rats = r.split_first();
      }
    }
  }
  let mut steps = vec![];
  let mut todo = vec![];
  for (&c, cl) in &ctx.clauses {
    if cl.lits.contains(&-pivot) {
      let mut resolvent = ls2.clone();
      resolvent.extend(cl.lits.iter().cloned().filter(|&i| i != -pivot));
      match proofs.get(&c) {
        Some(&chain) => todo.push((c, resolvent, Some(chain))),
        None if strict => panic!("Clause {:?} not in LRAT trace", cl.lits),
        None => {
          order.push(c);
          todo.push((c, resolvent, None));
        }
      }
    }
  }
  let mut proofs: HashMap<_, _> = todo.into_iter().map(|(c, resolvent, hint)|
    (c, build_step(&resolvent, ctx, hint, strict).unwrap_or_else(||
      panic!("Unit propagation stuck, cannot resolve clause {:?}", resolvent)))).collect();

  for c in order {
    let mut chain = proofs.remove(&c).unwrap();
    steps.push(-(c as i64));
    steps.append(&mut chain);
  }
  steps
}

fn elab<M: Mode>(mode: M, full: bool, frat: File, temp: File) -> io::Result<()> {
  let w = &mut BufWriter::new(temp);
  let mut ctx: Context = Context::new();
  let mut origs = Vec::new();

  for s in StepParser::new(mode, frat)? {
    // eprintln!("<- {:?}", s);
    match s {

      Step::Orig(i, ls) => {
        ctx.step = Some(i);
        if full || ctx.marked(i) {  // If the original clause is marked
          origs.push((i, ls)); // delay origs to the end
        }
        // else { eprintln!("delete {}", i); }
        ctx.remove(i);
      }

      Step::Add(i, ls, p) => {
        ctx.step = Some(i);
        let c = ctx.remove(i);
        if full || c.marked {
          let steps: Vec<i64> = if let Some(Proof::LRAT(is)) = p {
            if let Some(start) = is.iter().position(|&i| i < 0).filter(|_| !ls.is_empty()) {
              let (init, rest) = is.split_at(start);
              run_rat_step(&ls, &mut ctx, init, rest.split_first(), false)
            } else {
              run_rat_step(&ls, &mut ctx, &is, None, false)
            }
          } else {
            run_rat_step(&ls, &mut ctx, &[], None, false)
          };
          for &i in &steps {
            let i = i.abs() as u64;
            // let v = cs.get_mut(&i).unwrap();
            if !ctx.marked(i) { // If the necessary clause is not active yet
              ctx.mark(i); // Make it active
              if !full {
                ElabStep::Del(i).write(w).expect("Failed to write delete step");
              }
            }
          }
          ElabStep::Add(i, ls, steps).write(w).expect("Failed to write add step");
        }
        // else { eprintln!("delete {}", i); }
      }

      Step::Reloc(mut relocs) => {
        ctx.step = None;
        ctx.reloc(&mut relocs);
        if !relocs.is_empty() {
          ElabStep::Reloc(relocs).write(w).expect("Failed to write reloc step");
        }
      }

      Step::Del(i, ls) => {
        ctx.insert(i, false, ls);
        if full {
          ElabStep::Del(i).write(w).expect("Failed to write delete step");
        }
      },

      Step::Final(i, ls) => {
        // Identical to the Del case, except that the clause should be marked if empty
        ctx.insert(i, ls.is_empty(), ls);
      }

      Step::Todo(_) => ()
    }
  }

  for (i, ls) in origs {
    ElabStep::Orig(i, ls).write(w).expect("Failed to write orig step");
  }

  w.flush()
}

fn find_new_watch(c: &Clause, va: &VAssign) -> Option<usize> {
  c.iter().skip(2).position(|x| va.val(*x).is_none()).map(|u| u+2)
}

struct DeleteLine<'a, W>(&'a mut W, u64, bool);

impl<'a, W: Write> DeleteLine<'a, W> {
  fn with(lrat: &'a mut W, step: u64,
    f: impl FnOnce(&mut DeleteLine<'a, W>) -> io::Result<()>
  ) -> io::Result<()> {
    let mut l = DeleteLine(lrat, step, false);
    f(&mut l)?;
    if l.2 { write!(l.0, " 0\n")? }
    Ok(())
  }

  fn delete(&mut self, i: u64) -> io::Result<()> {
    if mem::replace(&mut self.2, true) {
      write!(self.0, " {}", i)
    } else {
      write!(self.0, "{} d {}", self.1, i)
    }
  }
}

fn trim(cnf: &[Vec<i64>], temp: File, lrat: &mut impl Write) -> io::Result<()> {

  let mut k = 0 as u64; // Counter for the last used ID
  let cnf: HashMap<PermClauseRef, u64> = // original CNF
    cnf.iter().map(|c| (PermClauseRef(c), (k += 1, k).1)).collect();
  let mut m: HashMap<u64, u64> = HashMap::new(); // Mapping between old and new IDs
  let mut bp = ElabStepParser::new(Bin, temp)?.peekable();
  let origs = k;
  let mut used_origs = vec![0u8; origs as usize];

  while let Some(ElabStep::Orig(_, _)) = bp.peek() {
    if let Some(ElabStep::Orig(i, ls)) = bp.next() {
      // eprintln!("-> Orig{:?}", (&i, &ls));
      let j = *cnf.get(&PermClauseRef(&ls)).unwrap_or_else( // Find position of clause in original problem
        || panic!("Orig step {} refers to nonexistent clause {:?}", i, ls));
      let r = &mut used_origs[j as usize - 1];
      *r = r.saturating_add(1);
      assert!(m.insert(i, j).is_none(), "Multiple orig steps with duplicate IDs");
      // eprintln!("{} -> {}", i, j);
      if ls.is_empty() {
        write!(lrat, "{} 0 {} 0\n", k+1, j)?;
        return Ok(())
      }
    } else {unreachable!()}
  }

  DeleteLine::with(lrat, k, |line| {
    for (j, &b) in used_origs.iter().enumerate() {
      if b == 0 { line.delete(j as u64 + 1)? }
    }
    Ok(())
  })?;

  while let Some(s) = bp.next() {
    // eprintln!("-> {:?}", s);

    match s {

      ElabStep::Orig(_, _) =>
        panic!("Orig steps must come at the beginning of the temp file"),

      ElabStep::Add(i, ls, is) => {
        k += 1; // Get the next fresh ID
        m.insert(i, k); // The ID of added clause is mapped to a fresh ID
        // eprintln!("{} -> {}", i, k);
        let b = ls.is_empty();

        write!(lrat, "{}", k)?;
        for x in ls { write!(lrat, " {}", x)? }
        write!(lrat, " 0")?;
        for x in is {
          let ux = x.abs() as u64;
          let s = *m.get(&ux).unwrap_or_else(||
            panic!("step {}: proof step {:?} not found", i, ux)) as i64;
          write!(lrat, " {}", if x < 0 {-s} else {s})?
        }
        write!(lrat, " 0\n")?;

        if b {return Ok(())}
      }

      ElabStep::Reloc(relocs) => {
        let removed: Vec<_> = relocs.iter()
          .map(|(from, to)| (*to, m.remove(from))).collect();
        for (to, o) in removed {
          if let Some(s) = o { m.insert(to, s); }
        }
      }

      ElabStep::Del(i) => DeleteLine::with(lrat, k, |line| {
        let m = &mut m;
        let used_origs = &mut used_origs;
        let mut delete = move |i| -> io::Result<()> {
          let j = m.remove(&i).unwrap();
          if match used_origs.get_mut(j as usize - 1) {
            None => true,
            Some(&mut u8::MAX) => false,
            Some(refc) => { *refc -= 1; *refc == 0 }
          } { line.delete(j)? }
          Ok(())
        };

        // Remove ID mapping to free space
        delete(i)?;
        // agglomerate additional del steps into this block
        while let Some(&ElabStep::Del(i)) = bp.peek() {
          bp.next();
          delete(i)?;
        }
        Ok(())
      })?
    }
  }

  panic!("did not find empty clause");
}

pub fn main(args: impl Iterator<Item=String>) -> io::Result<()> {
  let mut args = args.peekable();
  let full = if args.peek().map_or(false, |s| s == "--full") {
    args.next(); true
  } else {false};
  let dimacs = args.next().expect("missing input file");
  let frat_path = args.next().expect("missing proof file");

  let temp_path = format!("{}.temp", frat_path);
  let mut frat = File::open(frat_path)?;
  let bin = detect_binary(&mut frat)?;
  let temp_write = File::create(&temp_path)?;
  println!("elaborating...");
  if bin { elab(Bin, full, frat, temp_write)? }
  else { elab(Ascii, full, frat, temp_write)? };

  if !full {
    println!("parsing DIMACS...");
    let temp_read = File::open(temp_path)?;
    let (_vars, cnf) = parse_dimacs(read_to_string(dimacs)?.chars());
    println!("trimming...");
    if let Some(lrat_file) = args.next() {
      let mut lrat = BufWriter::new(File::create(&lrat_file)?);
      trim(&cnf, temp_read, &mut lrat)?;
      lrat.flush()?;
      match args.next() {
        Some(ref s) if s == "-v" => {
          println!("verifying...");
          check_lrat(Ascii, cnf, &lrat_file)?;
          println!("VERIFIED");
        }
        _ => ()
      }
    } else {
      trim(&cnf, temp_read, &mut io::sink())?;
    }
  }
  Ok(())
}

fn check_lrat(mode: impl Mode, cnf: Vec<Vec<i64>>, lrat_file: &str) -> io::Result<()> {
  let lrat = File::open(lrat_file)?;
  let lp = LRATParser::from(mode, BufReader::new(lrat).bytes().map(Result::unwrap));
  let mut ctx: Context = Context::new();
  let mut k = 0;

  for c in cnf {
    k += 1;
    ctx.step = Some(k);
    // eprintln!("{}: {:?}", k, c);
    ctx.insert(k, true, c);
  }

  for (i, s) in lp {
    ctx.step = Some(i);
    // eprintln!("{}: {:?}", i, s);
    match s {

      LRATStep::Add(ls, p) => {
        assert!(i > k, "out-of-order LRAT proofs not supported");
        k = i;
        // eprintln!("{}: {:?} {:?}", k, ls, p);
        if let Some(start) = p.iter().position(|&i| i < 0).filter(|_| !ls.is_empty()) {
          let (init, rest) = p.split_at(start);
          run_rat_step(&ls, &mut ctx, init, rest.split_first(), false);
        } else {
          run_rat_step(&ls, &mut ctx, &p, None, false);
        }
        if ls.is_empty() { return Ok(()) }
        ctx.insert(i, true, ls);
      }

      LRATStep::Del(ls) => {
        assert!(i == k, "out-of-order LRAT proofs not supported");
        for c in ls { ctx.remove(c.try_into().unwrap()); }
      }
    }
  }

  panic!("did not find empty clause")
}

pub fn lratchk(mut args: impl Iterator<Item=String>) -> io::Result<()> {
  let dimacs = args.next().expect("missing input file");
  let (_vars, cnf) = parse_dimacs(read_to_string(dimacs)?.chars());
  check_lrat(Ascii, cnf, &args.next().expect("missing proof file"))
}

fn refrat_pass(elab: File, w: &mut impl Write) -> io::Result<()> {

  let mut ctx: HashMap<u64, Vec<i64>> = HashMap::new();
  for s in ElabStepParser::new(Bin, elab)? {
    eprintln!("-> {:?}", s);

    match s {

      ElabStep::Orig(i, ls) => {
        StepRef::Orig(i, &ls).write(w)?;
        ctx.insert(i, ls);
      }

      ElabStep::Add(i, ls, is) => {
        StepRef::Add(i, &ls, Some(ProofRef::LRAT(&is))).write(w)?;
        ctx.insert(i, ls);
      }

      ElabStep::Reloc(relocs) => {
        StepRef::Reloc(&relocs).write(w)?;
        let removed: Vec<_> = relocs.iter()
          .map(|(from, to)| (*to, ctx.remove(from))).collect();
        for (to, o) in removed {
          if let Some(s) = o { ctx.insert(to, s); }
        }
      }

      ElabStep::Del(i) => {
        Step::Del(i, ctx.remove(&i).unwrap()).write(w)?;
      }
    }
  }

  for (i, s) in ctx { Step::Final(i, s).write(w)? }

  Ok(())
}

pub fn refrat(mut args: impl Iterator<Item=String>) -> io::Result<()> {
  let elab_path = args.next().expect("missing elab file");
  let frat_path = args.next().expect("missing frat file");
  let w = &mut BufWriter::new(File::create(&frat_path)?);
  refrat_pass(File::open(elab_path)?, w)?;
  w.flush()
}
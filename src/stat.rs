use std::fs::File;
use std::process::exit;
use std::io;
use crate::HashMap;
use crate::parser::{detect_binary, Mode, Ascii, Bin};
use crate::backparser::*;

type Clause = Vec<i64>;

fn subsumes(clause: &[i64], clause2: &[i64]) -> bool {
  clause2.iter().all(|lit2| clause.contains(lit2))
}

pub fn check_proof(mode: impl Mode, proof: File) -> io::Result<()> {
  let mut bp = StepIter(BackParser::new(mode, proof)?).peekable();
  let (mut orig, mut added, mut deleted, mut fin, mut _orig_xor, mut _add_xor, mut _del_xor, mut _imply, mut _imply_xor, mut _fin_xor) = (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64);
  let (mut dirty_orig, mut dirty_add, mut dirty_imply, mut double_del, mut double_fin) = (0i64, 0i64, 0i64, 0i64, 0i64);
  let mut missing = 0i64;
  let mut active: HashMap<u64, (bool, Clause)> = HashMap::default();
  let mut todos = HashMap::default();
  let mut bad = false;
  while let Some(s) = bp.next() {
    // println!("{:?}", s);
    match s {
      Step::Comment(_) => {}
      Step::Orig(i, lits) => {
        orig += 1;
        match active.remove(&i) {
          None => {
            dirty_orig += 1;
            // eprintln!("original clause {} {:?} never finalized", i, lits);
          },
          Some((_, lits2)) => if !subsumes(&lits2, &lits) {
            eprintln!("orig step {}: added {:?}, removed {:?}", i, lits, lits2);
            bad = true;
          }
        }
      },
      Step::Add(i, step, p) => {
        added += 1;
        if p.is_none() { missing += 1 }
        if let Some(Step::Todo(_)) = bp.peek() {} else if p.is_none() {
          *todos.entry(0).or_insert(0i64) += 1;
          // eprintln!("added clause {} {:?} has no proof and no todo", i, lits);
        }
        if let Some((need, lits2)) = active.remove(&i) {
          let lits = step.parse().lemma();
          if !subsumes(&lits2, lits) {
            eprintln!("add step {}: added {:?}, removed {:?}", i, lits, lits2);
            bad = true;
          }
          if need {
            if let Some(Proof::LRAT(steps)) = p {
              for s in steps {
                let needed = &mut active.get_mut(&s.unsigned_abs()).expect("bad LRAT proof").0;
                if !*needed {
                  // unimplemented!();
                  *needed = true;
                }
              }
            }
          }
        } else {
          dirty_add += 1;
          // eprintln!("added clause {} {:?} never finalized", i, lits);
        }
      },
      Step::Reloc(relocs) => {
        let removed: Vec<_> = relocs.iter()
          .map(|(from, to)| (*from, active.remove(to))).collect();
        for (from, o) in removed {
          if let Some(s) = o {
            if active.insert(from, s).is_some() {
              double_del += 1;
              // eprintln!("already deleted clause {} {:?}", i, active[&i]);
            }
          } else {
            dirty_add += 1;
            // eprintln!("added clause {} {:?} never finalized", i, lits);
          }
        }
      },
      Step::Del(i, lits) => {
        deleted += 1;
        if active.insert(i, (false, lits)).is_some() {
          double_del += 1;
          // eprintln!("already deleted clause {} {:?}", i, active[&i]);
        }
      },
      Step::Final(i, lits) => {
        fin += 1;
        if active.insert(i, (lits.is_empty(), lits)).is_some() {
          double_fin += 1;
          // eprintln!("already finalized clause {} {:?}", i, active[&i]);
        }
      },
      Step::Todo(i) => *todos.entry(i).or_insert(0i64) += 1,
      Step::OrigXor(_i, _lits) => {
        _orig_xor += 1;
      },
      Step::AddXor(_i, _lits, _p, u) => {
        _add_xor += 1;
        if let Some(Proof::Unit(units)) = u {
          for us in units {
            let needed = &mut active.get_mut(&us).expect("bad unit hints for add-xor step").0;
            if !*needed {
              *needed = true;
            }
          }
        }
      },
      Step::DelXor(_i, _lits) => {
        _del_xor += 1;
      },
      Step::Imply(i, lits, _p) => {
        _imply += 1;
        if let Some((_need, lits2)) = active.remove(&i) {
          if !subsumes(&lits2, &lits) {
            eprintln!("imply step {}: added {:?}, removed {:?}", i, lits, lits2);
            bad = true;
          }
        } else {
          dirty_imply += 1;
          // eprintln!("implied clause {} {:?} never finalized", i, lits);
        }
      },
      Step::ImplyXor(_i, _lits, p) => {
        _imply_xor += 1;
        if let Some(Proof::LRAT(steps)) = p {
          for s in steps {
            let needed = &mut active.get_mut(&s.unsigned_abs()).expect("bad clause hints for imply-xor step").0;
            if !*needed {
              *needed = true;
            }
          }
        }
      },
      Step::FinalXor(_i, _lits) => {
        _fin_xor += 1;
      }, 
    }
  }
  println!("{} orig + {} added - {} deleted - {} finalized = {}",
    orig, added, deleted, fin, orig + added - deleted - fin);
  println!("{} missing proofs ({:.1}%)", missing, 100. * missing as f32 / added as f32);
  let mut todo_vec: Vec<_> = todos.into_iter().collect();
  todo_vec.sort_by_key(|(_, v)| -v);
  for (k, v) in todo_vec.into_iter().take(5).filter(|&(_, v)| v != 0) {
    println!("type {}: {}", k, v);
  }
  if dirty_orig != 0 || dirty_add != 0 || dirty_imply != 0 {
    eprintln!("{} original + {} added + {} implied never finalized", dirty_orig, dirty_add, dirty_imply);
    bad = true;
  }
  if double_del != 0 || double_fin != 0 {
    eprintln!("{} double deletes + {} double finalized", double_del, double_fin);
    bad = true;
  }
  if !active.is_empty() {
    eprintln!("{} unjustified", active.len());
    bad = true;
  }
  if bad { exit(1) }
  Ok(())
}

pub fn main<I: Iterator<Item=String>>(mut args: I) -> io::Result<()> {
  let mut proof = File::open(args.next().expect("missing proof file"))?;
  let bin = detect_binary(&mut proof)?;
  if bin { check_proof(Bin, proof) }
  else { check_proof(Ascii, proof) }
}

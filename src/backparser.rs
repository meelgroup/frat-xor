use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use super::parser::*;
pub use super::parser::{Proof, Step, ElabStep};

pub struct VecBackParser(pub Vec<u8>);

impl Iterator for VecBackParser {
  type Item = Segment;

  fn next(&mut self) -> Option<Segment> {
    let (&n, most) = self.0.split_last()?;
    if n != 0 { panic!("expected 0 byte") }
    let i = most.iter().rposition(|&n| n == 0).map_or(0, |i| i + 1);
    Some(Bin.segment(|| i, self.0.drain(i..)))
  }
}

pub struct BackParser<M: Mode> {
  file: File,
  remaining: usize,
  pos: usize,
  last_read: usize,
  buffers: Vec<Box<[u8; BUFFER_SIZE]>>,
  free: Vec<Box<[u8; BUFFER_SIZE]>>,
  mode: M,
  scan: M::BackScanState,
}

impl<M: Mode> BackParser<M> {
  pub fn new(mode: M, mut file: File) -> io::Result<BackParser<M>> {
    let len = file.metadata()?.len() as usize;
    let pos = len.checked_sub(1).map_or(0, |l| l % BUFFER_SIZE + 1);
    file.seek(SeekFrom::End(-(pos as i64)))?;
    let mut buf = Box::new([0; BUFFER_SIZE]);
    file.read_exact(&mut buf[..pos])?;
    Ok(BackParser {
      file,
      remaining: len / BUFFER_SIZE - if pos == BUFFER_SIZE {1} else {0},
      pos,
      last_read: pos,
      buffers: vec![buf],
      free: Vec::new(),
      scan: mode.new_back_scan(),
      mode,
    })
  }

  fn read_chunk(&mut self) -> io::Result<Option<Box<[u8; BUFFER_SIZE]>>> {
    if self.remaining == 0 { return Ok(None) }
    let mut buf = self.free.pop().unwrap_or_else(|| Box::new([0; BUFFER_SIZE]));
    self.file.seek(SeekFrom::Current(-((BUFFER_SIZE + self.last_read) as i64)))?;
    self.file.read_exact(&mut *buf)?;
    self.last_read = BUFFER_SIZE;
    self.remaining -= 1;
    Ok(Some(buf))
  }

  fn parse_segment_from(&mut self, b: usize, i: usize) -> Segment {
    let seg_start = || (self.remaining + (self.buffers.len() - (b + 1))) * BUFFER_SIZE + i;
    if b == 0 {
      let res = self.mode.segment(seg_start, self.buffers[0][i..self.pos].iter().copied());
      self.pos = i;
      res
    } else {
      let res = self.mode.segment(seg_start,
        self.buffers[b][i..].iter()
          .chain(self.buffers[1..b].iter().rev().flat_map(|buf| buf.iter()))
          .chain(self.buffers[0][..self.pos].iter()).copied());
      self.pos = i;
      self.free.extend(self.buffers.drain(0..b));
      res
    }
  }
}

impl<M: Mode> Iterator for BackParser<M> {
  type Item = Segment;

  fn next(&mut self) -> Option<Segment> {
    for b in 0.. {
      let buf: &[u8; BUFFER_SIZE] = match self.buffers.get(b) {
        None => match self.read_chunk().expect("could not read from proof file") {
          None => {
            if b == 1 && self.pos == 0 { break }
            return Some(self.parse_segment_from(b-1, 0))
          },
          Some(buf) => { self.buffers.push(buf); self.buffers.last().unwrap() }
        },
        Some(buf) => buf
      };
      if b == 0 {
        if self.pos != 0 {
          if let Some(i) = self.scan.back_scan(&buf[..self.pos-1]) {
            return Some(self.parse_segment_from(b, i))
          }
        }
      } else if let Some(i) = self.scan.back_scan(buf) {
        return Some(self.parse_segment_from(b, i))
      }
    }
    None
  }
}

pub struct StepIter<I>(pub I);

impl<I: Iterator<Item=Segment>> Iterator for StepIter<I> {
  type Item = Step;

  fn next(&mut self) -> Option<Step> {

    fn _panic<I: Iterator<Item=Segment>>(self_ref: &mut StepIter<I>, msg: &str, mut next: Option<Segment>) -> Option<Step> {
      if let None = next {
        next = self_ref.0.next()
      }

      loop {
        match next {
          Some(Segment::Orig(idx, _)) => panic!("orig step {}: {}", idx, msg),
          Some(Segment::Add(idx, _)) => panic!("add step {}: {}", idx, msg),
          Some(Segment::Del(idx, _)) => panic!("del step {}: {}", idx, msg),
          Some(Segment::Final(idx, _)) => panic!("final step {}: {}", idx, msg),
          Some(Segment::Xor(idx, _)) => match self_ref.0.next() {
            Some(Segment::DelXor()) => panic!("del-xor step {}: {}", idx, msg),
            Some(Segment::FinalXor()) => panic!("final-xor step {}: {}", idx, msg),
            _ => panic!("xor step {}: {}", idx, msg),
          }
          Some(Segment::Imply(idx, _)) => panic!("imply step {}: {}", idx, msg),
          None => panic!("{}", msg),
          _ => { next = self_ref.0.next() },
        }
      }
    }

    match self.0.next() {
      None => None,
      Some(Segment::Comment(s)) => Some(Step::Comment(s)),
      Some(Segment::Orig(idx, vec)) => Some(Step::Orig(idx, vec)),
      Some(Segment::Add(idx, vec)) => Some(Step::Add(idx, AddStep(vec), None)),
      Some(Segment::Del(idx, vec)) => Some(Step::Del(idx, vec)),
      Some(Segment::Reloc(relocs)) => Some(Step::Reloc(relocs)),
      Some(Segment::Final(idx, vec)) => Some(Step::Final(idx, vec)),
      Some(Segment::LProof(steps)) => match self.0.next() {
        Some(Segment::Add(idx, vec)) =>
          Some(Step::Add(idx, AddStep(vec), Some(Proof::LRAT(steps)))),
        Some(Segment::Xor(idx, vec)) => match self.0.next() {
          Some(Segment::AddXor()) => 
            Some(Step::AddXor(idx, vec, Some(Proof::LRAT(steps)), None)),
          Some(Segment::ImplyXor()) =>
            Some(Step::ImplyXor(idx, vec, Some(Proof::LRAT(steps)))),
          Some(Segment::DelXor()) => panic!("del-xor step {}: 'x' 'l' step not preceded by 'a' or 'i' step", idx),
          Some(Segment::FinalXor()) => panic!("final-xor step {}: 'x' 'l' step not preceded by 'a' or 'i' step", idx),
          _ => panic!("xor step {}: 'x' 'l' step not preceded by 'a' or 'i' step", idx)
        }
        Some(Segment::Imply(idx, vec)) =>
          Some(Step::Imply(idx, vec, Some(Proof::LRAT(steps)))),
        other => _panic(self, "'l' step not preceded by 'a', 'x', or 'i' step", other)
      },
      Some(Segment::Todo(idx)) => Some(Step::Todo(idx)),
      Some(Segment::Xor(idx, vec)) => match self.0.next() {
        Some(Segment::OrigXor()) => Some(Step::OrigXor(idx, vec)),
        Some(Segment::AddXor()) => Some(Step::AddXor(idx, vec, None, None)),
        Some(Segment::DelXor()) => Some(Step::DelXor(idx, vec)),
        Some(Segment::ImplyXor()) => Some(Step::ImplyXor(idx, vec, None)),
        Some(Segment::FinalXor()) => Some(Step::FinalXor(idx, vec)),
        _ => panic!("xor step {}: 'x' step not preceded by 'o', 'a', 'd', 'i', or 'f' step", idx)
      }
      Some(Segment::OrigXor()) => _panic(self, "'o' step not followed by a clause or 'x' step", None),
      Some(Segment::AddXor()) => _panic(self, "'a' step not followed by a clause or 'x' step", None),
      Some(Segment::DelXor()) => _panic(self, "'d' step not followed by a clause or 'x' step", None),
      Some(Segment::Imply(idx, vec)) => Some(Step::Imply(idx, vec, None)),
      Some(Segment::ImplyXor()) => _panic(self, "'i' step not followed by a clause or 'x' step", None),
      Some(Segment::FinalXor()) => _panic(self, "'f' step not followed by a clause or 'x' step", None),
      Some(Segment::Unit(units)) => match self.0.next() {
        Some(Segment::LProof(steps)) => match self.0.next() {
          Some(Segment::Xor(idx, vec)) => match self.0.next() {
            Some(Segment::AddXor()) => Some(Step::AddXor(idx, vec, Some(Proof::LRAT(steps)), Some(Proof::Unit(units)))),
            Some(Segment::DelXor()) => panic!("del-xor step {}: 'x' 'l' 'u' step not preceded by 'a' step", idx),
            Some(Segment::FinalXor()) => panic!("final-xor step {}: 'x' 'l' 'u' step not preceded by 'a' step", idx),
            _ => panic!("xor step {}: 'x' 'l' 'u' step not preceded by 'a' step", idx),
          }
          other => _panic(self, "'l' 'u' step not preceded by 'x' step", other),
        }
        other => _panic(self, "'u' step not preceded by 'l' step", other),
      }
    }
  }
}

pub struct ElabStepIter<I>(pub I);

impl<I: Iterator<Item=Segment>> Iterator for ElabStepIter<I> {
  type Item = ElabStep;

  fn next(&mut self) -> Option<ElabStep> {
    match self.0.next() {
      None => None,
      Some(Segment::Comment(s)) => Some(ElabStep::Comment(s)),
      Some(Segment::Orig(idx, vec)) => Some(ElabStep::Orig(idx, vec)),
      Some(Segment::Add(idx, _)) => panic!("add step {}: add step has no proof", idx),
      Some(Segment::Del(idx, vec)) =>
        {assert!(vec.is_empty()); Some(ElabStep::Del(idx))},
      Some(Segment::Reloc(relocs)) => Some(ElabStep::Reloc(relocs)),
      Some(Segment::LProof(steps)) => match self.0.next() {
        Some(Segment::Add(idx, vec)) =>
          Some(ElabStep::Add(idx, AddStep(vec), steps)),
        Some(Segment::Xor(idx, vec)) => match self.0.next() {
          Some(Segment::AddXor()) => Some(ElabStep::AddXor(idx, vec, steps, None)),
          Some(Segment::ImplyXor()) => Some(ElabStep::ImplyXor(idx, vec, steps)),
          _ => panic!("xor step {}: 'x' 'l' step not preceded by 'a' or 'i' step", idx)
        }
        Some(Segment::Imply(idx, vec)) =>
          Some(ElabStep::Imply(idx, vec, steps)),
        _ => panic!("'l' step not preceded by 'a', 'x', or 'i' step")
      },
      Some(Segment::Final(idx, _)) => panic!("final step {}: unexpected 'f' segment", idx),
      Some(Segment::Todo(_)) => self.next(),
      Some(Segment::Xor(idx, vec)) => match self.0.next() {
        Some(Segment::OrigXor()) => Some(ElabStep::OrigXor(idx, vec)),
        Some(Segment::AddXor()) => panic!("add-xor step {}: add XOR step has no proof", idx),
        Some(Segment::DelXor()) =>
          {assert!(vec.is_empty()); Some(ElabStep::DelXor(idx))},
        Some(Segment::ImplyXor()) => panic!("imply-xor step {}: imply XOR step has no proof", idx),
        Some(Segment::FinalXor()) => panic!("final-xor step {}: unexpected 'f x' segment", idx),
        _ => panic!("xor step {}: 'x' step not preceded by 'o', 'a', 'd', 'i', or 'f' step", idx)
      }
      Some(Segment::OrigXor()) => panic!("'o' step not followed by a clause or 'x' step"),
      Some(Segment::AddXor()) => panic!("'a' step not followed by a clause or 'x' step"),
      Some(Segment::DelXor()) => panic!("'d' step not followed by a clause or 'x' step"),
      Some(Segment::Imply(idx, _)) => panic!("imply step {}: imply step has no proof", idx),
      Some(Segment::ImplyXor()) => panic!("'i' step not followed by a clause or 'x' step"),
      Some(Segment::FinalXor()) => panic!("unexpected 'f' segment"),
      Some(Segment::Unit(units)) => match self.0.next() {
        Some(Segment::LProof(steps)) => match self.0.next() {
          Some(Segment::Xor(idx, vec)) => match self.0.next() {
            Some(Segment::AddXor()) => Some(ElabStep::AddXor(idx, vec, steps, Some(Proof::Unit(units)))),
            _ => panic!("xor step {}: 'x' 'l' 'u' step not preceded by 'a' step", idx),
          }
          _ => panic!("'l' 'u' step not preceded by 'x' step"),
        }
        _ => panic!("'u' step not preceded by 'l' step"),
      }
    }
  }
}

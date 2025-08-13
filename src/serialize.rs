use arrayvec::ArrayVec;
use std::io::{self, Write};
use super::parser::{Ascii, Bin, DefaultMode,
  Step, StepRef, AddStep, AddStepRef, ElabStep, ElabStepRef, ProofRef};

pub trait ModeWrite<M=DefaultMode>: Write {}

pub struct ModeWriter<M, W>(pub M, pub W);

impl<M, W: Write> Write for ModeWriter<M, W> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.1.write(buf) }
  fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.1.write_all(buf) }
  fn flush(&mut self) -> io::Result<()> { self.1.flush() }
}
impl<M, W: Write> ModeWrite<M> for ModeWriter<M, W> {}

pub trait Serialize<M=DefaultMode> {
  fn write(&self, w: &mut impl ModeWrite<M>) -> io::Result<()>;
}

impl<A: Serialize<Bin>, B: Serialize<Bin>> Serialize<Bin> for (A, B) {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    self.0.write(w)?; self.1.write(w)
  }
}

impl Serialize<Bin> for u8 {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> { w.write_all(&[*self]) }
}

impl Serialize<Ascii> for u8 {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> { write!(w, "{}", self) }
}

impl<A: Serialize<Bin>> Serialize<Bin> for &[A] {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    for v in *self { v.write(w)? }
    0u8.write(w)
  }
}

impl<A: Serialize<Ascii>> Serialize<Ascii> for &[A] {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> {
    for v in *self { v.write(w)?; write!(w, " ")? }
    write!(w, "0")
  }
}

impl Serialize<Bin> for &str {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    write!(w, "{}", self)?;
    0u8.write(w)
  }
}

impl Serialize<Bin> for u64 {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    let mut val = *self;
    let mut buf = ArrayVec::<[u8; 10]>::new();
    loop {
      if val & !0x7f == 0 {
        buf.push((val & 0x7f) as u8);
        return w.write_all(&buf)
      } else {
        buf.push((val | 0x80) as u8);
        val >>= 7;
      }
    }
  }
}
impl Serialize<Ascii> for u64 {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> { write!(w, "{}", self) }
}

impl Serialize<Bin> for i64 {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
      let u: u64 = if *self < 0 { -*self as u64 * 2 + 1 } else { *self as u64 * 2 };
      u.write(w)
  }
}
impl Serialize<Ascii> for i64 {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> { write!(w, "{}", self) }
}

impl Serialize<Bin> for AddStepRef<'_> {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    match *self {
      AddStepRef::One(ls) => ls.iter().try_for_each(|v| v.write(w))?,
      AddStepRef::Two(ls, ls2) => ls.iter().chain(ls2).try_for_each(|v| v.write(w))?,
    }
    0u8.write(w)
  }
}

impl Serialize<Ascii> for AddStepRef<'_> {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> {
    match *self {
      AddStepRef::One(ls) => for v in ls { v.write(w)?; write!(w, " ")? },
      AddStepRef::Two(ls, ls2) => {
        for v in ls { v.write(w)?; write!(w, " ")? }
        write!(w, " ")?;
        for v in ls2 { v.write(w)?; write!(w, " ")? }
      }
    }
    write!(w, "0")
  }
}

impl<M> Serialize<M> for AddStep where for<'a> &'a [i64]: Serialize<M> {
  fn write(&self, w: &mut impl ModeWrite<M>) -> io::Result<()> { (&*self.0).write(w) }
}

impl<'a> Serialize<Bin> for StepRef<'a> {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    match *self {
      StepRef::Comment(s) => (b'c', s).write(w),
      StepRef::Orig(idx, vec) => (b'o', (idx, vec)).write(w),
      StepRef::Add(idx, vec, None) => (b'a', (idx, vec)).write(w),
      StepRef::Add(idx, vec, Some(ProofRef::LRAT(steps))) =>
        ((b'a', (idx, vec)), (b'l', steps)).write(w),
      StepRef::Add(idx, _, Some(ProofRef::Unit(_))) =>
        panic!("add step {}: unexpected 'u' step following 'a' step", idx),
      StepRef::Reloc(relocs) => (b'r', relocs).write(w),
      StepRef::Del(idx, vec) => (b'd', (idx, vec)).write(w),
      StepRef::Final(idx, vec) => (b'f', (idx, vec)).write(w),
      StepRef::Todo(idx) => (b't', (idx, 0u8)).write(w),
      StepRef::OrigXor(idx, vec) => ((b'o', (0u8, b'x')), (idx, vec)).write(w),
      StepRef::AddXor(idx, vec, pf, uf) => {
        ((b'a', (0u8, b'x')), (idx, vec)).write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          (b'l', steps).write(w)?;
          if let Some(ProofRef::Unit(units)) = uf {
            (b'u', units).write(w)?;
          }
        }
        write!(w, "")
      },
      StepRef::DelXor(idx, vec) => ((b'd', (0u8, b'x')), (idx, vec)).write(w),
      StepRef::Imply(idx, vec, None) => (b'i', (idx, vec)).write(w),
      StepRef::Imply(idx, vec, Some(ProofRef::LRAT(steps))) =>
        ((b'i', (idx, vec)), (b'l', steps)).write(w),
      StepRef::Imply(idx, _, Some(ProofRef::Unit(_))) =>
        panic!("imply step {}: unexpected 'u' step following 'i' step", idx),
      StepRef::ImplyXor(idx, vec, None) => ((b'i', (0u8, b'x')), (idx, vec)).write(w),
      StepRef::ImplyXor(idx, vec, Some(ProofRef::LRAT(steps))) =>
        (((b'i', (0u8, b'x')), (idx, vec)), (b'l', steps)).write(w),
      StepRef::ImplyXor(idx, _, Some(ProofRef::Unit(_))) =>
        panic!("imply-xor step {}: unexpected 'u' step following 'i' 'x' step", idx),
      StepRef::FinalXor(idx, vec) => ((b'f', (0u8, b'x')), (idx, vec)).write(w),
      StepRef::OrigBnn(idx, vec, rhs, out) => {
        ((b'o', (0u8, b'b')), (idx, vec)).write(w)?;
        if out == 0 {
          ((b'k', rhs), 0u8).write(w)?;
        } else {
          (((b'k', rhs), out), 0u8).write(w)?;
        }
        write!(w, "")
      }
      StepRef::AddBnn(idx, vec, rhs, out, pf) => {
        ((b'a', (0u8, b'b')), (idx, vec)).write(w)?;
        if out == 0 {
          ((b'k', rhs), 0u8).write(w)?;
        } else {
          (((b'k', rhs), out), 0u8).write(w)?;
        }
        if let Some(ProofRef::LRAT(steps)) = pf {
          (b'l', steps).write(w)?;
        }
        write!(w, "")
      },
      StepRef::DelBnn(idx, vec, rhs, out) => ((b'd', (0u8, b'b')), ((idx, vec), (((b'k', rhs), out), 0u8))).write(w),
      StepRef::BnnImply(idx, vec, pf, uf) => {
        (b'i', (idx, vec)).write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          ((b'b', (0u8, b'l')), steps).write(w)?;
          if let Some(ProofRef::Unit(units)) = uf {
            (b'u', units).write(w)?;
          }
        }
        write!(w, "")
      },
      StepRef::FinalBnn(idx, vec, rhs, out) => {
        ((b'f', (0u8, b'b')), (idx, vec)).write(w)?;
        if out == 0 {
          ((b'k', rhs), 0u8).write(w)?;
        } else {
          (((b'k', rhs), out), 0u8).write(w)?;
        }
        write!(w, "")
      },
    }
  }
}

impl<'a> Serialize<Ascii> for StepRef<'a> {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> {
    match *self {
      StepRef::Comment(s) =>
        s.split('\n').try_for_each(|s| writeln!(w, "c {}.", s)),
      StepRef::Orig(idx, vec) => {
        write!(w, "o {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::Add(idx, vec, pf) => {
        write!(w, "a {}  ", idx)?; vec.write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  l ")?; steps.write(w)?;
        }
        writeln!(w)
      }
      StepRef::Reloc(relocs) => {
        writeln!(w, "r")?;
        for chunks in relocs.chunks(8) {
          for &(a, b) in chunks {
            write!(w, "  {} {}", a, b)?;
          }
          writeln!(w)?;
        }
        writeln!(w, "  0")
      }
      StepRef::Del(idx, vec) => {
        write!(w, "d {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::Final(idx, vec) => {
        write!(w, "f {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::Todo(idx) => writeln!(w, "t {} 0", idx),
      StepRef::OrigXor(idx, vec) => {
        write!(w, "o x {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::AddXor(idx, vec, pf, uf) => {
        write!(w, "a x {}  ", idx)?; vec.write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  l ")?; steps.write(w)?;
          if let Some(ProofRef::Unit(units)) = uf {
            write!(w, " u ")?; units.write(w)?;
          }
        }
        writeln!(w)
      }
      StepRef::DelXor(idx, vec) => {
        write!(w, "d x {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::Imply(idx, vec, pf) => {
        write!(w, "i {}  ", idx)?; vec.write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  l ")?; steps.write(w)?;
        }
        writeln!(w)
      }
      StepRef::ImplyXor(idx, vec, pf) => {
        write!(w, "i x {}  ", idx)?; vec.write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  l ")?; steps.write(w)?;
        }
        writeln!(w)
      }
      StepRef::FinalXor(idx, vec) => {
        write!(w, "f x {}  ", idx)?; vec.write(w)?; writeln!(w)
      }
      StepRef::OrigBnn(idx, vec, rhs, out) => {
        write!(w, "o b {}  ", idx)?; vec.write(w)?;
        if out == 0 {
          write!(w, " k {} 0", rhs)?;
        } else {
          write!(w, " k {} {} 0", rhs, out)?;
        }
        writeln!(w)
      }
      StepRef::AddBnn(idx, vec, rhs, out, pf) => {
        write!(w, "a b {}  ", idx)?; vec.write(w)?;
        if out == 0 {
          write!(w, " k {} 0", rhs)?; 
        } else {
          write!(w, " k {} {} 0", rhs, out)?;
        }
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  l ")?; steps.write(w)?;
        }
        writeln!(w)
      }
      StepRef::DelBnn(idx, vec, rhs, out) => {
        write!(w, "d b {}  ", idx)?; vec.write(w)?; write!(w, " k {} {} 0", rhs, out)?; writeln!(w)
      }
      StepRef::BnnImply(idx, vec, pf, uf) => {
        write!(w, "i {}  ", idx)?; vec.write(w)?;
        if let Some(ProofRef::LRAT(steps)) = pf {
          write!(w, "  b l ")?; steps.write(w)?;
          if let Some(ProofRef::Unit(units)) = uf {
            write!(w, " u ")?; units.write(w)?;
          }
        }
        writeln!(w)
      }
      StepRef::FinalBnn(idx, vec, rhs, out) => {
        write!(w, "f b {}  ", idx)?; vec.write(w)?; 
        if out == 0 {
          write!(w, " k {} 0", rhs)?;
        } else {
          write!(w, " k {} {} 0", rhs, out)?;
        }
        writeln!(w)
      }
    }
  }
}

impl<M> Serialize<M> for Step where for<'a> StepRef<'a>: Serialize<M> {
  fn write(&self, w: &mut impl ModeWrite<M>) -> io::Result<()> {
    self.as_ref().write(w)
  }
}

impl<'a> Serialize<Bin> for ElabStepRef<'a> {
  fn write(&self, w: &mut impl ModeWrite<Bin>) -> io::Result<()> {
    match *self {
      ElabStepRef::Comment(s) =>
        s.split('\0').try_for_each(|s| (b'c', s).write(w)),
      ElabStepRef::Orig(idx, vec) => (b'o', (idx, vec)).write(w),
      ElabStepRef::Add(idx, vec, steps) =>
        ((b'a', (idx, vec)), (b'l', steps)).write(w),
      ElabStepRef::Reloc(relocs) => (b'r', relocs).write(w),
      ElabStepRef::Del(idx) => (b'd', (idx, 0u8)).write(w),
      ElabStepRef::OrigXor(idx, vec) => ((b'o', (0u8, b'x')), (idx, vec)).write(w), 
      ElabStepRef::AddXor(idx, vec, steps, None) =>
        (((b'a', (0u8, b'x')), (idx, vec)), (b'l', steps)).write(w),
      ElabStepRef::AddXor(idx, vec, steps, Some(ProofRef::Unit(units))) =>
        (((b'a', (0u8, b'x')), (idx, vec)), ((b'l', steps), (b'u', units))).write(w),
      ElabStepRef::AddXor(idx, _, _, Some(ProofRef::LRAT(_))) =>
        panic!("add-xor step {}: duplicated 'l' step following 'a' 'x' 'l' step", idx),
      ElabStepRef::DelXor(idx) => ((b'd', (0u8, b'x')), (idx, 0u8)).write(w),
      ElabStepRef::Imply(idx, vec, steps) =>
        ((b'i', (idx, vec)), (b'l', steps)).write(w),
      ElabStepRef::ImplyXor(idx, vec, steps) =>
        (((b'i', (0u8, b'x')), (idx, vec)), (b'l', steps)).write(w),
      ElabStepRef::OrigBnn(idx, vec, rhs, out) => {
        ((b'o', (0u8, b'b')), (idx, vec)).write(w)?;
        if out == 0 {
          ((b'k', rhs), 0u8).write(w)?; 
        } else {
          (((b'k', rhs), out), 0u8).write(w)?;
        }
        write!(w, "")
      }
      ElabStepRef::AddBnn(idx, vec, rhs, out, steps) => {
        ((b'a', (0u8, b'b')), (idx, vec)).write(w)?;
        if out == 0 {
          ((b'k', rhs), 0u8).write(w)?;
        } else {
          (((b'k', rhs), out), 0u8).write(w)?;
        }
        (b'l', steps).write(w)
      }
      ElabStepRef::DelBnn(idx) => ((b'd', (0u8, b'b')), (idx, 0u8)).write(w),
      ElabStepRef::BnnImply(idx, vec, steps, None) =>
        ((b'i', (idx, vec)), ((b'b', (0u8, b'l')), steps)).write(w),
      ElabStepRef::BnnImply(idx, vec, steps, Some(ProofRef::Unit(units))) =>
        ((b'i', (idx, vec)), (((b'b', (0u8, b'l')), steps), (b'u', units))).write(w),
      ElabStepRef::BnnImply(idx, _, _, Some(ProofRef::LRAT(_))) =>
        panic!("bnn-imply step {}: duplicated 'l' step following 'i' 'b' 'l' step", idx),
    }
  }
}

impl<'a> Serialize<Ascii> for ElabStepRef<'a> {
  fn write(&self, w: &mut impl ModeWrite<Ascii>) -> io::Result<()> {
    match *self {
      ElabStepRef::Comment(s) => StepRef::Comment(s).write(w),
      ElabStepRef::Orig(idx, vec) => StepRef::Orig(idx, vec).write(w),
      ElabStepRef::Add(idx, vec, steps) =>
        StepRef::Add(idx, vec, Some(ProofRef::LRAT(steps))).write(w),
      ElabStepRef::Reloc(relocs) => StepRef::Reloc(relocs).write(w),
      ElabStepRef::Del(idx) => writeln!(w, "d {}", idx),
      ElabStepRef::OrigXor(idx, vec) => StepRef::OrigXor(idx, vec).write(w),
      ElabStepRef::AddXor(idx, vec, steps, uf) => 
        StepRef::AddXor(idx, vec, Some(ProofRef::LRAT(steps)), uf).write(w),
      ElabStepRef::DelXor(idx) => writeln!(w, "d x {}", idx),
      ElabStepRef::Imply(idx, vec, steps) =>
        StepRef::Imply(idx, vec, Some(ProofRef::LRAT(steps))).write(w),
      ElabStepRef::ImplyXor(idx, vec, steps) => 
        StepRef::ImplyXor(idx, vec, Some(ProofRef::LRAT(steps))).write(w),
      ElabStepRef::OrigBnn(idx, vec, rhs, out) => StepRef::OrigBnn(idx, vec, rhs, out).write(w),
      ElabStepRef::AddBnn(idx, vec, rhs, out, steps) => StepRef::AddBnn(idx, vec, rhs, out, Some(ProofRef::LRAT(steps))).write(w),
      ElabStepRef::DelBnn(idx) => writeln!(w, "d b {}", idx),
      ElabStepRef::BnnImply(idx, vec, steps, uf) =>
        StepRef::BnnImply(idx, vec, Some(ProofRef::LRAT(steps)), uf).write(w),
    }
  }
}

impl<M> Serialize<M> for ElabStep where for<'a> ElabStepRef<'a>: Serialize<M> {
  fn write(&self, w: &mut impl ModeWrite<M>) -> io::Result<()> {
    self.as_ref().write(w)
  }
}

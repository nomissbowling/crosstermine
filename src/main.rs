#![doc(html_root_url = "https://docs.rs/crosstermine/0.3.0")]
//! crosstermine mine for Rust with crossterm
//!

use std::error::Error;
use std::time;
use std::sync::mpsc;

use crossterm::event::Event;
use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::event::KeyCode::{self, Left, Down, Up, Right};
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use crossterm::style::Color;

use prayterm::{PrayTerm, Rgb, NopColor};

use minefield::{MineField, WR};

/// Term
pub struct Term<T> {
  /// colors
  pub colors: Vec<T>,
  /// PrayTerm
  pub tm: PrayTerm
}

/// trait WR for Term
impl<T: NopColor + Clone> WR<T> for Term<T> {
  /// wr
  fn wr(&mut self, x: u16, y: u16, st: u16,
    bgc: u16, fgc: u16, msg: &String) -> Result<(), Box<dyn Error>> {
    self.tm.wr(x, y, st, self.col(bgc), self.col(fgc), msg)?;
    Ok(())
  }

  /// reg
  fn reg(&mut self, c: Vec<T>) -> () {
    self.colors = c;
  }

  /// col
  fn col(&self, n: u16) -> T {
    self.colors[n as usize].clone()
  }
}

/// Term
impl<T: NopColor + Clone> Term<T> {
  /// constructor
  pub fn new(k: u16) -> Result<Self, Box<dyn Error>> {
    Ok(Term{colors: vec![], tm: PrayTerm::new(k)?})
  }
}

/// CrossTermine
pub struct CrossTermine {
  /// minefield
  pub m: MineField,
  /// view
  pub v: Term<Rgb>,
  /// time Instant
  pub t: time::Instant
}

/// Drop for CrossTermine
impl Drop for CrossTermine {
  /// destructor
  fn drop(&mut self) {
    self.v.tm.fin().expect("fin");
  }
}

/// CrossTermine
impl CrossTermine {
  /// constructor
  pub fn new(m: MineField) -> Result<Self, Box<dyn Error>> {
    let mut s = CrossTermine{m, v: Term::new(2)?, t: time::Instant::now()};
    let colors = [ // bgc fgc
      [96, 240, 32, 0], [32, 96, 240, 0], // closed
      [32, 96, 240, 0], [240, 192, 32, 0], // opened
      [240, 32, 96, 0], [240, 192, 32, 0] // ending
    ].into_iter().map(|c| Rgb(c[0], c[1], c[2])).collect::<Vec<_>>();
    s.v.reg(colors);
    s.v.tm.begin()?;
    s.m.reset_tick(&mut s.v)?;
    Ok(s)
  }

  /// status terminal
  pub fn status_t(&mut self, h: u16, st: u16,
    bgc: impl NopColor, fgc: impl NopColor) ->
    Result<(), Box<dyn Error>> {
    self.v.tm.wr(0, self.v.tm.h - h, st, bgc, fgc,
      &self.msg(self.v.tm.w, self.v.tm.h))?;
    Ok(())
  }

  /// status mouse
  pub fn status_p(&mut self, h: u16, st: u16,
    bgc: impl NopColor, fgc: impl NopColor, x: u16, y: u16) ->
    Result<(), Box<dyn Error>> {
    self.v.tm.wr(0, self.v.tm.h - h, st, bgc, fgc,
      &self.msg(x, y))?;
    Ok(())
  }

  /// status minefield
  pub fn status_m(&mut self, h: u16, st: u16,
    bgc: impl NopColor, fgc: impl NopColor) ->
    Result<(), Box<dyn Error>> {
    self.v.tm.wr(0, self.v.tm.h - h, st, bgc, fgc,
      &self.msg(self.m.m, self.m.s & 0x3fff))?;
    Ok(())
  }

  /// msg
  pub fn msg(&self, x: u16, y: u16) -> String {
    format!("({}, {}) {:?}", x, y, self.t.elapsed())
  }

  /// key
  pub fn key(&mut self, k: KeyEvent) -> bool {
    if k.kind != KeyEventKind::Press { return false; }
    let mut f = true;
    match k.code {
    Left | KeyCode::Char('h') => { self.m.left(); },
    Down | KeyCode::Char('j') => { self.m.down(); },
    Up | KeyCode::Char('k') => { self.m.up(); },
    Right | KeyCode::Char('l') => { self.m.right(); },
    KeyCode::Char(' ') => { self.m.click(); },
    _ => { f = false; }
    }
    f
  }
}

/// main
pub fn main() -> Result<(), Box<dyn Error>> {
  // let m = MineField::new(1, 1, 0);
  // let m = MineField::new(1, 1, 1);
  // let m = MineField::new(2, 2, 0);
  // let m = MineField::new(2, 2, 2);
  // let m = MineField::new(5, 4, 3);
  // let m = MineField::new(8, 8, 5);
  let m = MineField::new(16, 8, 12);
  // let m = MineField::new(80, 50, 12);
  let mut g = CrossTermine::new(m)?;
  g.status_t(1, 3, Color::Magenta, Rgb(240, 192, 32))?;

  let (_tx, rx) = g.v.tm.prepare_thread(g.m.ms)?;
  loop {
    // thread::sleep(ms);
    match rx.recv_timeout(g.m.ms) {
    Err(mpsc::RecvTimeoutError::Disconnected) => break, // not be arrived here
    Err(mpsc::RecvTimeoutError::Timeout) => { // idle
      g.status_m(3, 1, Rgb(192, 192, 192), Rgb(8, 8, 8))?;
      g.m.tick(&mut g.v)?;
    },
    Ok(ev) => {
      match ev {
      Ok(Event::Key(k)) => {
        match k {
        KeyEvent{kind: KeyEventKind::Press, state: _, code, modifiers} => {
          match (code, modifiers) {
          (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
          (KeyCode::Char('q'), _) => break,
          (KeyCode::Char('\x1b'), _) => break,
          (KeyCode::Esc, _) => break,
          _ => () // through down when kind == KeyEventKind::Press
          }
        },
        _ => () // through down when kind != KeyEventKind::Press
        }
        if g.key(k) { g.m.reset_tick(&mut g.v)?; }
        if g.m.is_end() { g.m.ending(&mut g.v)?; break; }
      },
      Ok(Event::Mouse(MouseEvent{kind, column: x, row: y, modifiers: _})) => {
        match kind {
        MouseEventKind::Moved => {
          g.status_p(5, 1, Color::Blue, Color::Yellow, x, y)?;
          if g.m.update_m(x, y) { g.m.reset_tick(&mut g.v)?; }
        },
        MouseEventKind::Down(MouseButton::Left) => {
          g.status_p(4, 1, Color::Cyan, Color::Green, x, y)?;
          if g.m.click() { g.m.reset_tick(&mut g.v)?; }
          if g.m.is_end() { g.m.ending(&mut g.v)?; break; }
        },
        _ => ()
        }
      },
      Ok(Event::Resize(_w, _h)) => {
        ()
      },
      _ => ()
      }
    }
    }
  }

  // handle.join()?;

  g.status_m(3, 1, Rgb(240, 192, 32), Rgb(192, 32, 240))?;
  g.status_t(2, 3, Rgb(255, 0, 0), Rgb(255, 255, 0))?;
  Ok(())
}

/// test with [-- --nocapture] or [-- --show-output]
#[cfg(test)]
mod tests {
  // use super::*;

  /// test a
  #[test]
  fn test_a() {
    assert_eq!(true, true);
  }
}

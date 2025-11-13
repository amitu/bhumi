use std::io::{stdout, Write, Result};
use std::time::Duration;
use std::env;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, SetForegroundColor, ResetColor},
    terminal::{self, ClearType},
};

const GRID_W: usize = 80;
const GRID_H: usize = 40;
const CENTER_WORD: &str = "bhumi";

fn draw_grid_at(top: u16, left: u16, stdout: &mut std::io::Stdout, is_too_small: bool) -> Result<()> {
    // Prebuild each text row for the 80-wide grid, placing CENTER_WORD in the vertical center
    let mut rows: Vec<String> = Vec::with_capacity(GRID_H);
    let word_len = CENTER_WORD.chars().count();
    let word_row = GRID_H / 2;
    let word_col_start = (GRID_W.saturating_sub(word_len)) / 2;

    for r in 0..GRID_H {
        // start with dots
        let mut line = vec!['.'; GRID_W];
        
        if r == word_row {
            for (i, ch) in CENTER_WORD.chars().enumerate() {
                if word_col_start + i < GRID_W {
                    line[word_col_start + i] = ch;
                }
            }
        }
        rows.push(line.into_iter().collect());
    }

    // Draw rows to terminal at (top,left)
    for (i, row) in rows.into_iter().enumerate() {
        let y = top.saturating_add(i as u16);
        execute!(stdout, cursor::MoveTo(left, y))?;
        if is_too_small {
            execute!(stdout, SetForegroundColor(Color::Red))?;
        }
        write!(stdout, "{}", row)?;
        if is_too_small {
            execute!(stdout, ResetColor)?;
        }
    }
    stdout.flush()?;
    Ok(())
}

fn print_raw_grid() -> Result<()> {
    let word_len = CENTER_WORD.chars().count();
    let word_row = GRID_H / 2;
    let word_col_start = (GRID_W.saturating_sub(word_len)) / 2;

    for r in 0..GRID_H {
        let mut line = vec!['.'; GRID_W];
        
        if r == word_row {
            for (i, ch) in CENTER_WORD.chars().enumerate() {
                if word_col_start + i < GRID_W {
                    line[word_col_start + i] = ch;
                }
            }
        }
        
        let row: String = line.into_iter().collect();
        println!("{}", row);
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    // Check for --raw flag
    if args.contains(&"--raw".to_string()) {
        return print_raw_grid();
    }

    let mut stdout = stdout();

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    // Initial draw
    let (mut term_w, mut term_h) = terminal::size()?;
    // main loop: redraw on resize or on a small timeout; quit on 'q' or Esc.
    loop {
        // Check if terminal is too small
        let is_too_small = term_w < GRID_W as u16 || term_h < GRID_H as u16;
        
        // compute top-left to center the GRID inside terminal
        let left = if term_w as i32 - GRID_W as i32 > 0 {
            ((term_w as usize - GRID_W) / 2) as u16
        } else {
            0u16
        };
        let top = if term_h as i32 - GRID_H as i32 > 0 {
            ((term_h as usize - GRID_H) / 2) as u16
        } else {
            0u16
        };

        // clear area (clear entire screen to keep simple)
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        draw_grid_at(top, left, &mut stdout, is_too_small)?;

        // wait for events with a small timeout so we react to resize/keys
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Char('q') || k.code == KeyCode::Esc {
                        break;
                    }
                }
                Event::Resize(w, h) => {
                    term_w = w;
                    term_h = h;
                    // loop will redraw with new size
                }
                _ => {}
            }
        } else {
            // timeout expired -> loop and redraw (keeps center even if terminal changed without Resize event)
            let (w, h) = terminal::size()?;
            term_w = w; term_h = h;
        }
    }

    // restore terminal
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

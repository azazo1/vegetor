use crossterm::event::{KeyEvent, Event, KeyCode, KeyModifiers};
use std::io;
use crate::editor::editarea::{Area, EditArea};
use crate::editor::terminal::Terminal;

mod editarea;
mod terminal;
mod buffer;
mod error;

pub struct Editor {
    edit_area: EditArea,
    // status_bar: StatusBar,
    terminal: Terminal,
}

impl Editor {
    pub fn build() -> io::Result<Editor> {
        let mut terminal = Terminal::new();
        terminal.initialize()?;
        let mut edit_area = EditArea::new();
        let terminal_size = terminal.size()?;
        edit_area.configure_area(Area::new(0, 0, terminal_size.width, terminal_size.height)); // todo 改.
        Ok(Editor {
            edit_area,
            // status_bar: StatusBar::new(&terminal),
            terminal,
        })
    }

    pub fn run(&mut self) -> error::Result<()> {
        loop {
            self.edit_area.print_welcome_to(&mut self.terminal)?;
            self.terminal.flush()?;
            let evt = self.terminal.read_event_blocking();
            if let Ok(Event::Key(KeyEvent { state, code, kind, modifiers })) = evt {
                if code == KeyCode::Char('q') && modifiers == KeyModifiers::CONTROL {
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.terminal.destruct();
    }
}
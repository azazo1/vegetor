use crossterm::event::{KeyEvent, Event, KeyCode, KeyModifiers, KeyEventKind};
use crate::editor::editarea::{Area, EditArea};
use crate::error;
use crate::editor::terminal::Terminal;

pub use crate::editor::terminal::{Size, Location};

mod editarea;
mod terminal;
mod buffer;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum State {
    Welcoming,
    Editing,
    Exiting,
}

pub struct Editor {
    edit_area: EditArea,
    // status_bar: StatusBar,
    terminal: Terminal,
    state: State,
}

impl Editor {
    fn panic_handler(_info: &std::panic::PanicHookInfo) {
        let _ = Terminal::new().destruct(); // 唯一的 Terminal 二次构建情况, 我实在想不出来到底怎么合适的使用唯一那个 Terminal.
    }

    pub fn build() -> error::Result<Editor> {
        let raw_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Editor::panic_handler(info);
            raw_hook(info);
        }));

        let mut terminal = Terminal::new();
        terminal.initialize()?;
        let terminal_size = terminal.size()?;
        let mut edit_area = EditArea::new();
        edit_area.configure_area(Area::new(0, 0, terminal_size.width, terminal_size.height)); // todo 改.
        #[cfg(debug_assertions)] {
            edit_area.load_buffer("welcome.txt")?;
        }
        edit_area.load_welcome("welcome.txt")?;
        Ok(Editor {
            edit_area,
            // status_bar: StatusBar::new(&terminal),
            terminal,
            state: State::Welcoming,
        })
    }

    pub fn run(&mut self) -> error::Result<()> {
        while self.state != State::Exiting {
            if self.edit_area.need_printing() {
                match self.state {
                    State::Welcoming => self.edit_area.print_welcome_to(&mut self.terminal)?,
                    State::Editing => self.edit_area.print_to(&mut self.terminal)?,
                    _ => {}
                }
                self.edit_area.unset_need_printing();
            }
            self.terminal.flush()?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn handle_event(&mut self) -> error::Result<()> {
        let evt = self.terminal.read_event_blocking();
        match evt {
            Ok(Event::Key(KeyEvent { state, code, kind, modifiers })) => {
                if kind == KeyEventKind::Press {
                    match code {
                        KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => {
                            self.state = State::Exiting;
                        }
                        _ => {
                            if self.state == State::Welcoming {
                                self.state = State::Editing; // 有按键按下就进入 Editing, 其余不做任何动作.
                                self.edit_area.set_need_printing();
                            } else if let Ok(caret_move) = code.try_into() {
                                self.edit_area.move_caret(caret_move)?;
                            }
                            // todo
                        }
                    }
                }
            }
            Ok(Event::Resize(columns, rows)) => {
                let columns = columns as usize;
                let rows = rows as usize;
                self.edit_area.configure_area(Area::new(0, 0, columns, rows));
            }
            _ => {}
        }
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.terminal.destruct();
    }
}

//noinspection DuplicatedCode
#[cfg(test)]
mod split_screen_test {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use crate::editor::editarea::{Area, EditArea};
    use crate::editor::State;
    use crate::editor::terminal::Terminal;
    use crate::error;

    pub struct Editor {
        edit_area: EditArea,
        edit_area_2: EditArea,
        // status_bar: StatusBar,
        terminal: Terminal,
        state: State,
    }

    impl Editor {
        fn panic_handler(_info: &std::panic::PanicHookInfo) {
            let _ = Terminal::new().destruct(); // 唯一的 Terminal 二次构建情况, 我实在想不出来到底怎么合适的使用唯一那个 Terminal.
        }

        pub fn build() -> error::Result<Editor> {
            let raw_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                Editor::panic_handler(info);
                raw_hook(info);
            }));

            let mut terminal = Terminal::new();
            terminal.initialize()?;
            let terminal_size = terminal.size()?;
            let horizontal_sep = terminal_size.width / 2;
            let mut edit_area = EditArea::new();
            edit_area.configure_area(Area::new(0, 0, horizontal_sep, terminal_size.height)); // todo 改.
            #[cfg(debug_assertions)] {
                edit_area.load_buffer("welcome.txt")?;
            }
            edit_area.load_welcome("welcome.txt")?;

            // for test only.
            let mut edit_area_2 = EditArea::new();
            edit_area_2.configure_area(Area::new(horizontal_sep, 0, terminal_size.width - horizontal_sep, terminal_size.height));
            edit_area_2.load_buffer("Cargo.lock")?;
            edit_area_2.load_welcome("welcome.txt")?;

            Ok(Editor {
                edit_area,
                edit_area_2,
                // status_bar: StatusBar::new(&terminal),
                terminal,
                state: State::Welcoming,
            })
        }

        pub fn run(&mut self) -> error::Result<()> {
            while self.state != State::Exiting {
                if self.edit_area.need_printing() {
                    match self.state {
                        State::Welcoming => self.edit_area.print_welcome_to(&mut self.terminal)?,
                        State::Editing => self.edit_area.print_to(&mut self.terminal)?,
                        _ => {}
                    }
                    self.edit_area.unset_need_printing();
                }
                if self.edit_area_2.need_printing() {
                    match self.state {
                        State::Welcoming => self.edit_area_2.print_welcome_to(&mut self.terminal)?,
                        State::Editing => self.edit_area_2.print_to(&mut self.terminal)?,
                        _ => {}
                    }
                    self.edit_area_2.unset_need_printing();
                }
                self.terminal.flush()?;
                self.handle_event()?;
            }
            Ok(())
        }

        fn handle_event(&mut self) -> error::Result<()> {
            let evt = self.terminal.read_event_blocking();
            match evt {
                Ok(Event::Key(KeyEvent { state, code, kind, modifiers })) => {
                    if kind == KeyEventKind::Press {
                        match code {
                            KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => {
                                self.state = State::Exiting;
                            }
                            _ => {
                                if self.state == State::Welcoming {
                                    self.state = State::Editing; // 有按键按下就进入 Editing, 其余不做任何动作.
                                    self.edit_area.set_need_printing();
                                    self.edit_area_2.set_need_printing();
                                } else if let Ok(caret_move) = code.try_into() {
                                    self.edit_area.move_caret(caret_move)?;
                                }
                                // todo
                            }
                        }
                    }
                }
                Ok(Event::Resize(columns, rows)) => {
                    let columns = columns as usize;
                    let rows = rows as usize;
                    let horizontal_sep = columns / 2;
                    self.edit_area.configure_area(Area::new(0, 0, horizontal_sep, rows));
                    self.edit_area_2.configure_area(Area::new(horizontal_sep, 0, columns - horizontal_sep, rows))
                }
                _ => {}
            }
            Ok(())
        }
    }
    
    #[test]
    fn split_screen_run() {
        Editor::build().unwrap().run().unwrap();
    }
}
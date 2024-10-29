use std::io;
use std::io::BufReader;
use crate::editor::editarea::EditArea;
use crate::editor::terminal::Terminal;

mod editarea;
mod terminal;
mod buffer;

// struct Editor {
//     edit_area: EditArea,
//     status_bar: StatusBar,
    // terminal: Terminal,
// }

// impl Editor {
//     pub fn build() -> io::Result<Editor> {
//         let mut terminal = Terminal::new();
//         Ok(Editor {
            // edit_area: EditArea::new(&terminal),
            // status_bar: StatusBar::new(&terminal),
            // terminal,
        // })
    // }
// }
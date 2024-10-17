use std::io;
mod editarea;
mod terminal;
mod buffer;

// struct Editor {
//     edit_area: EditArea,
//     status_bar: StatusBar,
//     terminal: Terminal,
// }
//
// impl Editor {
//     pub fn build() -> io::Result<Editor> {
//         let mut terminal = Terminal::new()?;
//         Ok(Editor {
//             edit_area: EditArea::new(&terminal),
//             status_bar: StatusBar::new(&terminal),
//             terminal,
//         })
//     }
// }
use crossterm::event::KeyCode;
use std::path;
use std::io;
use crate::{error, CharsCount};
use crate::editor::buffer::Buffer;
use crate::editor::terminal::{Location, Size, Terminal};


#[derive(Debug, Eq, PartialEq, Copy, Clone)]
/// caret 的各种移动方式.
pub enum CaretMove {
    /// caret 向上一行.
    Up,
    /// caret 向下一行.
    Down,
    /// caret 向左一个字符.
    Left,
    /// caret 向右一个字符.
    Right,
    /// caret 移动到下一个单词开始.
    NextWord,
    /// caret 移动到上一个单词开始.
    PrevWord,
    /// caret 移动到行首.
    StartOfLine,
    /// caret 移动到行末.
    EndOfLine,
    /// caret 移动到下一页.
    PageUp,
    /// caret 移动到上一页.
    PageDown,
    /// caret 移动到文本初始.
    GlobalStart,
    /// caret 移动到文本末尾.
    GlobalEnd,
    /// caret 移动到跳转前的位置.
    ///
    /// # Notice
    ///
    /// `跳转` 不包括行内的 caret 移动.
    PrevTrace,
    /// caret 移动到跳转后的位置.
    ///
    /// # Notice
    ///
    /// `跳转` 不包括行内的 caret 移动.
    NextJump,
}

impl TryFrom<KeyCode> for CaretMove {
    type Error = ();

    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        Ok(match value {
            KeyCode::Left => CaretMove::Left,
            KeyCode::Right => CaretMove::Right,
            KeyCode::Up => CaretMove::Up,
            KeyCode::Down => CaretMove::Down,
            _ => { Err(())? }
        })
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Area {
    left_top: Location,
    size: Size,
}

impl Area {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Area {
        Area {
            left_top: Location::new(x, y),
            size: Size::new(w, h),
        }
    }

    pub fn x(&self) -> usize {
        self.left_top.x
    }

    pub fn y(&self) -> usize {
        self.left_top.y
    }

    pub fn width(&self) -> usize {
        self.size.width
    }

    pub fn height(&self) -> usize {
        self.size.height
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn left_top(&self) -> Location {
        self.left_top
    }

    pub fn center(&self) -> Location {
        Location::new(self.width() / 2 + self.x(), self.height() / 2 + self.y())
    }
}

pub struct EditArea {
    buffer: Buffer,
    /// 在终端中的打印区域, 打印的 buffer 内容不会超出此区域.
    display_area: Area,
    /// buffer 显示的偏移量, 对 welcome_buffer 无效. todo 实现, 注意 caret 移动和字符的增删改时此量的变化.
    buffer_display_offset: Location,
    welcome_buffer: Buffer,
    /// 标志画面是否需要重绘到终端上.
    need_printing: bool,
}

impl EditArea {
    /// 更改显示区域的大小, 在 [`EditArea::print_to`] 和 [`EditArea::print_to_center`] 之前需要调用以确保正确显示.
    pub fn configure_area(&mut self, new_area: Area) {
        self.display_area = new_area;
        self.set_need_printing();
        // todo 管理 buffer_display_offset.
    }

    /// 用于标识已经完成显示的步骤, 只由外部调用.
    pub fn unset_need_printing(&mut self) {
        self.need_printing = false;
    }

    pub fn need_printing(&self) -> bool {
        self.need_printing
    }

    /// 标记自身需要重绘, 可由内部调用也可由外部调用.
    pub fn set_need_printing(&mut self) {
        self.need_printing = true;
    }

    /// 把 buffer 内容打印到终端.
    ///
    /// # Arguments
    ///
    /// * `terminal`: 终端对象.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>`:
    ///     - `Ok(())`: 打印成功.
    ///     - `Err(Error)`: 打印尺寸不符合要求或者 io 错误.
    pub fn print_to(&self, terminal: &mut Terminal) -> io::Result<()> {
        terminal.hide_cursor()?;
        for row in 0..self.display_area.height() {
            // 清空在显示区域内的内容.
            terminal.move_cursor_to(Location::new(self.display_area.x(), self.display_area.y() + row))?;
            terminal.print(" ".repeat(self.display_area.width()))?;

            terminal.move_cursor_to(Location::new(self.display_area.x(), self.display_area.y() + row))?;
            match self.buffer.get(row + self.buffer_display_offset.y) {
                Some(line) => {
                    let len = self.display_area.width().min(line.chars_count() - self.buffer_display_offset.x);
                    // .min(line.width_cjk() - self.buffer_display_offset.x) // todo 测试 unicode width 是否准确, 多拿中文测.
                    terminal.print(&line[
                        self.buffer_display_offset.x
                            ..(self.buffer_display_offset.x + len)
                        ])?;
                }
                None => {}
            };
        }
        let caret = self.buffer.caret();
        let offset_x = caret.x.saturating_sub(self.buffer_display_offset.x).min(self.display_area.width());
        let offset_y = caret.y.saturating_sub(self.buffer_display_offset.y).min(self.display_area.height());
        terminal.move_cursor_to(Location::new(self.display_area.x() + offset_x, self.display_area.y() + offset_y))?;
        terminal.show_cursor()?;
        Ok(())
    }

    /// 把参数 welcome_buffer 内容打印到终端, 和 [`EditArea::print_to`] 相似, 但是文本内容在 area 中横向纵向居中显示.
    ///
    /// # Errors
    ///
    /// - [`error::Error::IOError`]: 见 [`io::Error`](io::Error).
    /// - [`error::Error::BufferSizeExceeds`]: welcome_buffer 的横向长度或者纵向长度超过了可打印范围.
    pub fn print_welcome_to(&self, terminal: &mut Terminal) -> error::Result<()> {
        let buffer_size = self.welcome_buffer.size();
        if !(self.display_area.size() > self.welcome_buffer.size()) { // 偏序比较.
            return Err(error::Error::BufferSizeExceeds {
                buffer_size,
                area_size: self.display_area.size(),
            });
        }

        terminal.hide_cursor()?;
        let (start_column, start_row): (usize, usize) = self.display_area.center().into();
        let start_row = start_row - buffer_size.height / 2;
        for row_offset in 0..buffer_size.height { // 这里已经确认了 welcome_buffer 高度比显示高度小了.
            let row = row_offset + start_row;
            let line = self.welcome_buffer.get(row_offset).unwrap();
            let column = start_column - line.chars_count() / 2;
            // 清除区域内的字符.
            terminal.move_cursor_to(Location::new(self.display_area.x(), row))?;
            terminal.print(" ".repeat(self.display_area.width()))?;
            // 居中显示
            terminal.move_cursor_to(Location::new(column, row))?;
            terminal.print(line)?;
        }
        terminal.move_cursor_to(self.display_area.left_top())?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn new() -> EditArea {
        EditArea {
            buffer_display_offset: Location::new(0, 0),
            display_area: Area::new(0, 0, 0, 0),
            buffer: Buffer::new(),
            welcome_buffer: Buffer::new(),
            need_printing: false,
        }
    }

    pub fn load_welcome(&mut self, welcome_file: impl AsRef<path::Path>) -> error::Result<()> {
        self.welcome_buffer.load(welcome_file)
    }

    pub fn load_buffer(&mut self, file: impl AsRef<path::Path>) -> error::Result<()> {
        self.buffer.load(file)
    }
}

impl EditArea {
    fn move_caret_left(&mut self) -> error::Result<()> {
        let mut caret = self.buffer.caret();
        if caret.x == 0 {
            if caret.y > 0 {
                match self.buffer.get(caret.y - 1) {
                    Some(line) => {
                        caret.x = line.chars_count(); // 移动到行末, 也就是最后一个字符的后面.
                        caret.y -= 1;
                    }
                    None => {
                        // 当前是第一行, 或者没有内容.
                        caret.y = 0;
                    }
                }
            }
        } else {
            caret.x -= 1;
        }
        self.move_caret_to(caret)
    }

    fn move_caret_right(&mut self) -> error::Result<()> {
        let mut caret = self.buffer.caret();
        let line = self.buffer.get(caret.y);
        match line {
            None => {
                // 到了末尾行.
                caret.x = 0;
                caret.y = self.buffer.len();
            }
            Some(line) => {
                if caret.x == line.chars_count() {
                    // 到了行末.
                    if self.buffer.get(caret.y + 1).is_some() {
                        // 下一行有内容.
                        caret.x = 0;
                        caret.y += 1;
                    }
                } else {
                    caret.x += 1;
                }
            }
        }
        self.move_caret_to(caret)
    }

    fn move_caret_up(&mut self) -> error::Result<()> {
        // todo 添加对非 ascii 字符宽度的字符的支持, 比如适配中文的宽度.
        let mut caret = self.buffer.caret();
        if caret.y != 0 {
            let prev_line = self.buffer.get(caret.y - 1);
            match prev_line {
                Some(line) => {
                    caret.y -= 1;
                    caret.x = caret.x.min(line.chars_count());
                }
                None => { /* 可能是没有内容, 不变化 y 值.*/ }
            }
            // caret.y == 0 不用考虑, 因为是向上.
        }
        self.move_caret_to(caret)
    }

    fn move_caret_down(&mut self) -> error::Result<()> {
        let mut caret = self.buffer.caret();
        let next_line = self.buffer.get(caret.y + 1);
        match next_line {
            Some(line) => {
                caret.y += 1;
                caret.x = caret.x.min(line.chars_count());
            }
            None => {}
        }
        self.move_caret_to(caret)
    }

    /// 移动 caret, 会根据 display_area 协调  buffer_display_offset 以使 buffer
    /// 的显示内容随 caret 移动而变化.
    ///
    /// # Errors
    ///
    /// - [`Error::CaretOutOfRange`]: caret 移动到的位置不合理.
    pub fn move_caret_to(&mut self, loc: Location) -> error::Result<()> {
        // todo 检测 caret 移动的位置是否合理.
        // todo 变化 self.buffer_display_offset,
        self.buffer.seek_unchecked(loc);
        // 由于此处没有持有 terminal 引用, 不管 self.buffer_display_offset 是否发生了变化, 都需要 set_need_printing.
        self.set_need_printing();
        Ok(())
    }

    /// 对 caret 执行特定的移动操作.
    /// 具体操作见 [`CaretMove`].
    pub fn move_caret(&mut self, caret_move: CaretMove) -> error::Result<()> {
        match caret_move {
            CaretMove::Left => self.move_caret_left()?,
            CaretMove::Right => self.move_caret_right()?,
            CaretMove::Up => self.move_caret_up()?,
            CaretMove::Down => self.move_caret_down()?,
            _ => {
                todo!()
            }
        }
        Ok(())
    }
}
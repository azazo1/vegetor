use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io;
use crate::{error, CharsCount};
use crate::editor::buffer::Buffer;
use crate::editor::terminal::{Location, Size, Terminal};

/// caret 上下移动时, 显示区域发生滚动会尽可能不会让 caret 直接贴住可显示范围的边缘, 而是保留一定的可视行数预览后/前几行.
/// 此变量用于设置要保留 caret 与画面在竖直上的距离的行数.
///
/// # 特殊情况
///
/// - 如果可显示范围的高度不足 `2 * VERTICAL_PADDING`, 那么此参数无效, 页面滚动将按照.
/// - 如果文本内容高度大于显示区域高度, 在 caret 即将到达文本底部时, 文本的末尾行最多上升到显示区域的最后一行,
/// 而不是继续向上产生显示区域的空白行.
const VERTICAL_PADDING: usize = 3;

/// caret 移动时与水平边缘的距离, 基本同理于 [`VERTICAL_PADDING`].
const HORIZONTAL_PADDING: usize = 5;

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
    LineStart,
    /// caret 移动到行末.
    LineEnd,
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
    NextTrace,
}

impl TryFrom<&KeyEvent> for CaretMove {
    type Error = ();

    fn try_from(value: &KeyEvent) -> Result<Self, Self::Error> {
        let modifiers = value.modifiers;
        Ok(match value.code {
            KeyCode::Left if modifiers == KeyModifiers::NONE => CaretMove::Left,
            KeyCode::Right if modifiers == KeyModifiers::NONE => CaretMove::Right,
            KeyCode::Up if modifiers == KeyModifiers::NONE => CaretMove::Up,
            KeyCode::Down if modifiers == KeyModifiers::NONE => CaretMove::Down,

            KeyCode::Left if modifiers == KeyModifiers::CONTROL => CaretMove::PrevWord,
            KeyCode::Right if modifiers == KeyModifiers::CONTROL => CaretMove::NextWord,

            KeyCode::Left if modifiers == KeyModifiers::CONTROL | KeyModifiers::ALT => CaretMove::PrevTrace,
            KeyCode::Right if modifiers == KeyModifiers::CONTROL | KeyModifiers::ALT => CaretMove::NextTrace,

            KeyCode::Home if modifiers == KeyModifiers::NONE => CaretMove::LineStart,
            KeyCode::End if modifiers == KeyModifiers::NONE => CaretMove::LineEnd,

            KeyCode::Home if modifiers == KeyModifiers::CONTROL => CaretMove::GlobalStart,
            KeyCode::End if modifiers == KeyModifiers::CONTROL => CaretMove::GlobalEnd,

            KeyCode::PageUp => CaretMove::PageUp,
            KeyCode::PageDown => CaretMove::PageDown,
            _ => { Err(())? }
        })
    }
}

impl TryFrom<KeyEvent> for CaretMove {
    type Error = ();

    fn try_from(value: KeyEvent) -> Result<Self, Self::Error> {
        (&value).try_into()
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

    #[inline]
    pub fn x(&self) -> usize {
        self.left_top.x
    }

    #[inline]
    pub fn y(&self) -> usize {
        self.left_top.y
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.size.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.size.height
    }

    #[inline]
    pub fn size(&self) -> Size {
        self.size
    }

    #[inline]
    pub fn left_top(&self) -> Location {
        self.left_top
    }

    #[inline]
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
    /// 把 buffer 的 caret 坐标转换成 cursor 坐标.
    fn get_cursor(&self) -> Location {
        let caret = self.buffer.caret();
        let offset_x = caret.x.saturating_sub(self.buffer_display_offset.x).min(self.display_area.width());
        let offset_y = caret.y.saturating_sub(self.buffer_display_offset.y).min(self.display_area.height());
        Location::new(offset_x, offset_y)
    }

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
                    let len = self.display_area.width()
                        // 这里 line.chars_count() 可能小于 offset.x, 因为视角移动到了太右侧.
                        .min(line.chars_count().saturating_sub(self.buffer_display_offset.x));
                    // .min(line.width_cjk() - self.buffer_display_offset.x) // todo 测试 unicode width 是否准确, 多拿中文测.
                    if len > 0 {
                        terminal.print(&line[
                            self.buffer_display_offset.x
                                ..(self.buffer_display_offset.x + len)
                            ])?;
                    }
                }
                None => {}
            };
        }
        let Location { x: offset_x, y: offset_y } = self.get_cursor();
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

    pub(crate) fn get_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub(crate) fn get_welcome_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.welcome_buffer
    }

    /// 根据 buffer 的 caret 来更新 buffer_display_offset.
    ///
    /// 适合在 buffer 被修改之后调用来让画面同步 caret 的变化.
    ///
    /// # Returns
    ///
    /// 返回 offset 是否发生变化, 即画面是否需要改变.
    pub fn update_display_offset(&mut self) -> bool {
        let raw_offset = self.buffer_display_offset;
        let caret = self.buffer.caret();
        // 检测 caret 是否在竖直方向移动较大.
        let v_padding = if self.display_area.height() >= 2 * VERTICAL_PADDING { VERTICAL_PADDING } else { 0 };
        let y_display = caret.y as isize - self.buffer_display_offset.y as isize; // caret 在显示区域的 y 坐标.
        if y_display >= (self.display_area.height() as isize - v_padding as isize) {
            // 向下较多.
            let bottom = (caret.y + v_padding)
                .min(self.buffer.lines_num() /*让最后一行最高上升到最底边(只在文本高高度大于显示区域的时候)*/);
            self.buffer_display_offset.y = bottom.saturating_sub(self.display_area.height());
        } else if y_display < v_padding as isize {
            // 向上较多.
            if caret.y >= v_padding {
                self.buffer_display_offset.y = caret.y - v_padding;
            } else {
                self.buffer_display_offset.y = 0;
            }
        }
        // 竖直方向的补充检查: 如果文本高度大于显示高度, 但是最后一行浮空(高于显示区域最后一行)了, 就让文本最后一行贴底.
        // 此检查针对用户拉高终端的操作.
        if self.buffer.lines_num() > self.display_area.height() {
            // 最后一行之后一行在显示区域的竖直方向从第一行开始的偏移量.
            let bottom_offset_from_display = self.buffer.lines_num() - self.buffer_display_offset.y;
            // 如果浮空了就贴底, 通过 saturating_sub 暗含了和 0 的比较.
            self.buffer_display_offset.y -= self.display_area.height().saturating_sub(bottom_offset_from_display);
        }
        // 检测 caret 是否在水平方向移动较大. 
        let h_padding = if self.display_area.width() >= 2 * HORIZONTAL_PADDING { HORIZONTAL_PADDING } else { 0 };
        let x_display = caret.x as isize - self.buffer_display_offset.x as isize; // caret 在显示区域的 x 坐标.
        if x_display < h_padding as isize {
            if caret.x < h_padding {
                self.buffer_display_offset.x = 0;
            } else {
                self.buffer_display_offset.x = caret.x - h_padding;
            }
        } else if x_display > (self.display_area.width() - h_padding) as isize {
            let right = caret.x + h_padding;
            // 这里不需要行末贴边, 让用户感知到这行后面是空的.
            self.buffer_display_offset.x = right.saturating_sub(self.display_area.width());
        }
        self.buffer_display_offset != raw_offset
    }
}

impl EditArea {
    fn move_caret_left(&mut self) -> Location {
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
        self.move_caret_to(caret).unwrap()
    }

    fn move_caret_right(&mut self) -> Location {
        let mut caret = self.buffer.caret();
        let line = self.buffer.get_current_line();
        match line {
            None => {
                // 到了末尾行.
                caret.x = 0;
                caret.y = self.buffer.lines_num();
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
        self.move_caret_to(caret).unwrap()
    }

    fn move_caret_up(&mut self) -> Location {
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
        self.move_caret_to(caret).unwrap()
    }

    fn move_caret_down(&mut self) -> Location {
        let mut caret = self.buffer.caret();
        let next_line = self.buffer.get(caret.y + 1);
        match next_line {
            Some(line) => {
                caret.y += 1;
                caret.x = caret.x.min(line.chars_count());
            }
            None => {}
        }
        self.move_caret_to(caret).unwrap()
    }

    fn move_caret_to_global_end(&mut self) -> Location {
        if self.buffer.lines_num() != 0 {
            let caret = Location::new(
                self.buffer.get(self.buffer.lines_num() - 1).unwrap().len(),
                self.buffer.lines_num() - 1,
            );
            self.move_caret_to(caret).unwrap()
        } else {
            Location::new(0, 0)
        }
    }

    fn move_caret_to_global_start(&mut self) -> Location {
        self.move_caret_to(Location::new(0, 0)).unwrap()
    }

    fn move_caret_to_next_word(&mut self) -> Location {
        let mut reader = self.buffer.get_reader().unwrap();
        let ok = reader.skip_until_blank().is_ok() && reader.skip_until_not_blank().is_ok();
        if ok {
            self.move_caret_to(reader.caret()).unwrap()
        } else {
            self.move_caret_to_global_end()
        }
    }

    fn move_caret_to_prev_word(&mut self) -> Location {
        let mut reader = self.buffer.get_reader().unwrap();
        let ok = match reader.peek() {
            Some(current_char) if !current_char.is_whitespace() => {
                reader.back_until_blank().is_ok()
                    && reader.back_until_not_blank().is_ok()
                    && reader.back_until_blank().is_ok()
            }
            Some(_) | None => { // None 表示 caret 在 buffer 末尾.
                reader.back_until_not_blank().is_ok()
                    && reader.back_until_blank().is_ok()
            }
        };
        if ok {
            self.move_caret_to(reader.caret()).unwrap()
        } else {
            self.move_caret_to_global_start()
        }
    }

    fn move_caret_to_line_end(&mut self) -> Location {
        let line = self.buffer.get_current_line().unwrap();
        let mut caret = self.buffer.caret();
        caret.x = line.len();
        self.move_caret_to(caret).unwrap()
    }

    fn move_caret_to_line_start(&mut self) -> Location {
        let mut caret = self.buffer.caret();
        caret.x = 0;
        self.move_caret_to(caret).unwrap()
    }

    /// 移动 caret, 会根据 display_area 协调  buffer_display_offset 以使 buffer
    /// 的显示内容随 caret 移动而变化.
    ///
    /// # Errors
    ///
    /// - [`Error::CaretOutOfRange`]: caret 移动到的位置不合理.
    ///
    /// # Returns
    ///
    /// - 移动到的 caret 在屏幕中的坐标, 也就是 cursor: [`Location`].
    pub fn move_caret_to(&mut self, caret: Location) -> error::Result<Location> {
        // 检测 caret 移动的位置是否合理.
        self.buffer.check_caret(caret)?;
        self.buffer.seek_unchecked(caret);
        if self.update_display_offset() {
            self.set_need_printing();
        }
        // 通过返回 caret 在屏幕中的位置来通知调用者对 cursor 进行更新而无需绘制其他的内容.
        Ok(self.get_cursor())
    }

    /// 对 caret 执行特定的移动操作.
    /// 具体操作见 [`CaretMove`].
    ///
    /// # Returns
    ///
    /// - 移动 caret 后, 屏幕 cursor 应该移动到的位置.
    pub fn move_caret(&mut self, caret_move: CaretMove) -> Location {
        match caret_move {
            CaretMove::Left => self.move_caret_left(),
            CaretMove::Right => self.move_caret_right(),
            CaretMove::Up => self.move_caret_up(),
            CaretMove::Down => self.move_caret_down(),
            CaretMove::NextWord => self.move_caret_to_next_word(),
            CaretMove::PrevWord => self.move_caret_to_prev_word(),
            CaretMove::GlobalEnd => self.move_caret_to_global_end(),
            CaretMove::GlobalStart => self.move_caret_to_global_start(),
            CaretMove::LineEnd => self.move_caret_to_line_end(),
            CaretMove::LineStart => self.move_caret_to_line_start(),
            _ => {
                todo!("{:?}.", caret_move)
            }
        } // CaretOutOfRange 在这里不会出现, 因为都是计算好了的坐标移动.
    }
}


// todo 解决调整终端大小的时候 cursor 显示在右下角的问题.
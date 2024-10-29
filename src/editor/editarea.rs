use std::io;
use crate::editor::buffer::Buffer;
use crate::editor::terminal::{Location, Size, Terminal};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IOError occurred: {0:?}")]
    IOError(#[from] io::Error),
    #[error("Buffer size {buffer_size:?} exceeds the display area size {area_size:?}.")]
    BufferSizeExceeds { buffer_size: Size, area_size: Size },
    #[error("Carpet out of range.")]
    CarpetOutOfRange,
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
    /// buffer 显示的偏移量, 对 welcome_buffer 无效. todo 实现, 注意 carpet 移动和字符的增删改时此量的变化.
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

    /// 只由外部调用, 用于标识已经完成显示的步骤.
    pub fn unset_need_printing(&mut self) {
        self.need_printing = false;
    }

    pub fn need_printing(&mut self) -> bool {
        self.need_printing
    }

    /// 仅由内部调用, 标记自身需要重绘.
    fn set_need_printing(&mut self) {
        self.need_printing = true;
    }

    /// 移动 carpet, 会根据 display_area 协调  buffer_display_offset 以使 buffer
    /// 的显示内容随 carpet 移动而变化.
    ///
    /// # Errors
    ///
    /// - [`Error::CarpetOutOfRange`]: carpet 移动到的位置不合理.
    pub fn move_carpet_to(&mut self, loc: Location) -> Result<(), Error> {
        self.set_need_printing();
        todo!()
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
            terminal.move_cursor_to(Location::new(self.display_area.x(), self.display_area.y() + row))?;
            let print_len = match self.buffer.get(row + self.buffer_display_offset.y) {
                Some(line) => {
                    let len = self.display_area.width().min(line.len() - self.buffer_display_offset.x);
                    // .min(line.width_cjk() - self.buffer_display_offset.x) // todo 测试 unicode width 是否准确, 多拿中文测.
                    terminal.print(&line[
                        self.buffer_display_offset.x
                            ..(self.buffer_display_offset.x + len)
                        ])?;
                    len
                }
                None => { 0 }
            };
            // 补充打印结尾的空格, 清除原来的内容, 注意不能清除完整的一行, 不然其他部分的内容会被清除.
            terminal.print(" ".repeat(self.display_area.width() - print_len))?;
        }
        let carpet = self.buffer.carpet();
        let offset_x = carpet.x.saturating_sub(self.buffer_display_offset.x).min(self.display_area.width());
        let offset_y = carpet.y.saturating_sub(self.buffer_display_offset.y).min(self.display_area.height());
        terminal.move_cursor_to(Location::new(self.display_area.x() + offset_x, self.display_area.y() + offset_y))?;
        terminal.show_cursor()?;
        Ok(())
    }

    /// 把参数 welcome_buffer 内容打印到终端, 和 [`EditArea::print_to`] 相似, 但是文本内容在 area 中横向纵向居中显示.
    ///
    /// # Errors
    ///
    /// - Error::IOError: 见 [`io::Error`](io::Error).
    /// - Error::BufferSizeExceeds: welcome_buffer 的横向长度或者纵向长度超过了可打印范围.
    fn print_welcome_to(&self, terminal: &mut Terminal) -> Result<(), Error> {
        let buffer_size = self.welcome_buffer.size();
        if !(self.display_area.size() > self.welcome_buffer.size()) { // 偏序比较.
            return Err(Error::BufferSizeExceeds {
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
            let column = start_column - line.len() / 2;
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

    fn new() -> EditArea {
        EditArea {
            buffer_display_offset: Location::new(0, 0),
            display_area: Area::new(0, 0, 0, 0),
            buffer: Buffer::new(),
            welcome_buffer: {
                let mut buffer = Buffer::new();
                buffer.load("welcome.txt").unwrap();
                buffer
            }, // todo 提供设置途径.
            need_printing: false,
        }
    }
}
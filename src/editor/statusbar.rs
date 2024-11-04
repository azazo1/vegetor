use crate::editor::{Location, Printable};
use crate::editor::editarea::Area;
use crate::editor::terminal::Terminal;
use crate::error;

/// 在状态条左右有多长的空白.
pub const HORIZONTAL_PADDING: usize = 2;

/// [`StatusBar`] 中文字的显示位置.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Packing {
    /// 居中显示.
    Center,
    /// 靠左显示.
    ///
    /// # Params
    ///
    /// - (usize, usize): 左边距和右边距, 如果显示区域宽度长度不足则无效.
    Left(usize, usize),
    /// 靠右显示.
    ///
    /// # Params
    ///
    /// - (usize, usize): 左边距和右边距, 如果显示区域宽度长度不足则无效.
    Right(usize, usize),
}

/// 状态显示条, 显示区域高度只有一行.
#[derive(Debug)]
pub struct StatusBar {
    /// 显示区域在终端中的行序号.
    display_line: usize,
    /// 显示区域在终端行中的起始位置.
    display_start: usize,
    /// 显示区域的水平宽度, 不是实际字符占据的宽度, 还要考虑 HORIZONTAL_PADDING (左右各一).
    display_width: usize,
    /// 显示的内容.
    content: String,
    content_packing: Packing,
    need_printing: bool,
}

impl Printable for StatusBar {
    fn need_printing(&self) -> bool {
        self.need_printing
    }

    fn set_need_printing(&mut self) {
        self.need_printing = true;
    }

    fn unset_need_printing(&mut self) {
        self.need_printing = false;
    }
}

impl StatusBar {
    pub fn new() -> StatusBar {
        StatusBar {
            display_line: 0,
            display_start: 0,
            display_width: 0,
            content: String::new(),
            content_packing: Packing::Center,
            need_printing: false,
        }
    }

    pub fn set_packing(&mut self, packing: Packing) {
        self.content_packing = packing;
    }

    /// 将自身内容打印到终端.
    ///
    /// # Notice
    ///
    /// 此方法成功被调用之后无法让 cursor 回归原来位置, 需要手动调整.
    pub fn print_to(&self, terminal: &mut Terminal) -> error::Result<()> {
        terminal.hide_cursor()?;
        // 清空显示区域.
        terminal.move_cursor_to(Location::new(self.display_line, self.display_start))?;
        terminal.print(" ".repeat(self.display_width))?;
        // 确定处理 padding 过后的显示区域.
        let (display_width, display_start) = match self.content_packing {
            Packing::Center => {
                // 这里暂时使用 len() 而不是 chars count, 防止对字符串的非字符边界索引.
                let line_display_width = self.content.len().min(self.display_width);
                (line_display_width,
                 self.display_start + (self.display_width / 2 - line_display_width / 2))
            }
            Packing::Left(l_padding, r_padding) => {
                if self.display_width > l_padding + r_padding {
                    (self.display_width - l_padding - r_padding, self.display_start + l_padding)
                } else {
                    (self.display_width, self.display_start)
                }
            }
            Packing::Right(l_padding, r_padding) => {
                if self.display_width > l_padding + r_padding {
                    let display_width = self.display_width - l_padding - r_padding;
                    let line_display_width = self.content.len().min(display_width);
                    (display_width,
                     self.display_start + self.display_width - r_padding - line_display_width)
                } else {
                    let line_display_width = self.content.len().min(self.display_width);
                    (self.display_width, self.display_start + self.display_width - line_display_width)
                }
            }
        };

        // 打印内容.
        let line = &self.content[..display_width.min(self.content.len())];
        terminal.move_cursor_to(Location::new(display_start, self.display_line))?;
        terminal.print(line)?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn set_content(&mut self, s: String) {
        if self.content != s {
            self.set_need_printing();
        }
        self.content = s;
    }

    /// 配置显示区域.
    ///
    /// # Params
    ///
    /// - `display_area`: Area, 其中:
    ///     - left_top 表示打印行在终端中的第一个位置.
    ///     - width 表示显示区域的水平宽度.
    ///     - height 目前被忽视, 使用使用 1 表示显示区域高度.
    pub fn configure_area(&mut self, display_area: Area) {
        self.display_line = display_area.y();
        self.display_start = display_area.x();
        self.display_width = display_area.width();
        self.set_need_printing();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn packing() {
        // test center
        // test left
        // test right
    }
}
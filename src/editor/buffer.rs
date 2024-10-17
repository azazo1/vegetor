use std::fs;
use std::fs::read_to_string;
use std::path::Path;
use thiserror;
use std::{fmt, io};
use unicode_width::UnicodeWidthStr;
use crate::editor::terminal::{Size, Location, Terminal};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The print area size doesn't fit the buffer.")]
    PrintAreaSizeNotFit,
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),
    #[error("Carpet out of buffer height, carpet y: {carpet}, buffer height: {height}.")]
    CarpetOutOfHeight { carpet: usize, height: usize },
    #[error("Carpet out of text len, carpet x: {carpet}, current line length: {len}.")]
    CarpetOutOfLen { carpet: usize, len: usize },
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Area {
    left_top: Location,
    size: Size,
}

impl Area {
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
}

#[derive(Debug)]
/// 储存文本内容.
pub struct Buffer {
    /// 当前写入 Buffer 的位置, 在 carpet 索引的字符前进行输入, 不是终端的 cursor.
    carpet: Location,
    lines: Vec<String>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            carpet: Location::default(),
            lines: Vec::new(),
        }
    }

    /// 从文件中加载 Buffer, 加载完毕之后 carpet 在末尾.
    pub fn load(&mut self, file: impl AsRef<Path>) -> Result<(), Error> {
        self.lines.clear();
        let s = read_to_string(file)?;
        self.lines = s.split('\n').map(str::to_string).collect();
        self.carpet = Location { x: self.lines.get(self.lines.len() - 1).unwrap().len(), y: self.lines.len() - 1 };
        Ok(())
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Option<&String> {
        self.lines.get(idx)
    }

    #[inline]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut String> {
        self.lines.get_mut(idx)
    }

    /// 获取当前 carpet 所在的行.
    pub fn get_current_line(&self) -> Option<&String> {
        self.get(self.carpet.y)
    }

    /// 获取当前 carpet 所在的行, 以用于修改.
    pub fn get_current_line_mut(&mut self) -> Option<&mut String> {
        self.get_mut(self.carpet.y)
    }

    /// 获取当前行, 如果当前行不存在则创建当前行及之前的空行, 并返回当前行.
    pub fn ensure_current_line(&mut self) -> &mut String {
        // 很奇怪, 为什么这样编译不过, 不是有个分支表示不同的情况吗怎么还说多次可变借用.
        // let line = self.get_current_line_mut();
        // if let Some(line) = line {
        //     return line;
        // } else {
        //     self.lines.push(String::new());
        //     // ...
        // }

        let line = self.get_current_line(); // 这里不能直接获取 mut 然后返回它, 不然会报二次可变借用的错误, 特奇怪.
        if line.is_some() {
            return self.get_current_line_mut().unwrap();
        }
        loop {
            let line = self.get_current_line(); // 这里同理不能使用 mut.
            if line.is_some() {
                return self.get_current_line_mut().unwrap();
            } else {
                self.lines.push(String::new());
            }
        }
    }

    /// 检查 carpet 位置是否合理.
    /// - 竖直方向上: 检查 carpet 是否在有效输入行内.
    /// - 水平方向上: 检查是否超出当前行文字范围.
    ///
    /// # Errors
    /// - [`Error::CarpetOutOfHeight`]: carpet 在竖直方向上超出.
    /// - [`Error::CarpetOutOfLen`]: carpet 在水平方向上超出.
    fn check_carpet(&self) -> Result<(), Error> {
        if self.carpet.y > self.len() { // 允许等于, 因为超出文字一行可以用来输入新的行.
            return Err(Error::CarpetOutOfHeight { carpet: self.carpet.y, height: self.len() });
        }
        let current_line = self.get_current_line();
        let len = if matches!(current_line, None) { 0 } else { current_line.unwrap().len() };
        if self.carpet.x > len { // 允许等于, 同上.
            Err(Error::CarpetOutOfLen { carpet: self.carpet.x, len })
        } else {
            Ok(())
        }
    }

    /// 把自身内容打印到终端.
    ///
    /// # Arguments
    ///
    /// * `terminal`: 终端对象.
    /// * `area`: 在终端中的打印区域.
    ///
    /// # returns
    /// * `Result<(), Error>`:
    ///     - `Ok(())`: 打印成功.
    ///     - `Err(Error)`: 打印尺寸不符合要求或者 io 错误.
    pub fn print_to(&self, terminal: &mut Terminal, area: Area) -> Result<(), Error> {
        // if !(area.size > self.size) { // 这里由于是偏序, 和 <= 不等价, 这里表达的意思是是否 area 有任一部分小于 self.size 中的对应部分.
        //     return Err(Error::PrintAreaSizeNotFit);
        // }
        // todo 这是 view 的功能.
        // todo 还要在 view 中加个滑动窗口的变量.
        for row in 0..area.height() {
            match self.get(row) {
                Some(line) => {
                    terminal.move_cursor_to(Location { x: area.x(), y: area.y() + row })?;
                    terminal.print(&line[0..area.width().min(line.width_cjk())])?; // todo 测试 unicode width 是否准确, 多拿中文测.
                }
                None => {}
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// 把 Buffer 内容保存到文件.
    ///
    /// # Arguments 
    ///
    /// * `path`: 要保存到的文件路径.
    ///
    /// # Errors
    ///
    /// - [`io::Error`].
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        fs::write(path, self.lines.join("\n"))?;
        Ok(())
    }

    /// 移动 carpet 到指定位置.
    #[cfg(test)]
    pub fn seek_uncheck(&mut self, carpet_pos: Location) {
        self.carpet = carpet_pos;
    }
}

impl fmt::Write for Buffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if !c.is_control() && c != '\r' {
                self.check_carpet().or_else(|_| Err(fmt::Error))?;
                let carpet_x = self.carpet.x; // 只能在 ensure_current_line 前获取.
                self.carpet.x += 1;
                let line = self.ensure_current_line();
                line.insert(carpet_x, c);
            } else if c == '\n' {
                self.carpet.y += 1;
                self.lines.insert(self.carpet.y, String::new());
            }
        }
        Ok(())
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

#[cfg(test)]
mod test {
    use crate::editor::buffer::Buffer;
    use std::fmt::Write;
    use crate::editor::buffer::Location;

    #[test]
    fn load_and_save() {
        let mut buffer = Buffer::new();
        buffer.load("Cargo.lock").unwrap();
        println!("{}", buffer);
        println!("{:?}", buffer);
        buffer.save("Cargo1.lock").unwrap();
    }

    #[test]
    fn write_to_buffer() {
        let mut buffer = Buffer::new();
        buffer.load("Cargo.lock").unwrap();
        buffer.seek_uncheck(Location { x: 2, y: 1 });
        write!(buffer, "Hello World").unwrap();
        println!("{}", buffer);
    }
}
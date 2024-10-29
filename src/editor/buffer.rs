use std::path::Path;
use crate::{error, CharsCount};
use std::{fmt, fs};
use crate::editor::terminal::{Size, Location};


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
    pub fn load(&mut self, file: impl AsRef<Path>) -> error::Result<()> {
        self.clear();
        let s = fs::read_to_string(file)?;
        self.lines = s.split('\n').map(|x| x.trim_matches(|c| c == '\r' || c == '\n').to_string()).collect();
        let line_cnt = self.lines.len();
        if line_cnt == 0 {
            self.carpet.x = 0;
            self.carpet.y = 0;
        } else {
            self.carpet.x = self.lines.get(line_cnt - 1).unwrap().chars_count();
            self.carpet.y = line_cnt - 1;
        }
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
    fn check_carpet(&self) -> error::Result<()> {
        if self.carpet.y > self.len() { // 允许等于, 因为超出文字一行可以用来输入新的行.
            return Err(error::Error::CarpetOutOfHeight { carpet: self.carpet.y, height: self.len() });
        }
        let current_line = self.get_current_line();
        let len = if matches!(current_line, None) { 0 } else { current_line.unwrap().len() };
        if self.carpet.x > len { // 允许等于, 同上.
            Err(error::Error::CarpetOutOfLen { carpet: self.carpet.x, len })
        } else {
            Ok(())
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// 获取最长一行的宽度, todo 考虑要不要使用 width_cjk.
    pub fn max_width(&self) -> usize {
        match self.lines.iter().max_by_key(|x| x.len()) {
            Some(l) => l.len(),
            None => 0
        }
    }

    /// 获取 Buffer 的二维占据尺寸, 使用的是 [`Buffer::max_width`] 和 [`Buffer::len`].
    pub fn size(&self) -> Size {
        Size::new(self.max_width(), self.len())
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
    pub fn save(&self, path: impl AsRef<Path>) -> error::Result<()> {
        fs::write(path, self.lines.join("\n"))?;
        Ok(())
    }

    /// 移动 carpet 到指定位置.
    pub(crate) fn seek_unchecked(&mut self, carpet_pos: Location) {
        self.carpet = carpet_pos;
    }

    /// 清空内容
    pub fn clear(&mut self) {
        self.carpet.x = 0;
        self.carpet.y = 0;
        self.lines.clear();
    }

    pub fn carpet(&self) -> Location {
        self.carpet
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
                self.carpet.x = 0;
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
    use std::fs;
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
        buffer.seek_unchecked(Location { x: 2, y: 1 });
        write!(buffer, "Hello World").unwrap();
        println!("{}", buffer);
    }

    #[test]
    fn write_file_to_buffer() {
        let mut buffer = Buffer::new();
        let string = fs::read_to_string("Cargo.lock").unwrap();
        buffer.write_str(&string).unwrap();
        print!("{}", buffer);
    }
}
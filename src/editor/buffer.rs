use std::path::Path;
use crate::{error, CharsCount};
use std::{fmt, fs};
use crate::editor::terminal::{Size, Location};


#[derive(Debug)]
/// 储存文本内容.
pub struct Buffer {
    /// 当前写入 Buffer 的位置, 在 caret 索引的字符前进行输入, 不是终端的 cursor.
    caret: Location,
    lines: Vec<String>,
}

/// [`Buffer`] 内容读取器, 在此读取器的生命周期时, buffer 内容不会改变.
pub struct BufferReader<'a> {
    caret: Location,
    buffer: &'a Buffer,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            caret: Location::default(),
            lines: Vec::new(),
        }
    }

    /// 从文件中加载 Buffer, 加载完毕之后 caret 在末尾.
    pub fn load(&mut self, file: impl AsRef<Path>) -> error::Result<()> {
        self.clear();
        let s = fs::read_to_string(file)?;
        self.lines = s.split('\n').map(|x| x.trim_matches(|c| c == '\r' || c == '\n').to_string()).collect();
        let line_cnt = self.lines.len();
        if line_cnt == 0 {
            self.caret.x = 0;
            self.caret.y = 0;
        } else {
            self.caret.x = self.lines.get(line_cnt - 1).unwrap().chars_count();
            self.caret.y = line_cnt - 1;
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

    /// 获取当前 caret 所在的行.
    pub fn get_current_line(&self) -> Option<&String> {
        self.get(self.caret.y)
    }

    /// 获取当前 caret 所在的行, 以用于修改.
    pub fn get_current_line_mut(&mut self) -> Option<&mut String> {
        self.get_mut(self.caret.y)
    }

    /// 获取当前行, 如果当前行不存在则创建当前行及之前的空行, 并返回当前行.
    pub fn ensure_current_line(&mut self) -> &mut String {
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

    fn check_self_caret(&self) -> error::Result<()> {
        self.check_caret(self.caret)
    }

    /// 检查 caret 位置是否合理.
    /// - 竖直方向上: 检查 caret 是否在有效输入行内.
    /// - 水平方向上: 检查是否超出当前行文字范围.
    ///
    /// # Errors
    /// - [`Error::CaretOutOfHeight`]: caret 在竖直方向上超出.
    /// - [`Error::CaretOutOfLen`]: caret 在水平方向上超出.    
    pub fn check_caret(&self, caret: Location) -> error::Result<()> {
        if caret.y >= self.len() { // 不允许等于, 如果要在新的一行写字, 先添加新行.
            return Err(error::Error::CaretOutOfHeight { caret: caret.y, height: self.len() });
        }
        let line = self.get(caret.y);
        let len = if matches!(line, None) { 0 } else { line.unwrap().len() };
        if caret.x > len { // 允许等于, 以便在行末添加文本.
            Err(error::Error::CaretOutOfLen { caret: caret.x, len })
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

    /// 移动 caret 到指定位置.
    pub(crate) fn seek_unchecked(&mut self, caret_pos: Location) {
        self.caret = caret_pos;
    }

    /// 清空内容
    pub fn clear(&mut self) {
        self.caret.x = 0;
        self.caret.y = 0;
        self.lines.clear();
    }

    pub fn caret(&self) -> Location {
        self.caret
    }

    /// 获取一个字符读取器, 从 caret 的位置开始读取.
    pub fn get_reader(&self) -> BufferReader {
        BufferReader::new(&self)
    }
}

impl<'a> BufferReader<'a> {
    fn new(buffer: &'a Buffer) -> BufferReader<'a> {
        BufferReader {
            caret: buffer.caret,
            buffer,
        }
    }

    pub fn caret(&self) -> Location {
        self.caret
    }

    /// 跳过字符直到 f 返回 true.
    ///
    /// caret 将指在第一个让 f 返回 true 的字符,
    /// 下一次调用[`BufferReader::next`] 将返回该字符.
    ///
    /// # Errors
    ///
    /// - [`error::Error::EndOfFile`]: 到达了 buffer 的末尾且仍没有字符使 f 返回 true, 此时 caret 的位置和调用前相同.
    pub fn skip_until(&mut self, f: impl Fn(char) -> bool) -> error::Result<()> {
        let origin_caret = self.caret;
        loop {
            let prev_caret = self.caret;
            match self.next() {
                Some(ch) if f(ch) => {
                    self.caret = prev_caret;
                    return Ok(());
                }
                None => {
                    self.caret = origin_caret;
                    return Err(error::Error::EndOfFile);
                }
                _ => ()
            }
        }
    }

    #[inline]
    pub fn skip_until_blank(&mut self) -> error::Result<()> {
        self.skip_until(char::is_whitespace)
    }

    #[inline]
    pub fn skip_until_not_blank(&mut self) -> error::Result<()> {
        self.skip_until(|c| !c.is_whitespace())
    }
}

impl<'a> Iterator for BufferReader<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.get(self.caret.y) {
            Some(line) => {
                if self.caret.x >= line.len() {
                    self.caret.y += 1;
                    self.caret.x = 0;
                    // 行末补充一个换行符, 除非是文本最末尾.
                    if self.buffer.get(self.caret.y).is_some() {
                        Some('\n')
                    } else {
                        None
                    }
                } else {
                    let line = &line[self.caret.x..];
                    let ch = line.chars().next().unwrap();
                    self.caret.x += ch.len_utf8();
                    Some(ch)
                }
            }
            None => None,
        }
    }
}

impl fmt::Write for Buffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if !c.is_control() && c != '\r' {
                self.check_self_caret().or_else(|_| Err(fmt::Error))?;
                let caret_x = self.caret.x; // 只能在 ensure_current_line 前获取.
                self.caret.x += 1;
                let line = self.ensure_current_line();
                line.insert(caret_x, c);
            } else if c == '\n' {
                self.caret.y += 1;
                self.caret.x = 0;
                self.lines.insert(self.caret.y, String::new());
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

    #[test]
    fn buffer_reader() {
        let mut buffer = Buffer::new();
        buffer.load("example-horizontal.txt").unwrap();
        buffer.seek_unchecked(Location::new(0, 0));
        let mut reader = buffer.get_reader();
        let mut string = String::new();
        loop {
            match reader.next() {
                Some(ch) => {
                    write!(string, "{}", ch).unwrap();
                }
                None => {
                    break;
                }
            }
        }
        assert_eq!(string, format!("{}", buffer));
    }
}
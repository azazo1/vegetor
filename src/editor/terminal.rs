use std::cmp;
use std::ops::Add;
use std::fmt::Display;
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode};
use crossterm::cursor::{Hide, Show};
use crossterm::event;
use crossterm::{Command, queue};
use std::io;
use std::io::Write;
use crossterm::cursor::MoveTo;
use crossterm::style::Print;


#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub struct Location {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        if self.width < other.width && self.height < other.height {
            Some(cmp::Ordering::Less)
        } else if self.width > other.width && self.height > other.height {
            Some(cmp::Ordering::Greater)
        } else if self.width == other.width && self.height == other.height {
            Some(cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

pub struct Terminal {
    _ign: u8,
}

impl Terminal {
    fn queue_command(&mut self, com: impl Command) -> io::Result<()> {
        queue!(io::stdout(), com)
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.queue_command(EnterAlternateScreen)
    }

    fn exit_alternate_screen(&mut self) -> io::Result<()> {
        self.queue_command(LeaveAlternateScreen)
    }

    pub fn new() -> Terminal {
        Terminal { _ign: 0 }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }

    pub fn initialize(&mut self) -> io::Result<()> {
        self.enter_alternate_screen()?;
        enable_raw_mode()?;
        Ok(())
    }

    pub fn destruct(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.exit_alternate_screen()?;
        self.flush()?; // 这样才能让 exit_alternate_screen 立即生效, 不然的话可能导致报错输出在 alternate_screen 中.
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.queue_command(Clear(ClearType::All))
    }

    pub fn print(&mut self, s: impl Display) -> io::Result<()> {
        self.queue_command(Print(s))
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.queue_command(Hide)
    }

    pub fn show_cursor(&mut self) -> io::Result<()> {
        self.queue_command(Show)
    }

    pub fn move_cursor_to(&mut self, loc: Location) -> io::Result<()> {
        let loc = loc.as_u16_checked().ok_or_else(
            || io::Error::new(io::ErrorKind::InvalidInput, "location cannot be cast to (u16, u16)")
        )?;
        self.queue_command(MoveTo(loc.0, loc.1))
    }

    /// 读取终端事件.
    ///
    /// 见 `crossterm::event::read` 函数.
    pub fn read_event_blocking(&self) -> io::Result<event::Event> {
        event::read()
    }

    /// 获取终端尺寸.
    pub fn size(&self) -> io::Result<Size> {
        let size = crossterm::terminal::size()?;
        Ok(size.into())
    }
}

macro_rules! usize_pair {
    ($t:ident, $u1: ident, $u2: ident) => {
        impl Add<(usize, usize)> for $t {
            type Output = $t;

            fn add(self, rhs: (usize, usize)) -> Self::Output {
                $t {
                    $u1: rhs.0 + self.$u1,
                    $u2: rhs.1 + self.$u2,
                }
            }
        }

        impl Add<$t> for $t {
            type Output = $t;

            fn add(self, rhs: $t) -> Self::Output {
                $t {
                    $u1: rhs.$u1 + self.$u1,
                    $u2: rhs.$u2 + self.$u2,
                }
            }
        }

        // impl TryFrom<$t> for (u16, u16) { // TryInto 和 TryFrom 会自动实现, 因为有了 Into 特征.

        impl Into<(u16, u16)> for $t {
            fn into(self) -> (u16, u16) {
                self.as_u16()
            }
        }
        
        impl Into<$t> for (u16, u16) {
            fn into(self) -> $t {
                $t::new(self.0 as usize, self.1 as usize)
            }
        }

        impl Into<(usize, usize)> for $t {
            fn into(self) -> (usize, usize) {
                (self.$u1, self.$u2)
            }
        }

        impl $t {
            pub fn new($u1: usize, $u2: usize) -> $t {
                $t { $u1, $u2 }
            }

            /// 把自己转换成 u16 元素的结构, 不检查截断.
            pub fn as_u16(&self) -> (u16, u16) {
                (self.$u1 as u16, self.$u2 as u16)
            }

            /// 转换成 (u16, u16) 但是会检查内容范围.
            ///
            /// # Returns
            /// 当内容转换成 (u16, u16) 后会被截断, 那么返回 None.
            pub fn as_u16_checked(&self) -> Option<(u16, u16)> {
                let u16_range = 0..=u16::MAX as usize;
                if u16_range.contains(&self.$u1) && u16_range.contains(&self.$u2) {
                    Some(self.as_u16())
                } else {
                    None
                }
            }
        }
    }
}

usize_pair!(Location, x, y);
usize_pair!(Size, width, height);

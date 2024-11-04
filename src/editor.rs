use std::fmt::Write;
use std::path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub use crate::editor::terminal::{Location, Size};
use crate::editor::editarea::{Area, EditArea};
use crate::editor::terminal::Terminal;
use crate::error;
use crate::CARGO_PKG_NAME;
use crate::editor::statusbar::{Packing, StatusBar};

mod editarea;
mod terminal;
mod buffer;
mod statusbar;

/// tab 键插入的空格数量.
const TAB_WIDTH: usize = 4;

trait Printable {
    /// 此对象是否需要重绘.
    fn need_printing(&self) -> bool;
    /// 设置为需要重绘.
    fn set_need_printing(&mut self);
    /// 设置为不需要重绘.
    fn unset_need_printing(&mut self);
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum State {
    Welcoming,
    Editing,
    Exiting,
}

#[derive(Debug)]
pub enum BufferLoadConfig<'a> {
    /// 使用文件的内容来填充 buffer.
    File(&'a path::Path),
    /// 使用字符串的内容来填充 buffer.
    String(&'a str),
    /// buffer 为空.
    ///
    /// # Notice
    ///
    /// 这和读取到空内容的 [`BufferLoadConfig::String`] 和 [`BufferLoadConfig::File`] 有点区别:
    ///
    /// 在初始化 welcome_buffer 时, 如果给了 [`BufferLoadConfig::Empty`], 那么不会显示 welcome 屏幕,
    /// 而是直接进入编辑状态.
    Empty,
}

impl<'a> Default for BufferLoadConfig<'a> {
    fn default() -> BufferLoadConfig<'a> {
        BufferLoadConfig::Empty
    }
}

#[derive(Debug, Default)]
pub struct EditorBuildConfig<'a> {
    /// 设置欢迎屏幕的显示内容.
    ///
    /// - [`BufferLoadConfig::File`]: 此选项会加载指定的文件, 然后对文件内容在欢迎屏幕上居中显示.
    /// - [`BufferLoadConfig::String`]: 此选项会让欢迎屏幕居中显示指定字符串.
    /// - [`BufferLoadConfig::Empty`]: 此选项会让 [`Editor`] 直接跳过欢迎阶段, 直接进入编辑阶段.
    pub welcome_config: BufferLoadConfig<'a>,
    /// 设置要进行编辑的文本.
    ///
    /// - [`BufferLoadConfig::File`]: 此选项会加载指定的文件, 然后对文件内容进行编辑.
    /// - [`BufferLoadConfig::String`]: 此选项会初始化 buffer 为指定的字符串, 并对其进行编辑.
    /// - [`BufferLoadConfig::Empty`]: 此选项让 buffer 初始化为空.
    pub edit_text_config: BufferLoadConfig<'a>,
}

pub struct Editor {
    edit_area: EditArea,
    status_bar: StatusBar,
    terminal: Terminal,
    state: State,
}

impl Editor {
    fn panic_handler(_info: &std::panic::PanicHookInfo) {
        let _ = Terminal::new().destruct(); // 唯一的 Terminal 二次构建情况,
        println!("{} error.", CARGO_PKG_NAME);
        // 我实在想不出来到底怎么合适的使用唯一那个 Terminal.

        // panic_handler 不能省, 因为是先执行的 panic 输出然后再执行的 drop,
        // 如果在 drop 中对 terminal 进行资源清理操作会导致 panic 信息无法显示.
    }

    pub fn build(config: &EditorBuildConfig) -> error::Result<Editor> {
        let raw_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Editor::panic_handler(info);
            raw_hook(info);
        }));

        let mut terminal = Terminal::new();
        terminal.initialize()?;
        let mut edit_area = EditArea::new();

        let mut status_bar = StatusBar::new();
        status_bar.set_content("Hello World".into());
        status_bar.set_packing(Packing::Left(statusbar::HORIZONTAL_PADDING, statusbar::HORIZONTAL_PADDING));

        let mut editor = Editor {
            edit_area,
            status_bar,
            terminal,
            state: State::Welcoming,
        };

        match config.welcome_config {
            BufferLoadConfig::Empty => { editor.state = State::Editing }
            BufferLoadConfig::File(file_path) => {
                let welcome = editor.edit_area.get_welcome_buffer_mut();
                welcome.load(file_path)?
            }
            BufferLoadConfig::String(string) => {
                let welcome = editor.edit_area.get_welcome_buffer_mut();
                // welcome.clear(); // 本来就没写什么
                write!(welcome, "{}", string).unwrap();
            }
        }

        match config.edit_text_config {
            BufferLoadConfig::Empty => {}
            BufferLoadConfig::String(string) => {
                let buffer = editor.edit_area.get_buffer_mut();
                // buffer.clear(); // 本来就没写什么
                write!(buffer, "{}", string).unwrap();
            }
            BufferLoadConfig::File(path) => {
                let buffer = editor.edit_area.get_buffer_mut();
                buffer.load(path)?;
            }
        }

        editor.update_area_configuration()?;

        Ok(editor)
    }

    pub fn run(&mut self) -> error::Result<()> {
        while self.state != State::Exiting {
            if self.check_need_printing() {
                self.terminal.clear_screen()?;
                match self.state {
                    State::Welcoming => {
                        self.edit_area.print_welcome_to(&mut self.terminal).or_else(|e| {
                            match e {
                                // 忽略 buffer 尺寸不合适的情况.
                                error::Error::BufferSizeExceeds { .. } => { Ok(()) }
                                _ => { Err(e) }
                            }
                        })?;
                    }
                    State::Editing => {
                        self.status_bar.print_to(&mut self.terminal)?; // 先打印, 因为其无法回归 cursor 位置.
                        self.edit_area.print_to(&mut self.terminal)?;
                    }
                    _ => {}
                }
                self.edit_area.unset_need_printing();
                self.status_bar.unset_need_printing();
            }
            self.terminal.flush()?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn handle_event(&mut self) -> error::Result<()> {
        let evt = self.terminal.read_event_blocking();
        match evt {
            Ok(Event::Key(key_event)) => {
                let KeyEvent { kind, code, modifiers, .. } = key_event;
                if kind == KeyEventKind::Press {
                    match code {
                        KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => {
                            self.state = State::Exiting;
                        }
                        #[cfg(debug_assertions)]
                        KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                            panic!("Ctrl-C");
                        }
                        _ => {
                            if self.state == State::Welcoming {
                                self.state = State::Editing; // 有按键按下就进入 Editing, 其余不做任何动作.
                                self.edit_area.set_need_printing();
                            } else if let Ok(caret_move) = key_event.try_into() {
                                self.terminal.move_cursor_to(self.edit_area.move_caret(caret_move))?;
                            } else {
                                match code {
                                    KeyCode::Char(ch) if modifiers == KeyModifiers::NONE => {
                                        write!(self.edit_area, "{ch}").unwrap();
                                    }
                                    KeyCode::Char('s') if modifiers == KeyModifiers::CONTROL => {
                                        // todo 当启动时指定文件名了, 那就保存到指定的文件.
                                        // todo 判断是否有编辑痕迹, 如果有编辑痕迹.
                                        // todo 如果启动时没有启动参数指定文件名, 那么使用 control s 保存的时候先询问文件名.
                                    }
                                    KeyCode::Enter if modifiers == KeyModifiers::NONE => {
                                        write!(self.edit_area, "\n").unwrap();
                                    }
                                    KeyCode::Tab if modifiers == KeyModifiers::NONE => {
                                        write!(self.edit_area, "{}", " ".repeat(TAB_WIDTH)).unwrap();
                                    }
                                    KeyCode::Backspace if modifiers == KeyModifiers::NONE => {
                                        let _ = self.edit_area.del_char();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Resize(_, _)) => {
                self.update_area_configuration()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 检查子元素中是否有需要重新绘制的.
    fn check_need_printing(&self) -> bool {
        self.edit_area.need_printing()
            || self.status_bar.need_printing()
    }

    fn update_area_configuration(&mut self) -> error::Result<()> {
        let (width, height): (usize, usize) = self.terminal.size()?.into();
        // 发现如果直接传入 terminal_size.width 和 terminal_size.height 的话, caret 会莫名奇妙保留到终端最右下角.
        self.edit_area.configure_area(Area::new(0, 0, width - 1, height - 1));
        self.status_bar.configure_area(Area::new(0, height - 1, width - 1, 1));
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        // 测试发现就算 panic 了这里也是会调用的.
        let _ = self.terminal.destruct();
        println!("{} leaving...", CARGO_PKG_NAME);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::editor::{BufferLoadConfig, Editor, EditorBuildConfig};

    #[test]
    fn draw_in_split() {}

    #[test]
    fn scroll_vertical() {
        let mut config = EditorBuildConfig::default();
        config.edit_text_config = BufferLoadConfig::File(Path::new("example-vertical.txt"));
        let mut editor = Editor::build(&config).unwrap();
        editor.run().unwrap();
    }

    #[test]
    fn scroll_horizontal() {
        let mut config = EditorBuildConfig::default();
        config.edit_text_config = BufferLoadConfig::File(Path::new("example-horizontal.txt"));
        let mut editor = Editor::build(&config).unwrap();
        editor.run().unwrap();
    }
}

// todo 保存文件功能.
// todo `撤销`功能.
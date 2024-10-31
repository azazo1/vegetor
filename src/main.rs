use std::path;
use std::path::Path;
use vegetor::editor::{Editor, EditorBuildConfig, BufferLoadConfig};

// 如果这里使用 fn main() -> anyhow::Result<()> { ... } 的话,
// 如果产生了错误, 那么 editor 的 panic_handler 将无法捕获错误,
// 应该在 main 函数中使用 unwrap 直接 panic.
fn main() {
    let mut config = EditorBuildConfig::default();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1) {
        Some(file) => {
            config.edit_text_config = BufferLoadConfig::File(Path::new(file));
        }
        None => {
            config.welcome_config = BufferLoadConfig::File(Path::new("welcome.txt"));
        }
    }
    let mut editor = Editor::build(&config).unwrap();
    editor.run().unwrap();
}
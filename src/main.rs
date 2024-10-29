use vegetor::editor::Editor;
use anyhow::Result;

fn main() -> Result<()> {
    let mut editor = Editor::build()?;
    editor.run()?;
    Ok(())
}

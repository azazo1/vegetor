pub mod editor;
pub mod error;

const CARGO_PKG_NAME: &'static str = env!("CARGO_PKG_NAME");

trait CharsCount {
    fn chars_count(&self) -> usize;
}

impl<T: AsRef<str>> CharsCount for T {
    fn chars_count(&self) -> usize {
        self.as_ref().chars().count()
    }
}
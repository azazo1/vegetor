pub mod editor;
pub mod error;

trait CharsCount {
    fn chars_count(&self) -> usize;
}

impl<T: AsRef<str>> CharsCount for T {
    fn chars_count(&self) -> usize {
        self.as_ref().chars().count()
    }
}
pub trait RemoveExtensions<T> {
    fn remove_first(&mut self) -> Option<T>;
    fn remove_last(&mut self) -> Option<T>;
}

impl<T> RemoveExtensions<T> for Vec<T> {
    fn remove_first(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(self.remove(0))
    }

    fn remove_last(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(self.remove(self.len() - 1))
    }
}

pub trait ResultExt<T> {
    fn inspect_ok(self, f: impl FnOnce(&T)) -> Self;
}

impl<T, E> ResultExt<T> for Result<T, E> {
    fn inspect_ok(self, f: impl FnOnce(&T)) -> Self {
        if let Ok(ref val) = self {
            f(val);
        }
        self
    }
}

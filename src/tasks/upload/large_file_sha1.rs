use std::cell::RefCell;

pub(super) struct LargeFileSha1(RefCell<Vec<String>>);
unsafe impl Send for LargeFileSha1 {}
unsafe impl Sync for LargeFileSha1 {}

impl LargeFileSha1 {
    pub fn new(num_of_parts: usize) -> Self {
        Self(RefCell::new(vec![String::new(); num_of_parts]))
    }

    pub fn set_sha1(&self, index: usize, sha1: String) {
        self.0.borrow_mut()[index] = sha1;
    }
}

impl Into<Vec<String>> for LargeFileSha1 {
    fn into(self) -> Vec<String> {
        self.0.into_inner()
    }
}

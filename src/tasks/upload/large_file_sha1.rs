use std::sync::Mutex;

pub(super) struct LargeFileSha1(Mutex<Vec<String>>);

impl LargeFileSha1 {
    pub fn new(num_of_parts: usize) -> Self {
        Self(Mutex::new(vec![String::new(); num_of_parts]))
    }

    pub fn set_sha1(&self, index: usize, sha1: String) {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())[index] = sha1;
    }
}

impl From<LargeFileSha1> for Vec<String> {
    fn from(val: LargeFileSha1) -> Self {
        val.0.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

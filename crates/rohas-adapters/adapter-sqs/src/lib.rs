pub struct SqsAdapter;

impl SqsAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

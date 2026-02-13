/// Progress information for tracking long-running operations.
#[derive(Debug, Clone, Copy)]
pub struct Progress {
    /// Number of items completed.
    pub completed: usize,
    /// Total number of items.
    pub total: usize,
}

impl Progress {
    /// Create a new progress tracker.
    pub fn new(completed: usize, total: usize) -> Self {
        Self { completed, total }
    }

    /// Calculate completion percentage (0.0 to 1.0).
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.completed as f64 / self.total as f64
        }
    }
}

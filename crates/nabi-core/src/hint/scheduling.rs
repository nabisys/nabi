/// Hint to the runtime about which scheduler should execute a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SchedulingHint {
    /// Work-stealing scheduler — tasks may migrate between workers.
    #[default]
    WorkStealing,
    /// Thread-per-core scheduler — tasks are pinned to their originating core.
    ThreadPerCore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_work_stealing() {
        assert_eq!(SchedulingHint::default(), SchedulingHint::WorkStealing);
    }

    #[test]
    fn variants_are_distinct() {
        assert_ne!(SchedulingHint::WorkStealing, SchedulingHint::ThreadPerCore);
    }
}

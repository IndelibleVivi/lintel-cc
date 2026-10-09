//! Native admission for App operations versus updater installation/restart.
use std::sync::Arc;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

#[derive(Default)]
pub(crate) struct ActivityGate(Arc<RwLock<()>>);
impl ActivityGate {
    pub(crate) fn operation(&self) -> Result<OwnedRwLockReadGuard<()>, &'static str> {
        self.0
            .clone()
            .try_read_owned()
            .map_err(|_| "update_in_progress")
    }
    pub(crate) fn installation(&self) -> Result<OwnedRwLockWriteGuard<()>, &'static str> {
        self.0
            .clone()
            .try_write_owned()
            .map_err(|_| "app_operation_active")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutually_exclusive_and_recovers_after_completion() {
        let gate = ActivityGate::default();
        let task = gate.operation().unwrap();
        assert!(gate.installation().is_err());
        drop(task);
        let install = gate.installation().unwrap();
        assert!(gate.operation().is_err());
        assert!(gate.installation().is_err());
        drop(install);
        assert!(gate.operation().is_ok());
    }
}

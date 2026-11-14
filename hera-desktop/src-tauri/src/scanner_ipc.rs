use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};
use std::sync::{Arc, OnceLock};

static GLOBAL_SCANNER_SCHEDULER: OnceLock<Arc<ScanScheduler>> = OnceLock::new();

pub fn global_scheduler() -> &'static Arc<ScanScheduler> {
    GLOBAL_SCANNER_SCHEDULER.get_or_init(|| {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Arc::new(ScanScheduler::new(cpus))
    })
}

#[tauri::command]
pub fn scanner_boost_viewport(track_ids: Vec<String>) -> usize {
    let sched = global_scheduler();
    for id in &track_ids {
        sched.enqueue_or_promote(id.clone(), ScanPriority::P1Viewport);
    }
    sched.pending_count()
}

#[tauri::command]
pub fn scanner_prewarm_track(track_id: String) -> usize {
    let sched = global_scheduler();
    sched.enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    sched.pending_count()
}

#[tauri::command]
pub fn scanner_notify_deck_track(track_id: String, camelot_neighbors: Vec<String>) -> usize {
    let sched = global_scheduler();
    sched.enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    for neighbor in camelot_neighbors.into_iter().take(15) {
        sched.enqueue_or_promote(neighbor, ScanPriority::P1Viewport);
    }
    sched.pending_count()
}

#[tauri::command]
pub fn scanner_set_deck_playing(playing: bool) -> usize {
    let sched = global_scheduler();
    sched.governor.set_deck_playing(playing);
    sched.governor.effective_workers()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_ipc_commands_and_governor() {
        let pending = scanner_notify_deck_track(
            "deck_a_trk".into(),
            (0..20).map(|i| format!("nbr_{i}")).collect(),
        );
        assert!(pending >= 16);
        assert_eq!(scanner_set_deck_playing(true), 1);
        assert!(scanner_set_deck_playing(false) >= 1);
    }
}

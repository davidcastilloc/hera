use std::sync::{Arc, OnceLock};

use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};

static GLOBAL_SCANNER_SCHEDULER: OnceLock<Arc<ScanScheduler>> = OnceLock::new();

pub fn global_scheduler() -> &'static Arc<ScanScheduler> {
    GLOBAL_SCANNER_SCHEDULER.get_or_init(|| Arc::new(ScanScheduler::new(4)))
}

#[tauri::command]
pub fn scanner_boost_viewport(track_ids: Vec<String>) -> usize {
    let sched = global_scheduler();
    let mut boosted = 0_usize;
    for id in track_ids {
        sched.enqueue_or_promote(id, ScanPriority::P1Viewport);
        boosted += 1;
    }
    boosted
}

#[tauri::command]
pub fn scanner_prewarm_track(track_id: String) -> usize {
    global_scheduler().enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    1
}

#[tauri::command]
pub fn scanner_notify_deck_track(track_id: String, camelot_neighbors: Vec<String>) -> usize {
    let sched = global_scheduler();
    sched.enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    for neighbor in &camelot_neighbors {
        sched.enqueue_or_promote(neighbor.clone(), ScanPriority::P1Viewport);
    }
    1 + camelot_neighbors.len()
}

#[tauri::command]
pub fn scanner_set_deck_playing(playing: bool) -> usize {
    global_scheduler().governor.set_deck_playing(playing);
    global_scheduler().governor.effective_workers()
}

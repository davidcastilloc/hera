use super::{ScanPriority, ScanScheduler};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_p0_overtakes_500_p2_and_governor_throttles_to_1() {
    let sched = ScanScheduler::new(16);
    assert_eq!(sched.governor.effective_workers(), 16);
    sched.governor.set_deck_playing(true);
    assert_eq!(sched.governor.effective_workers(), 1);
    sched.governor.set_deck_playing(false);
    assert_eq!(sched.governor.effective_workers(), 16);

    for i in 0..500 {
        sched.enqueue_or_promote(format!("track_{i}"), ScanPriority::P2FastBackground);
    }
    sched.enqueue_or_promote("track_420", ScanPriority::P0Immediate);
    assert_eq!(sched.pending_count(), 500);

    let (first, prio) = sched.pop_next().unwrap();
    assert_eq!(
        (first.as_str(), prio),
        ("track_420", ScanPriority::P0Immediate)
    );
    assert_eq!(sched.pending_count(), 499);
}

#[test]
fn test_clear_background_queues_preserves_p0_and_p1() {
    let sched = ScanScheduler::new(4);
    sched.enqueue_or_promote("t0", ScanPriority::P0Immediate);
    sched.enqueue_or_promote("t1", ScanPriority::P1Viewport);
    sched.enqueue_or_promote("t2", ScanPriority::P2FastBackground);
    sched.enqueue_or_promote("t3", ScanPriority::P3FullElastic);
    assert_eq!(sched.pending_count(), 4);

    sched.clear_background_queues();
    assert_eq!(sched.pending_count(), 2);
    assert_eq!(
        sched.pop_next(),
        Some(("t0".to_string(), ScanPriority::P0Immediate))
    );
    assert_eq!(
        sched.pop_next(),
        Some(("t1".to_string(), ScanPriority::P1Viewport))
    );
    assert!(sched.pop_next().is_none());
}

#[test]
fn test_wait_next_wakes_on_enqueue() {
    let sched = Arc::new(ScanScheduler::new(4));
    let sched_clone = Arc::clone(&sched);

    let handle = std::thread::spawn(move || sched_clone.wait_next(Duration::from_millis(500)));
    std::thread::sleep(Duration::from_millis(50));
    sched.enqueue_or_promote("wake_track", ScanPriority::P0Immediate);

    let res = handle.join().unwrap();
    assert_eq!(
        res,
        Some(("wake_track".to_string(), ScanPriority::P0Immediate))
    );
}

#[test]
fn test_active_in_flight_blocks_duplicate_enqueue_until_complete() {
    let sched = ScanScheduler::new(4);
    sched.enqueue_batch(vec!["t_active".to_string()], ScanPriority::P2FastBackground);
    let popped = sched.pop_next().unwrap();
    assert_eq!(popped.0, "t_active");
    assert_eq!(sched.active_count(), 1);
    assert_eq!(sched.pending_count(), 0);

    sched.enqueue_batch(vec!["t_active".to_string()], ScanPriority::P2FastBackground);
    assert!(sched.pop_next().is_none());

    sched.complete("t_active");
    assert_eq!(sched.active_count(), 0);
    assert_eq!(sched.pending_count(), 0);

    sched.enqueue_batch(vec!["t_active".to_string()], ScanPriority::P2FastBackground);
    assert_eq!(sched.pop_next().unwrap().0, "t_active");
}

#[test]
fn test_fast_completed_repromotes_to_full_and_skips_quarantined_minus_one() {
    let sched = ScanScheduler::new(4);
    sched.seed_completed_levels(vec![
        ("quarantined_corrupt".to_string(), -1),
        ("fast_done".to_string(), 1),
        ("full_done".to_string(), 2),
    ]);

    assert_eq!(sched.completed_level_of("quarantined_corrupt"), Some(-1));
    assert_eq!(sched.completed_level_of("fast_done"), Some(1));
    assert_eq!(sched.completed_level_of("full_done"), Some(2));
    assert_eq!(sched.completed_level_of("unknown"), None);

    sched.enqueue_or_promote("quarantined_corrupt", ScanPriority::P0Immediate);
    sched.enqueue_batch(
        vec!["quarantined_corrupt".to_string()],
        ScanPriority::P3FullElastic,
    );
    sched.enqueue_or_promote("fast_done", ScanPriority::P2FastBackground);
    sched.enqueue_or_promote("fast_done", ScanPriority::P1Viewport);
    sched.enqueue_or_promote("full_done", ScanPriority::P0Immediate);
    assert_eq!(sched.pending_count(), 0);

    sched.enqueue_or_promote("fast_done", ScanPriority::P3FullElastic);
    assert_eq!(
        sched.pop_next(),
        Some(("fast_done".to_string(), ScanPriority::P3FullElastic))
    );
    assert_eq!(sched.active_count(), 1);

    sched.mark_completed("fast_done", 2);
    assert_eq!(sched.active_count(), 0);
    assert_eq!(sched.completed_level_of("fast_done"), Some(2));
    sched.enqueue_or_promote("fast_done", ScanPriority::P0Immediate);
    assert_eq!(sched.pending_count(), 0);

    sched.mark_completed("quarantined_corrupt", 1);
    assert_eq!(sched.completed_level_of("quarantined_corrupt"), Some(-1));
}

#[test]
fn test_external_booth_and_battery_governor_freezes_p3_and_throttles_to_1() {
    let sched = ScanScheduler::new(6);
    assert!(!sched.governor.is_throttled_booth_mode());
    assert_eq!(sched.governor.effective_workers(), 6);

    sched.enqueue_or_promote("p3_elastic", ScanPriority::P3FullElastic);
    sched.enqueue_or_promote("p1_viewport", ScanPriority::P1Viewport);

    sched.governor.set_external_booth_mode(true);
    assert!(sched.governor.is_external_booth_mode());
    assert!(sched.governor.is_throttled_booth_mode());
    assert_eq!(sched.governor.effective_workers(), 1);

    assert_eq!(
        sched.pop_next(),
        Some(("p1_viewport".to_string(), ScanPriority::P1Viewport))
    );
    sched.mark_completed("p1_viewport", 1);
    assert_eq!(sched.pending_count(), 1);
    assert!(sched.pop_next().is_none());

    sched.governor.set_external_booth_mode(false);
    sched.governor.set_on_battery_power(true);
    assert!(sched.governor.is_on_battery_power());
    assert!(sched.governor.is_throttled_booth_mode());
    assert_eq!(sched.governor.effective_workers(), 1);
    assert!(sched.pop_next().is_none());

    sched.governor.set_on_battery_power(false);
    assert!(!sched.governor.is_throttled_booth_mode());
    assert_eq!(sched.governor.effective_workers(), 6);
    assert_eq!(
        sched.pop_next(),
        Some(("p3_elastic".to_string(), ScanPriority::P3FullElastic))
    );
}

#[test]
fn test_cancel_and_drain_background_returns_drained_count() {
    let sched = ScanScheduler::new(4);
    sched.enqueue_or_promote("p0", ScanPriority::P0Immediate);
    sched.enqueue_or_promote("p1", ScanPriority::P1Viewport);
    sched.enqueue_or_promote("p2_a", ScanPriority::P2FastBackground);
    sched.enqueue_or_promote("p2_b", ScanPriority::P2FastBackground);
    sched.enqueue_or_promote("p3_a", ScanPriority::P3FullElastic);
    assert_eq!(sched.pending_count(), 5);

    let drained = sched.cancel_and_drain_background();
    assert_eq!(drained, 3);
    assert_eq!(sched.pending_count(), 2);
}

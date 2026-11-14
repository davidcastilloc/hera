use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum ScanPriority {
    P0Immediate = 0,
    P1Viewport = 1,
    P2FastBackground = 2,
    P3FullElastic = 3,
}

pub struct AutoGigGovernor {
    max_workers: usize,
    deck_is_playing: AtomicBool,
}

impl AutoGigGovernor {
    pub fn new(max_workers: usize) -> Self {
        Self {
            max_workers: max_workers.max(1),
            deck_is_playing: AtomicBool::new(false),
        }
    }

    pub fn set_deck_playing(&self, playing: bool) {
        self.deck_is_playing.store(playing, Ordering::Relaxed);
    }

    pub fn is_deck_playing(&self) -> bool {
        self.deck_is_playing.load(Ordering::Relaxed)
    }

    pub fn effective_workers(&self) -> usize {
        if self.deck_is_playing.load(Ordering::Relaxed) {
            1
        } else {
            self.max_workers
        }
    }
}

struct SchedulerInner {
    queues: [VecDeque<String>; 4],
    in_flight_or_queued: HashMap<String, ScanPriority>,
}

pub struct ScanScheduler {
    inner: Mutex<SchedulerInner>,
    pub governor: AutoGigGovernor,
}

impl ScanScheduler {
    pub fn new(max_workers: usize) -> Self {
        Self {
            inner: Mutex::new(SchedulerInner {
                queues: [
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                ],
                in_flight_or_queued: HashMap::new(),
            }),
            governor: AutoGigGovernor::new(max_workers),
        }
    }

    /// Encola o promueve en O(1) amortizado sin duplicar `track_id`.
    pub fn enqueue_or_promote(&self, track_id: impl Into<String>, prio: ScanPriority) {
        let id = track_id.into();
        let mut guard = self.inner.lock().unwrap();
        if let Some(&existing) = guard.in_flight_or_queued.get(&id) {
            if prio < existing {
                guard.in_flight_or_queued.insert(id.clone(), prio);
                guard.queues[prio as usize].push_back(id);
            }
            return;
        }
        guard.in_flight_or_queued.insert(id.clone(), prio);
        guard.queues[prio as usize].push_back(id);
    }

    pub fn pop_next(&self) -> Option<(String, ScanPriority)> {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        for (idx, q) in inner.queues.iter_mut().enumerate() {
            while let Some(id) = q.pop_front() {
                let prio = match idx {
                    0 => ScanPriority::P0Immediate,
                    1 => ScanPriority::P1Viewport,
                    2 => ScanPriority::P2FastBackground,
                    _ => ScanPriority::P3FullElastic,
                };
                if inner.in_flight_or_queued.get(&id) == Some(&prio) {
                    inner.in_flight_or_queued.remove(&id);
                    return Some((id, prio));
                }
            }
        }
        None
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().in_flight_or_queued.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Promueve track_420 a P0 sin duplicar
        sched.enqueue_or_promote("track_420", ScanPriority::P0Immediate);
        assert_eq!(sched.pending_count(), 500);

        let (first, prio) = sched.pop_next().unwrap();
        assert_eq!(first, "track_420");
        assert_eq!(prio, ScanPriority::P0Immediate);
        assert_eq!(sched.pending_count(), 499);
    }
}

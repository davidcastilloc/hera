use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

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
    cv: Condvar,
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
            cv: Condvar::new(),
            governor: AutoGigGovernor::new(max_workers),
        }
    }

    fn pop_from_inner(inner: &mut SchedulerInner) -> Option<(String, ScanPriority)> {
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

    /// Encola o promueve en O(1) amortizado sin duplicar `track_id`.
    pub fn enqueue_or_promote(&self, track_id: impl Into<String>, prio: ScanPriority) {
        let id = track_id.into();
        let mut guard = self.inner.lock().unwrap();
        if let Some(&existing) = guard.in_flight_or_queued.get(&id) {
            if prio < existing {
                guard.in_flight_or_queued.insert(id.clone(), prio);
                guard.queues[prio as usize].push_back(id);
                drop(guard);
                self.cv.notify_one();
            }
            return;
        }
        guard.in_flight_or_queued.insert(id.clone(), prio);
        guard.queues[prio as usize].push_back(id);
        drop(guard);
        self.cv.notify_one();
    }

    /// Encola un lote de pistas bajo una misma prioridad y notifica a los workers.
    pub fn enqueue_batch<I: IntoIterator<Item = String>>(&self, ids: I, prio: ScanPriority) {
        let mut guard = self.inner.lock().unwrap();
        let mut any_added = false;
        let p_idx = prio as usize;
        for id in ids {
            if let Some(&existing) = guard.in_flight_or_queued.get(&id) {
                if prio < existing {
                    guard.in_flight_or_queued.insert(id.clone(), prio);
                    guard.queues[p_idx].push_back(id);
                    any_added = true;
                }
            } else {
                guard.in_flight_or_queued.insert(id.clone(), prio);
                guard.queues[p_idx].push_back(id);
                any_added = true;
            }
        }
        drop(guard);
        if any_added {
            self.cv.notify_all();
        }
    }

    pub fn pop_next(&self) -> Option<(String, ScanPriority)> {
        let mut guard = self.inner.lock().unwrap();
        Self::pop_from_inner(&mut guard)
    }

    /// Extrae la siguiente tarea de forma inmediata si hay disponibles,
    /// o espera hasta `timeout` con Condvar (0% CPU en reposo).
    pub fn wait_next(&self, timeout: Duration) -> Option<(String, ScanPriority)> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(item) = Self::pop_from_inner(&mut guard) {
            return Some(item);
        }
        if timeout.is_zero() {
            return None;
        }
        let (mut guard_after_wait, _) = self.cv.wait_timeout(guard, timeout).unwrap();
        Self::pop_from_inner(&mut guard_after_wait)
    }

    /// Limpia colas P2 (FastBackground) y P3 (FullElastic), preservando P0 y P1.
    pub fn clear_background_queues(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.queues[2].clear();
        guard.queues[3].clear();
        guard.in_flight_or_queued.retain(|_, prio| {
            *prio == ScanPriority::P0Immediate || *prio == ScanPriority::P1Viewport
        });
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().in_flight_or_queued.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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

        let (id0, p0) = sched.pop_next().unwrap();
        assert_eq!(id0, "t0");
        assert_eq!(p0, ScanPriority::P0Immediate);

        let (id1, p1) = sched.pop_next().unwrap();
        assert_eq!(id1, "t1");
        assert_eq!(p1, ScanPriority::P1Viewport);

        assert!(sched.pop_next().is_none());
    }

    #[test]
    fn test_wait_next_wakes_on_enqueue() {
        let sched = Arc::new(ScanScheduler::new(4));
        let sched_clone = Arc::clone(&sched);

        let handle = std::thread::spawn(move || {
            sched_clone.wait_next(Duration::from_millis(500))
        });

        std::thread::sleep(Duration::from_millis(50));
        sched.enqueue_or_promote("wake_track", ScanPriority::P0Immediate);

        let res = handle.join().unwrap();
        assert_eq!(res, Some(("wake_track".to_string(), ScanPriority::P0Immediate)));
    }
}

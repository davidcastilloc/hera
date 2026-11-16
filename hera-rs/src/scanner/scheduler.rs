use std::collections::{HashMap, HashSet, VecDeque};
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

impl ScanPriority {
    #[inline]
    pub const fn target_scan_level(self) -> i8 {
        match self {
            Self::P0Immediate | Self::P3FullElastic => 2,
            Self::P1Viewport | Self::P2FastBackground => 1,
        }
    }
}

pub struct AutoGigGovernor {
    max_workers: usize,
    deck_is_playing: AtomicBool,
    external_booth_mode: AtomicBool,
    on_battery_power: AtomicBool,
}

impl AutoGigGovernor {
    pub fn new(max_workers: usize) -> Self {
        Self {
            max_workers: max_workers.max(1),
            deck_is_playing: AtomicBool::new(false),
            external_booth_mode: AtomicBool::new(false),
            on_battery_power: AtomicBool::new(false),
        }
    }

    pub fn set_deck_playing(&self, playing: bool) {
        self.deck_is_playing.store(playing, Ordering::Relaxed);
    }

    pub fn is_deck_playing(&self) -> bool {
        self.deck_is_playing.load(Ordering::Relaxed)
    }

    pub fn set_external_booth_mode(&self, active: bool) {
        self.external_booth_mode.store(active, Ordering::Relaxed);
    }

    pub fn is_external_booth_mode(&self) -> bool {
        self.external_booth_mode.load(Ordering::Relaxed)
    }

    pub fn set_on_battery_power(&self, on_battery: bool) {
        self.on_battery_power.store(on_battery, Ordering::Relaxed);
    }

    pub fn is_on_battery_power(&self) -> bool {
        self.on_battery_power.load(Ordering::Relaxed)
    }

    pub fn is_throttled_booth_mode(&self) -> bool {
        self.is_deck_playing() || self.is_external_booth_mode() || self.is_on_battery_power()
    }

    pub fn effective_workers(&self) -> usize {
        if self.is_throttled_booth_mode() {
            1
        } else {
            self.max_workers
        }
    }
}

struct SchedulerInner {
    queues: [VecDeque<String>; 4],
    in_flight_or_queued: HashMap<String, ScanPriority>,
    active_in_flight: HashSet<String>,
    completed_level: HashMap<String, i8>,
}

impl SchedulerInner {
    #[inline]
    fn should_skip_for_completed(&self, id: &str, prio: ScanPriority) -> bool {
        if let Some(&lvl) = self.completed_level.get(id) {
            lvl < 0 || lvl >= prio.target_scan_level()
        } else {
            false
        }
    }
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
                active_in_flight: HashSet::new(),
                completed_level: HashMap::new(),
            }),
            cv: Condvar::new(),
            governor: AutoGigGovernor::new(max_workers),
        }
    }

    pub fn seed_completed_levels<I: IntoIterator<Item = (String, i8)>>(&self, items: I) {
        let mut guard = self.inner.lock().unwrap();
        for (id, lvl) in items {
            guard
                .completed_level
                .entry(id)
                .and_modify(|existing| {
                    *existing = if *existing < 0 || lvl < 0 {
                        -1
                    } else {
                        (*existing).max(lvl)
                    };
                })
                .or_insert(if lvl < 0 { -1 } else { lvl });
        }
    }

    pub fn mark_completed(&self, track_id: &str, level: i8) {
        let mut guard = self.inner.lock().unwrap();
        guard.active_in_flight.remove(track_id);
        guard
            .completed_level
            .entry(track_id.to_owned())
            .and_modify(|existing| {
                *existing = if *existing < 0 || level < 0 {
                    -1
                } else {
                    (*existing).max(level)
                };
            })
            .or_insert(if level < 0 { -1 } else { level });
    }

    pub fn completed_level_of(&self, track_id: &str) -> Option<i8> {
        self.inner
            .lock()
            .unwrap()
            .completed_level
            .get(track_id)
            .copied()
    }

    fn pop_from_inner(
        inner: &mut SchedulerInner,
        freeze_p3: bool,
    ) -> Option<(String, ScanPriority)> {
        let max_q = if freeze_p3 { 3 } else { 4 };
        for idx in 0..max_q {
            while let Some(id) = inner.queues[idx].pop_front() {
                let prio = match idx {
                    0 => ScanPriority::P0Immediate,
                    1 => ScanPriority::P1Viewport,
                    2 => ScanPriority::P2FastBackground,
                    _ => ScanPriority::P3FullElastic,
                };
                if inner.in_flight_or_queued.get(&id) == Some(&prio) {
                    inner.in_flight_or_queued.remove(&id);
                    if inner.should_skip_for_completed(&id, prio) {
                        continue;
                    }
                    inner.active_in_flight.insert(id.clone());
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
        if guard.should_skip_for_completed(&id, prio) {
            return;
        }
        if guard.active_in_flight.contains(&id) && prio != ScanPriority::P0Immediate {
            return;
        }
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
            if guard.should_skip_for_completed(&id, prio) || guard.active_in_flight.contains(&id) {
                continue;
            }
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

    pub fn complete(&self, track_id: &str) {
        self.inner.lock().unwrap().active_in_flight.remove(track_id);
    }

    pub fn active_count(&self) -> usize {
        self.inner.lock().unwrap().active_in_flight.len()
    }

    pub fn pop_next(&self) -> Option<(String, ScanPriority)> {
        let freeze_p3 = self.governor.is_throttled_booth_mode();
        let mut guard = self.inner.lock().unwrap();
        Self::pop_from_inner(&mut guard, freeze_p3)
    }

    /// Extrae la siguiente tarea de forma inmediata si hay disponibles,
    /// o espera hasta `timeout` con Condvar (0% CPU en reposo).
    pub fn wait_next(&self, timeout: Duration) -> Option<(String, ScanPriority)> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(item) =
            Self::pop_from_inner(&mut guard, self.governor.is_throttled_booth_mode())
        {
            return Some(item);
        }
        if timeout.is_zero() {
            return None;
        }
        let (mut guard_after_wait, _) = self.cv.wait_timeout(guard, timeout).unwrap();
        Self::pop_from_inner(
            &mut guard_after_wait,
            self.governor.is_throttled_booth_mode(),
        )
    }

    /// Drena colas P2 y P3 (< 100 ms), despierta workers y devuelve elementos drenados.
    pub fn cancel_and_drain_background(&self) -> usize {
        let mut guard = self.inner.lock().unwrap();
        guard.queues[2].clear();
        guard.queues[3].clear();
        let before = guard.in_flight_or_queued.len();
        guard.in_flight_or_queued.retain(|_, prio| {
            *prio == ScanPriority::P0Immediate || *prio == ScanPriority::P1Viewport
        });
        let drained = before.saturating_sub(guard.in_flight_or_queued.len());
        drop(guard);
        self.cv.notify_all();
        drained
    }

    /// Limpia colas P2 (FastBackground) y P3 (FullElastic), preservando P0 y P1.
    pub fn clear_background_queues(&self) {
        let _ = self.cancel_and_drain_background();
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().in_flight_or_queued.len()
    }
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;

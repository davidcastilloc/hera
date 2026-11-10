//! Lock-free retired audio buffer GC channel keeping deallocation off the real-time audio thread.

use std::sync::{Arc, Mutex};
use rtrb::{Consumer, Producer, RingBuffer};
use super::PreallocatedDeckBuffer;

/// Capacity of the ring buffer holding retired deck buffers pending GC.
pub const RETIRED_BUFFER_CHANNEL_CAPACITY: usize = 16;

/// Create a lock-free retired buffer channel pair for off-thread buffer deallocation.
pub(crate) fn create_retired_buffer_channel() -> (
    Producer<Arc<PreallocatedDeckBuffer>>,
    Mutex<Consumer<Arc<PreallocatedDeckBuffer>>>,
) {
    let (tx, rx) = RingBuffer::new(RETIRED_BUFFER_CHANNEL_CAPACITY);
    (tx, Mutex::new(rx))
}

/// Drain retired buffers on a non-audio worker thread, dropping their Arcs safely.
pub(crate) fn drain_retired_buffers(
    rx_mutex: &Mutex<Consumer<Arc<PreallocatedDeckBuffer>>>,
) -> usize {
    let mut drained = 0;
    if let Ok(mut rx) = rx_mutex.lock() {
        while let Ok(buf) = rx.pop() {
            drop(buf);
            drained += 1;
        }
    }
    drained
}

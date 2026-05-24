//! Typed SPSC ring buffer wrapper over `ringbuf`.

use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

pub struct AudioRing {
    capacity: usize,
}

pub struct RingProducer {
    inner: <HeapRb<f32> as Split>::Prod,
}

pub struct RingConsumer {
    inner: <HeapRb<f32> as Split>::Cons,
}

impl AudioRing {
    /// `capacity` is number of f32 samples the buffer can hold.
    pub fn new(capacity: usize) -> (RingProducer, RingConsumer) {
        let rb = HeapRb::<f32>::new(capacity);
        let (prod, cons) = rb.split();
        (RingProducer { inner: prod }, RingConsumer { inner: cons })
    }
}

impl RingProducer {
    /// Push as many samples as fit. Returns count written.
    pub fn push(&mut self, src: &[f32]) -> usize {
        self.inner.push_slice(src)
    }

    pub fn free_len(&self) -> usize {
        self.inner.vacant_len()
    }
}

impl RingConsumer {
    /// Pop up to `dst.len()` samples into `dst`. Returns count read.
    pub fn pop(&mut self, dst: &mut [f32]) -> usize {
        self.inner.pop_slice(dst)
    }

    pub fn occupied_len(&self) -> usize {
        self.inner.occupied_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_pop_round_trips() {
        let (mut p, mut c) = AudioRing::new(16);
        let pushed = p.push(&[1.0, 2.0, 3.0]);
        assert_eq!(pushed, 3);
        let mut out = [0.0_f32; 4];
        let popped = c.pop(&mut out);
        assert_eq!(popped, 3);
        assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn push_caps_at_capacity() {
        let (mut p, mut _c) = AudioRing::new(4);
        let pushed = p.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(pushed, 4);
    }

    #[test]
    fn pop_caps_at_occupied() {
        let (mut p, mut c) = AudioRing::new(8);
        p.push(&[1.0, 2.0]);
        let mut out = [0.0_f32; 5];
        let popped = c.pop(&mut out);
        assert_eq!(popped, 2);
        assert_eq!(&out[..2], &[1.0, 2.0]);
    }

    #[test]
    fn occupied_and_free_track_state() {
        let (mut p, mut c) = AudioRing::new(8);
        assert_eq!(c.occupied_len(), 0);
        assert_eq!(p.free_len(), 8);
        p.push(&[1.0, 2.0, 3.0]);
        assert_eq!(c.occupied_len(), 3);
        assert_eq!(p.free_len(), 5);
    }
}

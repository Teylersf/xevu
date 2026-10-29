//! Display-only random stream. This state never accepts passwords or vault keys.
use chacha20::{
    XChaCha20,
    cipher::{KeyIvInit, StreamCipher},
};
use zeroize::Zeroizing;

pub struct Signal {
    stream: Option<XChaCha20>,
    noise: [u8; 1024],
    pub sample: Option<Zeroizing<[u8; 32]>>,
    pub source: &'static str,
    pub generation: u64,
    pub frame: u64,
    pub motion: bool,
    pub paused: bool,
}
impl Signal {
    pub fn new(seed: Option<([u8; 56], &'static str)>) -> Self {
        let (stream, source) = match seed {
            Some((bytes, source)) => {
                let bytes = Zeroizing::new(bytes);
                (
                    XChaCha20::new_from_slices(&bytes[..32], &bytes[32..]).ok(),
                    source,
                )
            }
            None => (None, "UNAVAILABLE"),
        };
        let mut s = Self {
            stream,
            noise: [0; 1024],
            sample: None,
            source,
            generation: 0,
            frame: 0,
            motion: true,
            paused: false,
        };
        s.advance();
        s
    }
    pub fn active(&self) -> bool {
        self.motion && !self.paused && self.stream.is_some()
    }
    pub fn advance(&mut self) -> bool {
        if !self.active() {
            return false;
        }
        self.noise.fill(0);
        if let Some(stream) = &mut self.stream
            && stream.try_apply_keystream(&mut self.noise).is_ok()
        {
            self.frame = self.frame.wrapping_add(1);
            return true;
        }
        self.stream = None;
        self.noise.fill(0);
        false
    }
    /// A displayed sample is fetched separately from the cosmetic stream.
    /// Failure removes the old value instead of displaying it as freshly drawn.
    pub fn set_sample(&mut self, value: Option<([u8; 32], &'static str)>) {
        self.sample = None;
        match value {
            Some((bytes, source)) => {
                self.sample = Some(Zeroizing::new(bytes));
                self.source = source;
                self.generation = self.generation.wrapping_add(1);
            }
            None => self.source = "UNAVAILABLE",
        }
    }
    pub fn word(&self, i: usize) -> u32 {
        let p = (i % 256) * 4;
        u32::from_le_bytes(self.noise[p..p + 4].try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_display_stream_pause_and_failed_sample() {
        let mut a = Signal::new(Some(([1; 56], "PUBLIC TEST FIXTURE")));
        let b = Signal::new(Some(([2; 56], "PUBLIC TEST FIXTURE")));
        assert_ne!(a.noise, b.noise);
        let first = a.noise;
        assert!(a.advance());
        assert_ne!(first, a.noise);
        a.set_sample(Some(([7; 32], "TEST RNG")));
        let sample = **a.sample.as_ref().unwrap();
        a.advance();
        assert_eq!(sample, **a.sample.as_ref().unwrap());
        a.paused = true;
        let held = a.noise;
        let frame = a.frame;
        assert!(!a.advance());
        assert_eq!(frame, a.frame);
        assert_eq!(held, a.noise);
        a.set_sample(None);
        assert!(a.sample.is_none());
        assert_eq!(a.source, "UNAVAILABLE");
        let mut absent = Signal::new(None);
        assert!(!absent.advance());
        assert!(!absent.active());
        assert!(absent.sample.is_none());
    }
}

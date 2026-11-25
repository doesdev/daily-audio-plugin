//! A delay line read at any distance back, between samples too, so a
//! delay time can be eased or modulated without stepping.

#[derive(Debug, Clone)]
pub struct Delay {
    buf: Vec<f32>,
    at: usize,
}

impl Delay {
    /// Room for up to `max` samples of delay. Allocates.
    pub fn new(max: usize) -> Self {
        Self {
            buf: vec![0.0; max.max(1) + 2],
            at: 0,
        }
    }

    /// The longest delay it can read.
    pub fn max(&self) -> usize {
        self.buf.len() - 2
    }

    pub fn reset(&mut self) {
        self.buf.fill(0.0);
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.buf[self.at] = x;
        self.at = (self.at + 1) % self.buf.len();
    }

    /// The sample written `back` samples ago, where 1 is the last one
    /// written; in between, a straight line between its neighbours.
    #[inline]
    pub fn read(&self, back: f32) -> f32 {
        let back = back.clamp(1.0, self.max() as f32);
        let whole = back as usize;
        let part = back - whole as f32;
        let n = self.buf.len();
        let a = self.buf[(self.at + n - whole) % n];
        let b = self.buf[(self.at + n - whole - 1) % n];
        a + (b - a) * part
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_was_written_that_long_ago() {
        let mut d = Delay::new(8);
        for i in 1..=20 {
            d.write(i as f32);
        }
        let cases = [(1.0, 20.0), (2.0, 19.0), (8.0, 13.0), (2.5, 18.5)];

        for (back, want) in cases {
            assert_eq!(d.read(back), want, "{back} back");
        }
        assert_eq!(d.read(0.0), 20.0, "no nearer than the last sample");
        assert_eq!(d.read(99.0), 13.0, "no further than it has room for");
        d.reset();
        assert_eq!(d.read(3.0), 0.0);
    }
}

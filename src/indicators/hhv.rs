use std::f64::INFINITY;
use std::fmt;

use crate::errors::*;
use crate::{High, Next, Reset, Update};

/// Returns the highest value in a given time frame.
///
/// # Parameters
///
/// * _n_ - size of the time frame (integer greater than 0). Default value is 14.
///
/// # Example
///
/// ```
/// use quantaxis_rs::indicators::HHV;
/// use quantaxis_rs::Next;
///
/// let mut max = HHV::new(3).unwrap();
/// assert_eq!(max.next(7.0), 7.0);
/// assert_eq!(max.next(5.0), 7.0);
/// assert_eq!(max.next(4.0), 7.0);
/// assert_eq!(max.next(4.0), 5.0);
/// assert_eq!(max.next(8.0), 8.0);
/// ```
#[derive(Debug, Clone)]
pub struct HHV {
    n: usize,
    vec: Vec<f64>,
    max_index: usize,
    cur_index: usize,
    pub cached: Vec<f64>,
}

impl HHV {
    pub fn new(n: u32) -> Result<Self> {
        let n = n as usize;

        if n == 0 {
            return Err(Error::from_kind(ErrorKind::InvalidParameter));
        }

        let indicator = Self {
            n: n,
            vec: vec![-INFINITY; n],
            max_index: 0,
            cur_index: 0,
            cached: vec![-INFINITY; n],
        };
        Ok(indicator)
    }

    fn find_max_index(&self) -> usize {
        let mut max = -INFINITY;
        let mut index: usize = 0;

        for (i, &val) in self.vec.iter().enumerate() {
            if val > max {
                max = val;
                index = i;
            }
        }

        index
    }
}

impl Next<f64> for HHV {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        self.cur_index = (self.cur_index + 1) % (self.n as usize);
        self.vec[self.cur_index] = input;

        if input > self.vec[self.max_index] {
            self.max_index = self.cur_index;
        } else if self.max_index == self.cur_index {
            self.max_index = self.find_max_index();
        }
        self.cached.push(self.vec[self.max_index]);
        self.cached.remove(0);
        self.vec[self.max_index]
    }
}

impl Update<f64> for HHV {
    type Output = f64;

    fn update(&mut self, input: f64) -> Self::Output {
        self.vec[self.cur_index] = input;

        if input > self.vec[self.max_index] {
            self.max_index = self.cur_index;
        } else if self.max_index == self.cur_index {
            self.max_index = self.find_max_index();
        }
        self.cached.remove(self.n - 1);
        self.cached.push(self.vec[self.max_index]);

        self.vec[self.max_index]
    }
}

impl<'a, T: High> Next<&'a T> for HHV {
    type Output = f64;

    fn next(&mut self, input: &'a T) -> Self::Output {
        self.next(input.high())
    }
}

impl Reset for HHV {
    fn reset(&mut self) {
        for i in 0..self.n {
            self.vec[i] = -INFINITY;
        }
    }
}

impl Default for HHV {
    fn default() -> Self {
        Self::new(14).unwrap()
    }
}

impl fmt::Display for HHV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MAX({})", self.n)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helper::*;

    use super::*;

    macro_rules! test_indicator {
        ($i:tt) => {
            #[test]
            fn test_indicator() {
                let bar = Bar::new();

                // ensure Default trait is implemented
                let mut indicator = $i::default();

                // ensure Next<f64> is implemented
                let first_output = indicator.next(12.3);

                // ensure next accepts &DataItem as well
                indicator.next(&bar);

                // ensure Reset is implemented and works correctly
                indicator.reset();
                assert_eq!(indicator.next(12.3), first_output);

                // ensure Display is implemented
                format!("{}", indicator);
            }
        };
    }
    test_indicator!(HHV);

    #[test]
    fn test_new() {
        assert!(HHV::new(0).is_err());
        assert!(HHV::new(1).is_ok());
    }

    #[test]
    fn test_next() {
        let mut max = HHV::new(3).unwrap();

        assert_eq!(max.next(4.0), 4.0);
        assert_eq!(max.next(1.2), 4.0);
        assert_eq!(max.next(5.0), 5.0);
        assert_eq!(max.next(3.0), 5.0);
        assert_eq!(max.next(4.0), 5.0);
        assert_eq!(max.next(0.0), 4.0);
        assert_eq!(max.next(-1.0), 4.0);
        assert_eq!(max.next(-2.0), 0.0);
        assert_eq!(max.next(-1.5), -1.0);
    }

    #[test]
    fn test_update() {
        let mut max = HHV::new(3).unwrap();

        assert_eq!(max.next(4.0), 4.0);
        assert_eq!(max.next(1.2), 4.0);
        assert_eq!(max.next(5.0), 5.0);
        assert_eq!(max.update(3.0), 4.0);
        assert_eq!(max.next(3.0), 3.0);
    }

    #[test]
    fn test_next_with_bars() {
        fn bar(high: f64) -> Bar {
            Bar::new().high(high)
        }

        let mut max = HHV::new(2).unwrap();

        assert_eq!(max.next(&bar(1.1)), 1.1);
        assert_eq!(max.next(&bar(4.0)), 4.0);
        assert_eq!(max.next(&bar(3.5)), 4.0);
        assert_eq!(max.next(&bar(2.0)), 3.5);
    }

    #[test]
    fn test_reset() {
        let mut max = HHV::new(100).unwrap();
        assert_eq!(max.next(4.0), 4.0);
        assert_eq!(max.next(10.0), 10.0);
        assert_eq!(max.next(4.0), 10.0);

        max.reset();
        assert_eq!(max.next(4.0), 4.0);
    }

    #[test]
    fn test_default() {
        HHV::default();
    }

    #[test]
    fn test_display() {
        let indicator = HHV::new(7).unwrap();
        assert_eq!(format!("{}", indicator), "MAX(7)");
    }
}

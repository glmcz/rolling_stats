#![no_std]
use byte_converter::ByteConverter;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

mod byte_converter;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use byte_converter::{std_error, std_info, std_warn};
// goal is to save int32 user input until it reach user defined windows size.
// input should be as sequence of bytes.
// and each byte can have value -128 up to 255.
// compute statistics over recent values. It means when new value is added we need to compute new statistics
// implement lib with std and without rust std -- it means fixed compile size, prefering stack over heap
// rand_distr can be used without std

struct RollingStats {
    // use byteConverter obejct for handling input
    input_i32: ByteConverter,
    mean: f32,
    std_dev: f32,
    std_dis_samle: f32,
}

impl RollingStats {
    pub fn default() -> Self {
        Self {
            input_i32: ByteConverter::init(3),
            mean: 0.0,
            std_dev: 0.0,
            std_dis_samle: 0.0,
        }
    }

    // arithmetic mean
    fn mean(&mut self) -> f32 {
        if *self.input_i32.get_sum() <= 0 || self.input_i32.get_buf().is_empty() {
            return 0.0;
        }
        self.mean = *self.input_i32.get_sum() as f32 / self.input_i32.get_buf().len() as f32;
        self.mean
    }

    // standard deviation
    // use to tell us how much each value is far from mean <=> find out how many people are dissconnected from mainstream matrix.
    fn std_deviation(&mut self) -> f32 {
        // should never reach
        if self.input_i32.get_buf().is_empty() {
            std_error("std_deviation can`t be computed from empty buf_current");
            return 0.0;
        }
        let mut square_diffs = 0.0; // need to square because diff can have pos or neg sign
        for value in self.input_i32.get_buf().iter() {
            let diff = (*value as f32) - self.mean;
            square_diffs += diff * diff;
        }
        // compute variance
        let variance = square_diffs / self.input_i32.get_buf().len() as f32;
        self.std_dev = variance.sqrt();
        self.std_dev
    }

    fn std_distribution(&mut self) -> f32 {
        // should never reach
        if self.input_i32.get_buf().is_empty() {
            std_error("std_deviation can`t be computed from empty buf_current");
            return 0.0;
        }

        let mut rng = thread_rng();
        let normal_dis = Normal::new(self.mean, self.std_dev).unwrap();
        self.std_dis_samle = normal_dis.sample(&mut rng).clone();
        self.std_dis_samle
    }

    #[cfg(not(feature = "std"))]
    pub fn write_no_std(&mut self, buf: &[u8]) {
        // clear previous statistics
        self.input_i32.set_sum(0);
        self.mean = 0.0;
        self.std_dev = 0.0;
        self.std_dis_samle = 0.0;
        self.input_i32.clear_buf();

        if buf.len() > 0 {
            // need at leat 2 (bytes) values in first call to do statistics, in second call we need at least 1 byte
            self.input_i32.convert_bytes_to_i32(buf);
        } else {
            // need to be redirect in embedded or rather use https://github.com/knurling-rs/defmt
            //println!("can`t proceed with empty value. Put at least one bytes into the write input")
        }
    }
}

#[cfg(feature = "std")]
impl std::io::Write for RollingStats {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // clear previous statistics
        self.input_i32.set_sum(0);
        self.mean = 0.0;
        self.std_dev = 0.0;
        self.std_dis_samle = 0.0;
        self.input_i32.clear_buf();

        if buf.len() > 0 {
            // need at leat 2 (bytes) values in first call to do statistics, in second call we need at least 1 byte
            self.input_i32.convert_bytes_to_i32(buf);
        } else {
            std_error("can`t proceed with empty value. Put at least one bytes into the write input")
        }
        Ok(self.input_i32.get_buf().len() * 4)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // fix my error handling
        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use std::io::Write;
    use crate::std::string::ToString;

    use super::*;

    // it work
    #[test]
    fn test_one_write() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
        assert_eq!(stats.std_deviation(), 0.816496611);
        std_info(stats.std_distribution().to_string().as_str());
    }

    #[test]
    fn test_one_write_with_less_values() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2]);
        assert_eq!(stats.mean(), 1.5);
    }
    #[test]
    fn test_one_byte_reminder() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0]);
        _ = stats.write(&[0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
    }

    #[test]
    fn test_one_byte_reminder_second_reminder() {
        let mut stats = RollingStats::default();
        // we are reconstructiing splited 0,0,0,1 = 1 next we read 2 with reminder 0,0,6
        // and therefore window_size is 3 we are taking 0,0,0,1 from previous write call.
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0]);
        _ = stats.write(&[0, 0, 1, 0, 0, 0, 2, 0, 0, 6]); //reminder 0,0,6
        assert_eq!(stats.mean(), 1.33333337);
    }

    #[test]
    fn test_one_byte_reminder_with_bigger_window_size() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 7]); // reminder not interested us!!!!!
        assert_eq!(stats.mean(), 2.0);
        _ = stats.write(&[0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 3.0);
    }

    #[test]
    fn test_two_byte_reminder() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0]);
        _ = stats.write(&[0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
    }

    #[test]
    fn test_three_byte_reminder() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
        _ = stats.write(&[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
    }

    #[test]
    fn test_only_one_byte_reminder() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0]);
        _ = stats.write(&[2]); // added previous values up to window_size
        assert_eq!(stats.mean(), 2.0);
    }

    #[test]
    fn test_incomplete_i32() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 1]);
        _ = stats.write(&[0, 0, 1]);
        assert_eq!(stats.mean(), 0.0);
    }
}

#[cfg(test)]
#[cfg(not(feature = "std"))]
mod tests_no_std {    
   use super::RollingStats;

    // it work
    #[test]
    fn test_one_write() {
        let mut stats = RollingStats::default();
        _ = stats.write_no_std(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
        assert_eq!(stats.std_deviation(), 0.816496611);
        std_info(stats.std_distribution());
    }
}
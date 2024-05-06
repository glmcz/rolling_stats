use byteorder::{BigEndian, ByteOrder, LittleEndian};
use core::cmp;
use log::{error, info, warn};
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

// goal is to save int32 user input until it reach user defined windows size.
// input should be as sequence of bytes.
// and each byte can have value -128 up to 255.
// compute statistics over recent values. It means when new value is added we need to compute new statistics
// implement lib with std and without rust std -- it means fixed compile size, prefering stack 
// rand_distr can be used without std

// TODO tasks:
// replace heap Vec<> with fixed stack arrrays, but basically we can use alloc crate instead of std
// add r_byte into struct and return num of bytes grabed from input buf

struct RollingStats {
    // self.buf_last is used only for filling window_size gap for current values
    buf_last: Vec<i32>,
    // all values in current write restricted by window_siez
    buf_current: Vec<i32>,
    // saving uncompleted bytes from write call
    buf_remainder: Vec<u8>,
    // len of seq of bytes
    window_size: usize,
    // sum of all input i32 values
    sum: i32,
    mean: f32,
    std_dev: f32,
    std_dis_samle: f32,
    // are we using big or little endian order, default is big (true)
    little_endian: bool,
}

enum NoStdError {
    EmptyBuffer,
    FileWrite,
}

#[cfg(not(feature = "std"))]
trait NoStdWriter {
    fn write(&mut self, buf: &[u8]) -> Result<(), NoStdError>;
}

impl RollingStats {
    pub fn default() -> Self {
        Self {
            buf_last: Vec::new(),
            buf_current: Vec::new(),
            buf_remainder: Vec::new(), // can be only sequence of < 4 bytes
            window_size: 3,
            sum: 0,
            mean: 0.0,
            std_dev: 0.0,
            std_dis_samle: 0.0,
            little_endian: false,
        }
    }

    pub fn get_window_size(&self) -> usize {
        self.window_size * 4
    }

    // iterate until window_size -1 and check for reminder
    // if not add next iteration into buf_current and break
    // or add into buf_remainder
    pub fn read_little_endian(&mut self, buf: &[u8], start_index: usize) {
        self.little_endian = true;
        let window_size = self.get_window_size();
        let max_size = cmp::min(buf.len(), window_size);
        let mut slice = buf[start_index..max_size].chunks_exact(4);

        for chunk in slice.by_ref().into_iter() {
            // enumerate takes ownership
            let value = LittleEndian::read_i32(chunk);
            self.buf_current.push(value);
            self.sum += value;
        }

        // reminder bigger than window_size is not interested
        if !slice.remainder().is_empty() && buf.len() <= window_size {
            self.buf_remainder.extend(slice.remainder())
        }
    }

    pub fn read_big_endian(&mut self, buf: &[u8], start_index: usize) {
        self.little_endian = true;
        let window_size = self.get_window_size();
        let max_size = cmp::min(buf.len(), window_size);
        let mut slice = buf[start_index..max_size].chunks_exact(4);

        for chunk in slice.by_ref().into_iter() {
            // enumerate takes ownership
            let value = BigEndian::read_i32(chunk);
            self.buf_current.push(value);
            self.sum += value;
        }

        // reminder bigger than window_size is not interested
        if !slice.remainder().is_empty() && buf.len() <= window_size {
            self.buf_remainder.extend(slice.remainder())
        }
    }

    // add togethet i32 value from saved remainder
    // case 1 byte in remaining 3 bytes + another in current scope
    // case 2 bytes remaining 2 bytes in current scope
    // case 3 bytes remaining 1 bytes in current scope
    pub fn reconstruct_i32_bytes(&self, buf: &[u8]) -> Option<Vec<u8>> {
        // max remainder is 3
        match self.buf_remainder.len() {
            1 => {
                // if one is inside buf_remainder it means that we need 3 from buf
                if let Some(fist_part) = self.buf_remainder.get(0..1) {
                    if let Some(sec_part) = buf.get(0..3) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        warn!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    error!("could not get 1 index of buf_remainder");
                    None
                }
            }
            2 => {
                if let Some(fist_part) = self.buf_remainder.get(0..2) {
                    if let Some(sec_part) = buf.get(0..2) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        warn!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    error!("could not get 1 index of buf_remainder");
                    None
                }
            }
            3 => {
                if let Some(fist_part) = self.buf_remainder.get(0..3) {
                    if let Some(sec_part) = buf.get(0..1) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        warn!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    error!("could not get 1 index of buf_remainder");
                    None
                }
            }
            _ => {
                error!("could get together from byte sequences i32 value");
                None
            }
        }
    }

    // input: &buf slice with bytes
    // in this fn we are taking current write and save it into buf_currenct
    // For now skipping returning number of succesfully converted bytes.
    pub fn convert_bytes_to_f32(&mut self, buf: &[u8]) {
        // split bytes sequence by 4
        // pre-buffering
        self.buf_current.reserve(self.get_window_size());

        // basically we need only 8 bytes for mean(), but for now we don`t support less numbers in
        // first call then window_size. Otherwise we would need to adjust old value adding
        if self.buf_remainder.is_empty() {
            if buf.len() < 8 {
                warn!("first call doesn`t have enough values for making statistics");
            } else {
                if buf[0] > buf[3] {
                    // if there are more values on the input it will be skipped
                    self.read_little_endian(buf, 0);
                } else {
                    // if there are more values on the input it will be skipped
                    self.read_big_endian(buf, 0);
                }
            }
        } else {
            // Next write call.
            // Take a look into self.buf_remainder and try to reconstruct i32 from next write call
            // TODO add r_byte into struct and return num of bytes grabed from input buf
            if let Some(r_byte) = self.reconstruct_i32_bytes(&buf) {
                if r_byte[0] > r_byte[3] {
                    // TODO refactor me
                    let value = LittleEndian::read_i32(r_byte.as_slice());
                    self.buf_current.push(value);
                    self.sum += value;

                    // after we add r_bytes we need to skip reconstructed bytes and adjust window_size
                    self.read_little_endian(buf, 4 - self.buf_remainder.len());
                } else {
                    // TODO refactor me
                    let value = BigEndian::read_i32(r_byte.as_slice());
                    self.buf_current.push(value);
                    self.sum += value;

                    // after we add r_bytes we need to skip reconstructed bytes and adjust window_size
                    self.read_big_endian(buf, 4 - self.buf_remainder.len());
                }
            } else {
                error!("could not get together i32 value from previous and next write call");
            }
            // at the end clear buf_remaining to be ready for next write iteration
            self.buf_remainder.clear();
        }

        // if we are getting less values than windows size. We need to take a look into the self.buf_last
        // for some values to get full window size.
        if !self.buf_last.is_empty() && self.buf_current.len() < self.window_size {
            // add last values to the current input. In Assigment was not specify if the values should be
            // used before or after user input.
            let gap_len = self.window_size - self.buf_current.len();

            // check if buf_last has enought values to fill a gap in buf_current
            if self.buf_last.len() >= gap_len {
                self.buf_current
                    .extend_from_slice(&self.buf_last[0..gap_len]);
                // need to also add sum of values
                self.sum += self.buf_last[0..gap_len].iter().sum::<i32>();
            } else {
                // add at least what we have and warn that we don`t have a full window_size`
                self.buf_current.extend_from_slice(&self.buf_last);
                // need to also add sum of values
                self.sum += self.buf_last.iter().sum::<i32>();
                warn!(
                    "buf_current doesn`t fill window_size constraint, either from last write call."
                )
            }
        }

        // we are going to save values for later usage
        // need to store only full window_size
        if self.buf_last.is_empty() {
            self.buf_last.clear();
            self.buf_last
                .extend_from_slice(&self.buf_current.as_slice());
        }
        // compute statistics staff
        info!("convertion of byte sequence into i32 values is complete");
    }

    // arithmetic mean
    fn mean(&mut self) -> f32 {
        if self.sum <= 0 || self.buf_current.is_empty() {
            return 0.0;
        }
        self.mean = self.sum as f32 / self.buf_current.len() as f32;
        self.mean
    }

    // standard deviation
    // use to tell us how much each value is far from mean <=> find out how many people are dissconnected from mainstream matrix.
    fn std_deviation(&mut self) -> f32 {
        // should never reach
        if self.buf_current.is_empty() {
            error!("std_deviation can`t be computed from empty buf_current");
            return 0.0;
        }
        let mut square_diffs = 0.0; // need to square because diff can have pos or neg sign
        for value in self.buf_current.iter() {
            let diff = (*value as f32) - self.mean;
            square_diffs += diff * diff;
        }
        // compute variance = rozptyl
        let variance = square_diffs / self.buf_current.len() as f32;
        self.std_dev = variance.sqrt(); // standartni odchylka je odmocnina rozptylu
        self.std_dev
    }

    fn std_distribution(&mut self) -> f32 {
        // should never reach
        if self.buf_current.is_empty() {
            error!("std_deviation can`t be computed from empty buf_current");
            return 0.0;
        }

        let mut rng = thread_rng();
        let normal_dis = Normal::new(self.mean, self.std_dev).unwrap();
        self.std_dis_samle = normal_dis.sample(&mut rng).clone();
        self.std_dis_samle
    }
}

impl std::io::Write for RollingStats {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // clear previous statistics
        self.sum = 0;
        self.mean = 0.0;
        self.std_dev = 0.0;
        self.std_dis_samle = 0.0;
        self.buf_current.clear();

        if buf.len() > 0 {
            // need at leat 2 (bytes) values in first call to do statistics, in second call we need at least 1 byte
            self.convert_bytes_to_f32(buf);
        } else {
            error!("can`t proceed with empty value. Put at least one bytes into the write input")
        }
        Ok(self.buf_current.len() * 4)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // fix my error handling
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl NoStdWriter for RollingStats {
    fn write(&mut self, buf: &[u8]) -> Result<(), NoStdError> {
        // clear previous statistics
        self.mean = 0.0;
        self.std_dev = 0.0;
        self.std_dis_samle = 0.0;

        if buf.len() > 0 {
            // need at leat 2 (bytes) values, if one we use last values if they are available
            // fix my error handling
            self.convert_bytes_to_f32(buf);
        } else {
            error!("can`t proceed with empty value. Put at least one bytes into the write input")
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    // it work
    #[test]
    fn test_one_write() {
        let mut stats = RollingStats::default();
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]);
        assert_eq!(stats.mean(), 2.0);
        assert_eq!(stats.std_deviation(), 0.816496611);
        println!("Sample from std distribution {}", stats.std_distribution());
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
        _ = stats.write(&[0, 0, 0, 1, 0, 0, 0, 2, 0]);
        _ = stats.write(&[0, 0, 1, 0, 0, 0, 2, 0, 0, 6]); //reminder 0,0,6
        assert_eq!(stats.mean(), 1.5);
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

// TODO testing
// otestovat a napsat test jestli se spravne pridavaji z min write callu hodnoty
// taky aby:
// case 1 buf_last obsahuje max window_size values a checkujeme jestli funguje if ktery znemoznuje pridat do buf_current vice elementu
// case 2 buf_last obsahuje 1 anebo mene alementu a v next write call mame taky jeden element. Mela by vyskocit hlaska nedostatek hodnot v buf_current

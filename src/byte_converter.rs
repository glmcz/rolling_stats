use byteorder::{BigEndian, ByteOrder, LittleEndian};
use core::cmp;

use core::default::Default;
use core::marker::Copy;
use core::slice::Iter;

use crate::logs::Logger;

// in no_std we have not stdout, stderr output so we need to use logger only in std environment.
#[cfg(not(feature = "std"))]
use crate::logs::NoStdLogger;
#[cfg(not(feature = "std"))]
pub const LOG: NoStdLogger = NoStdLogger;

#[cfg(feature = "std")]
use crate::logs::StdLogger;
#[cfg(feature = "std")]
pub const LOG: StdLogger = StdLogger;

// Vec<> alternative for no_std. I know that we can still use Vec<> from core crate, but for small window size inputs
// it`s better to use fixed static array saved on stack, so we are not slow down by accessing heap.
pub struct FixedArray<T, const N: usize> {
    data: [T; N],
    counter: usize,
}

pub enum FixeArrayError {
    OutOfTheBounds,
    FullCapacity,
    ElementNotFound,
}

impl<T: Default + Copy, const N: usize> FixedArray<T, N> {
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            counter: 0,
        }
    }

    // append to the end of array
    pub fn push(&mut self, element: T) {
        if self.counter < N {
            self.data[self.counter] = element;
            self.counter += 1;
        } else {
            LOG.error("array is out of the bounds");
        }
    }

    // get element from fixed array
    pub fn get(&self, range: core::ops::Range<usize>) -> Option<&[T]> {
        if range.start <= range.end && range.end <= N {
            let len = range.end - range.start;
            Some(self.slice(range.start, len))
        } else {
            None
        }
    }

    // helper fn for slice in get method
    fn slice(&self, start: usize, len: usize) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr().add(start), len) }
    }

    // clear whole array
    pub fn clear(&mut self) {
        self.counter = 0;
        self.data.fill(T::default());
    }

    pub fn is_empty(&self) -> bool {
        if self.counter <= 0 {
            return true;
        }
        false
    }

    // it should iter through array and get sum of all values
    pub fn iter(&self) -> Iter<T> {
        self.data[..self.counter].iter()
    }

    pub fn len(&self) -> usize {
        self.counter
    }

    // do not handle adding elements up to the available space and then return an error
    // or a partial success indication.
    // possible feature for upgrade.
    pub fn extend_by_slice(&mut self, slice: &[T]) -> Result<(), FixeArrayError> {
        let size = slice.len();
        let free_size = N - self.counter;
        if size <= 0 {
            LOG.error("slice has no element");
            return Err(FixeArrayError::ElementNotFound);
        }

        if size > N {
            LOG.error("slice size is bigger that size of array, can`t add all elements");
            return Err(FixeArrayError::OutOfTheBounds);
        }

        if size > free_size {
            LOG.error(
                "capacity of array would be exceeted, try to delete old data in array and continue",
            );
            return Err(FixeArrayError::FullCapacity);
        } else {
            for i in 0..size {
                self.data[self.counter] = slice[i];
                self.counter += 1;
            }
            Ok(())
        }
    }

    pub fn extend_by_array(&mut self, array: &FixedArray<T, N>) -> Result<(), FixeArrayError> {
        let size = array.len();
        let free_size = N - self.counter;
        if size <= 0 {
            LOG.error("slice has no element");
            return Err(FixeArrayError::ElementNotFound);
        }

        if size > N {
            LOG.error("slice size is bigger that size of array, can`t add all elements");
            return Err(FixeArrayError::OutOfTheBounds);
        }

        if size > free_size {
            LOG.error(
                "capacity of array would be exceeted, try to delete old data in array and continue",
            );
            return Err(FixeArrayError::FullCapacity);
        } else {
            for i in 0..free_size {
                self.data[self.counter] = array.data[i];
                self.counter += 1;
            }
            Ok(())
        }
    }
}
pub struct ByteConverter {
    // len of seq of bytes. Defined by user at the start
    window_size: usize,
    // self.buf_last is used only for filling window_size gap for current converted values
    buf_last: FixedArray<i32, 255>,
    // all converted values in current write restricted by window_siez
    buf_current: FixedArray<i32, 255>,
    // saving uncompleted bytes from write call. There sould be only up to 3 bytes => 3 * 8 = 24
    buf_remainder: FixedArray<u8, 4>,
    // are we using big or little endian order, default is big (true)
    litle_endian: bool,
    // sum of all input i32 values. It is convinient to count it while converting input
    sum: i32, // remove pub TODO
}

impl ByteConverter {
    pub fn init(window_size: usize) -> ByteConverter {
        if window_size > 0 {
            ByteConverter {
                window_size: window_size,
                buf_last: FixedArray::<i32, 255>::new(),
                buf_current: FixedArray::<i32, 255>::new(),
                buf_remainder: FixedArray::<u8, 4>::new(),
                litle_endian: false,
                sum: 0,
            }
        } else {
            LOG.warn("please use real windo_size for input. Using default value = 3");
            ByteConverter {
                window_size: 3,
                buf_last: FixedArray::<i32, 255>::new(),
                buf_current: FixedArray::<i32, 255>::new(),
                buf_remainder: FixedArray::<u8, 4>::new(),
                litle_endian: false,
                sum: 0,
            }
        }
    }

    pub fn get_window_size(&self) -> usize {
        self.window_size * 4
    }

    pub fn get_sum(&self) -> &i32 {
        &self.sum
    }

    pub fn get_buf(&self) -> &FixedArray<i32, 255> {
        &self.buf_current
    }

    pub fn set_sum(&mut self, value: i32) {
        self.sum = value;
    }

    // clear current buffer
    pub fn clear_buf(&mut self) {
        self.buf_current.clear();
    }

    // if not add next iteration into buf_current and break
    // or add into buf_remainder
    pub fn read_little_endians(&mut self, buf: &[u8], start_index: usize) {
        self.litle_endian = true;
        let window_size = self.get_window_size();
        let max_size = cmp::min(buf.len(), window_size);
        let mut slice = buf[start_index..max_size].chunks_exact(4);

        for chunk in slice.by_ref().into_iter() {
            let value = LittleEndian::read_i32(chunk);
            self.buf_current.push(value);
            self.sum += value;
        }

        // reminder bigger than window_size is not interested
        if !slice.remainder().is_empty() && buf.len() <= window_size {
            _ = self.buf_remainder.extend_by_slice(slice.remainder());
        }
    }

    pub fn read_big_endians(&mut self, buf: &[u8], start_index: usize) {
        self.litle_endian = true;
        let window_size = self.get_window_size();
        let max_size = cmp::min(buf.len(), window_size);
        let mut slice = buf[start_index..max_size].chunks_exact(4);

        for chunk in slice.by_ref().into_iter() {
            let value = BigEndian::read_i32(chunk);
            self.buf_current.push(value);
            self.sum += value;
        }

        // reminder bigger than window_size is not interested
        if !slice.remainder().is_empty() && buf.len() <= window_size {
            _ = self.buf_remainder.extend_by_slice(slice.remainder());
        }
    }

    // add togethet i32 value from saved remainder
    // case 1 byte in remaining 3 bytes + another in current scope
    // case 2 bytes remaining 2 bytes in current scope
    // case 3 bytes remaining 1 bytes in current scope
    pub fn reconstruct_i32_bytes(&self, buf: &[u8]) -> Option<FixedArray<u8, 4>> {
        // max remainder is 3
        match self.buf_remainder.counter {
            1 => {
                // if one is inside buf_remainder it means that we need 3 from buf
                if let Some(fist_part) = self.buf_remainder.data.get(0..1) {
                    if let Some(sec_part) = buf.get(0..3) {
                        let mut r_byte = FixedArray::<u8, 4>::new();
                        _ = r_byte.extend_by_slice(fist_part);
                        _ = r_byte.extend_by_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        LOG.warn("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    LOG.error("could not get 1 index of buf_remainder");
                    None
                }
            }
            2 => {
                if let Some(fist_part) = self.buf_remainder.get(0..2) {
                    if let Some(sec_part) = buf.get(0..2) {
                        let mut r_byte = FixedArray::<u8, 4>::new();
                        _ = r_byte.extend_by_slice(fist_part);
                        _ = r_byte.extend_by_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        LOG.warn("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    LOG.error("could not get 1 index of buf_remainder");
                    None
                }
            }
            3 => {
                if let Some(fist_part) = self.buf_remainder.get(0..3) {
                    if let Some(sec_part) = buf.get(0..1) {
                        let mut r_byte = FixedArray::<u8, 4>::new();
                        _ = r_byte.extend_by_slice(fist_part);
                        _ = r_byte.extend_by_slice(sec_part);
                        return Some(r_byte);
                    } else {
                        LOG.warn("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                } else {
                    LOG.error("could not get 1 index of buf_remainder");
                    None
                }
            }
            _ => {
                LOG.error("could get together from byte sequences i32 value");
                None
            }
        }
    }

    // input: &buf slice with bytes
    // in this fn we are taking current write and save it into buf_currenct
    // For now skipping returning number of succesfully converted bytes.
    pub fn convert_bytes_to_i32(&mut self, buf: &[u8]) {
        // split bytes sequence by 4

        // basically we need only 8 bytes for mean(), but for now we don`t support less numbers in
        // first call then window_size. Otherwise we would need to adjust old value adding
        if self.buf_remainder.is_empty() {
            if buf.len() < 8 {
                LOG.warn("first call doesn`t have enough values for making statistics");
            } else {
                if buf[0] > buf[3] {
                    // if there are more values on the input it will be skipped
                    self.read_little_endians(buf, 0);
                } else {
                    // if there are more values on the input it will be skipped
                    self.read_big_endians(buf, 0);
                }
            }
        } else {
            // Next write call.
            // Take a look into self.buf_remainder and try to reconstruct i32 from next write call
            // TODO add r_byte into struct and return num of bytes grabed from input buf
            if let Some(r_byte) = self.reconstruct_i32_bytes(&buf) {
                if r_byte.data[0] > r_byte.data[3] {
                    // TODO refactor me
                    let value = LittleEndian::read_i32(r_byte.data.get(..).unwrap());
                    self.buf_current.push(value);
                    self.sum += value;

                    // after we add r_bytes we need to skip reconstructed bytes and adjust window_size
                    self.read_little_endians(buf, 4 - self.buf_remainder.len());
                } else {
                    // TODO refactor me
                    let value = BigEndian::read_i32(r_byte.data.get(..).unwrap());
                    self.buf_current.push(value);
                    self.sum += value;

                    // after we add r_bytes we need to skip reconstructed bytes and adjust window_size
                    self.read_big_endians(buf, 4 - self.buf_remainder.len());
                }
            } else {
                LOG.error("could not get together i32 value from previous and next write call");
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
                if let Some(extend_data) = self.buf_last.get(0..gap_len) {
                    _ = self.buf_current.extend_by_slice(extend_data);
                    // need to also add sum of values
                    self.sum += self.buf_last.data[0..gap_len].iter().sum::<i32>();
                }
            } else {
                // add at least what we have and warn that we don`t have a full window_size`
                _ = self.buf_current.extend_by_array(&self.buf_last);
                // need to also add sum of values
                self.sum += self.buf_last.iter().sum::<i32>();
                LOG.warn(
                    "buf_current doesn`t fill window_size constraint, either from last write call.",
                )
            }
        }

        // we are going to save values for later usage
        // need to store only full window_size
        if self.buf_last.is_empty() {
            self.buf_last.clear();

            _ = self.buf_last.extend_by_array(&self.buf_current);
        }
        // compute statistics staff
        LOG.info("convertion of byte sequence into i32 values is complete");
    }
}

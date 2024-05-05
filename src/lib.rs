use byteorder::{BigEndian, ByteOrder, LittleEndian};
use core::cmp;

// goal is to save int32 user input until it reach user defined windows size.
// input should be as sequence of bytes. 
// and each byte can have value -128 up to 255.
// compute statistics over recent values. It means when new value is added we need to compute new statistics
// implement lib with std and without rust std

struct RollingStats {
    // self.buf_last is used only for filling window_size gap for current values
    // it doesn`t save all values
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
        let mut slice= buf[start_index..max_size].chunks_exact(4);
       
        for chunk in slice.by_ref().into_iter() { // enumerate takes ownership
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
        let mut slice= buf[start_index..max_size].chunks_exact(4);
       
        for chunk in slice.by_ref().into_iter() { // enumerate takes ownership
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
    pub fn reconstruct_i32_bytes(&self, buf: &[u8])-> Option<Vec<u8>> {
        // max remainder is 3
        match self.buf_remainder.len() {
            1 => {
                // if one is inside buf_remainder it means that we need 3 from buf
                if let Some(fist_part) = self.buf_remainder.get(0..1){
                    if let Some(sec_part) = buf.get(0..3) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte)
                    }else {
                        println!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                }else {
                    println!("could not get 1 index of buf_remainder");
                    None
                }                

                
            }
            2 => {
                 if let Some(fist_part) = self.buf_remainder.get(0..2){
                    if let Some(sec_part) = buf.get(0..2) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte)
                    }else {
                        println!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                }else {
                    println!("could not get 1 index of buf_remainder");
                    None
                }
            }
            3 => {
                if let Some(fist_part) = self.buf_remainder.get(0..3){
                    if let Some(sec_part) = buf.get(0..1) {
                        let mut r_byte = Vec::with_capacity(fist_part.len() + sec_part.len());
                        r_byte.extend_from_slice(fist_part);
                        r_byte.extend_from_slice(sec_part);
                        return Some(r_byte)
                    }else {
                        println!("cannot happend, because we are checking buf input at the start of conversion");
                        None
                    }
                }else {
                    println!("could not get 1 index of buf_remainder");
                    None
                }
            }
            _ => {
                println!("could get together from byte sequences i32 value");
                None
            }
        }
    }

    // input: &buf array with bytes
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
                println!("first call doesn`t have enough values for making statistics");
            }
            else 
            {
                if buf[0] > buf[3] {
                    // if there are more values on the input it will be skipped
                    self.read_little_endian(buf, 0);
                } 
                else {
                    // if there are more values on the input it will be skipped
                    self.read_big_endian(buf, 0);
                }
            }

        }
        else {
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
                } 
                else {
                    // TODO refactor me
                    let value = BigEndian::read_i32(r_byte.as_slice());
                    self.buf_current.push(value);
                    self.sum += value;

                    // after we add r_bytes we need to skip reconstructed bytes and adjust window_size
                    self.read_big_endian(buf, 4 - self.buf_remainder.len());
                }
            } else {
                println!("could not get together i32 value from previous and next write call");
            }
            // at the end clear buf_remaining to be ready for next write iteration
            self.buf_remainder.clear();
        }

        // if we are getting less values than windows size. We need to take a look into the self.buf_last
        // for some values to get full window size.
        if !self.buf_last.is_empty() && buf.len() <= self.get_window_size(){
            // add last values to the current input. In Assigment was not specify if the values should be
            // used before or after user input.
            self.buf_current.extend_from_slice(&self.buf_last);
            // need to also add sum of values
            self.sum += self.buf_last.iter().sum::<i32>();
        } 
  
        // at the end of converting we are going to save values for later usege
        if self.buf_last.is_empty() && buf.len() >= self.window_size*4{
            self.buf_last.extend_from_slice(&self.buf_current.as_slice());
        }else {
            // discart old values and add new one
            // TODO need to handle buf.len() >= self.window_size * 4
            self.buf_last.clear();
            self.buf_last.extend_from_slice(&self.buf_current.as_slice());

        }
        // compute statistics staff
    }

    // arithmetic mean
    fn mean(&self) -> f32 {
        let mut output = 0.0;
        if self.sum <= 0 || self.buf_current.len() <= 0 {
            return 0.0
        }
        output = self.sum as f32 / self.buf_current.len() as f32;
        output
    }

    fn std_deviation(&self) -> f32 {
        0.0
    }

    fn std_distribution(&self) -> f32 {
        0.0
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

        // first check if write call can do statistics
        // basically it needs to be 4 because in second call there can be only one bytes
        // and we should use previous values from the first call
        if buf.len() > 0 {
            // need at leat 2 (bytes) values, if one we use last values if they are available
            // fix my error handling
            self.convert_bytes_to_f32(buf);
        } else {
            println!("can`t proceed with empty value. Put at least one bytes into the write input")
        }
        Ok(self.buf_current.len()*4)
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
            println!("can`t proceed with empty value. Put at least one bytes into the write input")
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
    }

    // it should failed
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
        _ = stats.write(&[0, 0, 0, 2, 0, 0,0, 2, 0, 0, 0, 5, 0, 0, 0, 4]);
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

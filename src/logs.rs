
use core::str;

#[cfg(feature = "std")]
use log::{error, info, warn};


pub trait Logger {
    fn error(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn info(&self, msg: &str);
}

#[cfg(feature = "std")]
pub struct StdLogger;

#[cfg(not(feature = "std"))]
pub struct NoStdLogger;

#[cfg(feature = "std")]
impl Logger for StdLogger {
    fn error(&self, msg: &str) {
        error!("{}", &msg);
    }

    fn warn(&self, msg: &str) {
        warn!("{}", &msg);
    }

    fn info(&self, msg: &str) {
        info!("{}", &msg);
    }
}

#[cfg(not(feature = "std"))]
impl Logger for NoStdLogger {
    fn error(&self, msg: &str) {
       // write into file or to the serial so we can see debug msg
       //todo!();
    }

    fn warn(&self, msg: &str) {
         // write into file or to the serial so we can see debug msg
       //todo!();
    }

    fn info(&self, msg: &str) {
       // write into file or to the serial so we can see debug msg
       //todo!();
    }
}

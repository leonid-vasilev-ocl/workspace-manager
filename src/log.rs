use anyhow::{anyhow, Result};
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Mutex, MutexGuard, OnceLock},
};

static LOG: OnceLock<Mutex<Log>> = OnceLock::new();

pub fn init_logger() -> Result<()> {
    let log = Log::new()?;
    let mutex = Mutex::new(log);
    LOG.set(mutex).map_err(|_| anyhow!("Can't create logger"))?;
    Ok(())
}

fn get_logger() -> Result<MutexGuard<'static, Log>> {
    let mutex = LOG.get().ok_or(anyhow!("Can't get logger"))?;
    mutex.lock().map_err(|e| anyhow!("{}", e))
}

pub struct Log {
    writer: BufWriter<File>,
}

impl Log {
    fn new() -> Result<Log> {
        std::fs::create_dir_all("/tmp/wsm/")?;

        let file_path = "/tmp/wsm/log.txt";

        let file_writer = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        let writer = BufWriter::new(file_writer);
        Ok(Log { writer })
    }
}

macro_rules! info {
      ($($arg:tt)*) => {
          $crate::log::info_impl(format_args!($($arg)*))
      };
  }

pub(crate) fn info_impl(args: std::fmt::Arguments<'_>) {
    if let Ok(mut log) = get_logger() {
        let _ = Write::write_fmt(&mut log.writer, format_args!("[INFO] {args}\n"));
        let _ = log.writer.flush();
    }
}

macro_rules! warn {
      ($($arg:tt)*) => {
          $crate::log::warn_impl(format_args!($($arg)*))
      };
  }

pub(crate) fn warn_impl(args: std::fmt::Arguments<'_>) {
    if let Ok(mut log) = get_logger() {
        let _ = Write::write_fmt(&mut log.writer, format_args!("[WARN] {args}\n"));
        let _ = log.writer.flush();
    }
}

macro_rules! error {
      ($($arg:tt)*) => {
          $crate::log::error_impl(format_args!($($arg)*))
      };
  }

pub(crate) fn error_impl(args: std::fmt::Arguments<'_>) {
    if let Ok(mut log) = get_logger() {
        let _ = Write::write_fmt(&mut log.writer, format_args!("[ERROR] {args}\n"));
        let _ = log.writer.flush();
    }
}

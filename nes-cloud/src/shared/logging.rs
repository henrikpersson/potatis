use std::error::Error;

use flexi_logger::{DeferredNow, Record, style, TS_DASHES_BLANK_COLONS_DOT_BLANK, Logger, Duplicate, FileSpec, WriteMode};

fn log_format(
  w: &mut dyn std::io::Write,
  now: &mut DeferredNow,
  record: &Record,
) -> Result<(), std::io::Error> {
  let level = record.level();
  write!(
      w,
      "[{}] {} [{}:{}] {} {}",
      style(level).paint(now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK).to_string()),
      style(level).paint(level.to_string()),
      record.file().unwrap_or("<unnamed>"),
      record.line().unwrap_or(0),
      style(level).paint(format!("[fd: {}]", std::env::var("FD").unwrap_or_else(|_| "N/A".into()))),
      style(level).paint(&record.args().to_string())
  )
  // Ok(())
}

pub fn init(file: bool) -> Result<(), Box<dyn Error>> {
  let logger = Logger::try_with_str("debug")?
    .format(log_format)
    .append()
    .duplicate_to_stdout(Duplicate::All);

  if file {
    logger.log_to_file(FileSpec::default())
      .write_mode(WriteMode::BufferAndFlush)
      .start()?;
  } else {
    logger.start()?;
  }

  Ok(())
}
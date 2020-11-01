use log::{Record, Level, Metadata, SetLoggerError, LevelFilter};

use crate::println;

static LOGGER: Logger = Logger;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        use vga::colors::Color16;
        // println!("[{}] {}", record.level(), record.args());
        let level_colour = match record.level() {
            Level::Error => Color16::Red,
            Level::Warn => Color16::Yellow,
            Level::Trace => Color16::Cyan,
            _ => Color16::Green,
        };
        crate::vga_buffer::print_colored(&format!("[{}] ", record.level()), level_colour);
        crate::vga_buffer::print_colored(&format!("{}\n", record.args()), Color16::White);
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::max()))
}

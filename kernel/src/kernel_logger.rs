use log::{Record, Level, Metadata, SetLoggerError, LevelFilter};

use crate::println;

static LOGGER: Logger = Logger;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        use crate::vga_buffer::ColourCode;
        use crate::vga_buffer::Colour;
        // println!("[{}] {}", record.level(), record.args());
        let level_colour = match record.level() {
            Level::Error => Colour::Red,
            Level::Warn => Colour::Yellow,
            Level::Trace => Colour::Cyan,
            _ => Colour::Green,
        };
        crate::vga_buffer::print_colored(&format!("[{}] ", record.level()), ColourCode::new(level_colour, Colour::Black));
        crate::vga_buffer::print_colored(&format!("{}\n", record.args()), ColourCode::new(Colour::White, Colour::Black));
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::max()))
}

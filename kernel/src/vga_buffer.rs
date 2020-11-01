use volatile::Volatile;

use core::fmt;

use lazy_static::lazy_static;
use spin::Mutex;

use vga::colors::{Color16, TextModeColor};
use vga::writers::{ScreenCharacter, TextWriter, Text40x25, Text40x50, Text80x25};

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        mode: ModeEnum::NoMode,
        cursor_pos: (0,0),
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

pub fn print_colored(string: &str, colour_code: Color16) {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_string_coloured(string, colour_code);
    });
}

pub fn set_mode(mode: ModeEnum) {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().set_mode(mode);
    });
}

#[derive(Copy, Clone)]
pub enum ModeEnum {
    NoMode,
    Text40x25(Text40x25),
    Text40x50(Text40x50),
    Text80x25(Text80x25),
}

pub struct Writer {
    mode: ModeEnum,
    cursor_pos: (usize, usize),
}

impl Writer {
    fn set_mode(&mut self, mode: ModeEnum) {
        self.mode = mode;
        match mode {
            ModeEnum::Text40x25(m) => m.set_mode(),
            _ => {},
        }
    }

    fn new_line(&mut self) {

    }

    fn clear_row(&mut self, row: usize) {

    }

    pub fn write_string(&mut self, s: &str) {
        match self.mode {
            ModeEnum::Text40x25(m) => {
                let color = TextModeColor::new(Color16::Pink, Color16::Black);
                let width = 40;
                let height = 25;
                for byte in s.bytes() {
                    match byte {
                        0x20..=0x7e => {
                            let char = ScreenCharacter::new(byte, color);
                            m.write_character(self.cursor_pos.0, self.cursor_pos.1, char);
                            self.cursor_pos.0 += 1;
                            if self.cursor_pos.0 >= width {
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 += 1;
                            }
                        },
                        b'\n' => {
                            self.cursor_pos.0 = 0;
                            self.cursor_pos.1 += 1;
                        },
                        _ => {}, //Not writable
                    }
                }
            },
            _ => {}, //Can't do anything without a mode
        }
    }

    pub fn write_string_coloured(&mut self, s: &str, colour_code: Color16) {

    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}

use volatile::Volatile;

use core::fmt;

use lazy_static::lazy_static;
use spin::Mutex;

use vga::colors::{Color16, TextModeColor};
use vga::writers::{
    ScreenCharacter, TextWriter, GraphicsWriter,
    Text40x25, Text40x50, Text80x25,
    Graphics320x200x256, Graphics320x240x256, Graphics640x480x16,
};

use tui::{
    backend::Backend,
    buffer::Cell,
    style::Color,
    layout::Rect,
};

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        mode: ModeEnum::NoMode,
        cursor_pos: (0,0),

        bg_color: Color16::Black,
        fg_color: Color16::Pink,
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

use core::panic::PanicInfo;
pub fn kernel_panic(error: &PanicInfo) {
    set_mode(ModeEnum::Text80x25(
        vga::writers::Text80x25::new()
    ));
    set_text_color(Color16::White, Color16::Black);
    clear_screen();
    // print!("                                        "); //40 spaces
    // print!("                                                                                "); //80 spaces
    println!("");
    print_colored("------/!\\--------------------------------------------------------------/!\\------", Color16::Red);
    println!("Don't worry, your important data was saved! I hope...");
    print_colored("--------------------------------------------------------------------------------", Color16::Red);
    println!("");
    println!("{}", error);
}

pub fn clear_screen() {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().clear_screen();
    });
}

pub fn set_text_color(fg_color: Color16, bg_color: Color16) {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().set_text_color(fg_color, bg_color);
    });
}

pub fn set_cursor_enabled(enabled: bool) {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().set_cursor_enabled(enabled);
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
    //Text modes
    Text40x25(Text40x25),
    Text40x50(Text40x50),
    Text80x25(Text80x25),
    //Graphics modes
    Graphics320x200x256(Graphics320x200x256),
    Graphics320x240x256(Graphics320x240x256),
    Graphics640x480x16(Graphics640x480x16),
}

pub struct Writer {
    pub mode: ModeEnum,
    cursor_pos: (usize, usize),
    bg_color: Color16,
    fg_color: Color16,
}

impl Writer {
    fn set_mode(&mut self, mode: ModeEnum) {
        self.mode = mode;
        match mode {
            ModeEnum::Text40x25(m) => m.set_mode(),
            ModeEnum::Text40x50(m) => m.set_mode(),
            ModeEnum::Text80x25(m) => m.set_mode(),

            ModeEnum::Graphics320x200x256(m) => m.set_mode(),
            ModeEnum::Graphics320x240x256(m) => m.set_mode(),
            ModeEnum::Graphics640x480x16(m) => m.set_mode(),
            _ => {},
        }
    }

    pub fn clear_screen(&mut self) {
        match self.mode {
            ModeEnum::Text40x25(m) => m.clear_screen(),
            ModeEnum::Text40x50(m) => m.clear_screen(),
            ModeEnum::Text80x25(m) => m.clear_screen(),

            ModeEnum::Graphics320x200x256(m) => m.clear_screen(0),
            ModeEnum::Graphics320x240x256(m) => m.clear_screen(0),
            ModeEnum::Graphics640x480x16(m) => m.clear_screen(self.bg_color),
            _ => {},
        }
        self.cursor_pos = (0,0);
    }

    pub fn set_text_color(&mut self, fg_color: Color16, bg_color: Color16) {
        self.fg_color = fg_color;
        self.bg_color = bg_color;
    }

    pub fn set_cursor_enabled(&mut self, enabled: bool) {
        match self.mode {
            ModeEnum::Text40x25(m) => if enabled { m.enable_cursor(); } else { m.disable_cursor(); },
            ModeEnum::Text40x50(m) => if enabled { m.enable_cursor(); } else { m.disable_cursor(); },
            ModeEnum::Text80x25(m) => if enabled { m.enable_cursor(); } else { m.disable_cursor(); },

            _ => {},
        }
    }

    pub fn get_size(&self) -> (usize, usize) {
        match self.mode {
            ModeEnum::Text40x25(_) => (40, 25),
            ModeEnum::Text40x50(_) => (40, 50),
            ModeEnum::Text80x25(_) => (80, 25),

            //Probably should check how big characters are (default: 8x8 font)
            ModeEnum::Graphics320x200x256(_) => (320, 200),
            ModeEnum::Graphics320x240x256(_) => (320, 240),
            ModeEnum::Graphics640x480x16(_) => (640, 480),

            _ => (0, 0), //No mode set or mode not supported yet
        }
    }

    pub fn write_string(&mut self, s: &str) {
        self.write_string_coloured(s, self.fg_color);
    }

    pub fn write_string_coloured(&mut self, s: &str, fg_color: Color16) {
        match self.mode {
            //Text modes
            ModeEnum::Text40x25(m) => {
                let color = TextModeColor::new(fg_color, self.bg_color);
                let width = 40;
                let height = 25;
                for byte in s.bytes() {
                    match byte {
                        0x20..=0x7e => {
                            if self.cursor_pos.1 >= height {
                                m.clear_screen();
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 = 0;
                            }
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
            ModeEnum::Text40x50(m) => {
                let color = TextModeColor::new(fg_color, self.bg_color);
                let width = 40;
                let height = 50;
                for byte in s.bytes() {
                    match byte {
                        0x20..=0x7e => {
                            if self.cursor_pos.1 >= height {
                                m.clear_screen();
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 = 0;
                            }
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
            ModeEnum::Text80x25(m) => {
                let color = TextModeColor::new(fg_color, self.bg_color);
                let width = 80;
                let height = 25;
                for byte in s.bytes() {
                    match byte {
                        0x20..=0x7e => {
                            if self.cursor_pos.1 >= height {
                                m.clear_screen();
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 = 0;
                            }
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

            //Graphics modes
            ModeEnum::Graphics320x200x256(m) => {
                // let color = TextModeColor::new(fg_color, Color16::Black);
                let char_width = 8;
                let char_height = 12; //???
                let width = 320 / char_width;
                let height = 200 / char_height;
                for (offset, char) in s.chars().enumerate() {
                    match char {
                        '\n' => {
                            self.cursor_pos.0 = 0;
                            self.cursor_pos.1 += 1;
                        },
                        _ => {
                            if self.cursor_pos.1 >= height {
                                m.clear_screen(0);
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 = 0;
                            }
                            m.draw_character(self.cursor_pos.0 * char_width, self.cursor_pos.1 * char_height, char, 255);
                            self.cursor_pos.0 += 1;
                            if self.cursor_pos.0 >= width {
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 += 1;
                            }
                        },
                    }
                }
            },
            ModeEnum::Graphics640x480x16(m) => {
                let char_width = 8;
                let char_height = 12; //???
                let width = 640 / char_width;
                let height = 480 / char_height;
                for (offset, char) in s.chars().enumerate() {
                    match char {
                        '\n' => {
                            self.cursor_pos.0 = 0;
                            self.cursor_pos.1 += 1;
                        },
                        _ => {
                            if self.cursor_pos.1 >= height {
                                m.clear_screen(self.bg_color);
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 = 0;
                            }
                            m.draw_character(self.cursor_pos.0 * char_width, self.cursor_pos.1 * char_height, char, fg_color);
                            self.cursor_pos.0 += 1;
                            if self.cursor_pos.0 >= width {
                                self.cursor_pos.0 = 0;
                                self.cursor_pos.1 += 1;
                            }
                        },
                    }
                }
            },

            _ => {}, //Can't do anything without a mode
        }
    }
}

impl Backend for Writer {
    fn draw<'a, I>(&mut self, content: I) -> Result<(), tui::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>
    {
        unimplemented!();
    }

    fn hide_cursor(&mut self) -> Result<(), tui::io::Error> {
        self.set_cursor_enabled(false);
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), tui::io::Error> {
        self.set_cursor_enabled(true);
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), tui::io::Error> {
        Ok((self.cursor_pos.0 as u16, self.cursor_pos.1 as u16)) // Shit code but it works lol
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), tui::io::Error> {
        self.cursor_pos = (x as usize, y as usize);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), tui::io::Error> {
        self.clear_screen();
        Ok(())
    }

    fn size(&self) -> Result<Rect, tui::io::Error> {
        let size = self.get_size();
        Ok(Rect {
            x: 0,
            y: 0,
            width: size.0 as u16, //These 2 are originally usize, but for now, everything fits in a u16 so it's fine
            height: size.1 as u16,
        })
    }

    fn flush(&mut self) -> Result<(), tui::io::Error> {
        unimplemented!();
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

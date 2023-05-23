// uses libc for termios, Not the full interface, because it's confusing.
//
// Should be usable

use std::io::{self, Read, Write, BufRead};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, RawFd};

const NCCS: usize = 32;
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
struct termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; NCCS],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[link(name = "c")]
extern "C" {
    fn tcsetattr(fd: RawFd, optional_actions: i32, termios_p: *const termios) -> i32;
    fn tcgetattr(fd: RawFd, termios_p: *mut termios) -> i32;
}

pub struct TerfLleol<O: Write + AsRawFd, I: Read + AsRawFd> {
    allbwn: O,
    mewnbwn: I,
    llawnsgrin: bool,
    blaen: termios,
    cyfred: termios,
}

impl<O: Write + AsRawFd, I: Read + AsRawFd> TerfLleol<O, I> {
    pub fn newidd(allan: O, mewn: I) -> io::Result<Self> {
        let mut temp = MaybeUninit::<termios>::uninit();
        match io_result(unsafe { tcgetattr(allan.as_raw_fd(), temp.as_mut_ptr()) }) {
            Ok(_) => {
                let temp = unsafe { temp.assume_init() };
                Ok(TerfLleol {
                    allbwn: allan,
                    mewnbwn: mewn,
                    llawnsgrin: false,
                    blaen: temp,
                    cyfred: temp,
                })
            }
            Err(err) => Err(err),
        }
    }
    pub fn newid<'a>(&'a mut self) -> Newidiad<'a, O, I> {
        let blaen = self.cyfred.clone();
        Newidiad{
            targed: self,
            blaen: blaen,
            llawnsgrin: None,
        }
    }
    pub fn ungell(&mut self) -> io::Result<Option<char>> {
        // currently only supports up to 4 byte utf8 strings
        let mut init_buff: [u8; 1] = [0; 1];
        if self.mewnbwn.read(&mut init_buff)? == 0 {
            return Ok(None);
        }
        if !utf8_start(init_buff[0]) {
            return Ok(None);
        }
        if utf8_len(init_buff[0]) == 1 {
            return Ok(Some(init_buff[0] as char));
        }

        let mut rest_buff = vec![0; utf8_len(init_buff[0])];
        let _ = self.mewnbwn.read(&mut rest_buff)?;
        rest_buff.insert(0, init_buff[0]);
        return Ok(String::from_utf8(rest_buff)
            .expect("fatal error")
            .chars()
            .next());
    }
}
impl<O: Write + AsRawFd, I: Read + AsRawFd> Drop for TerfLleol<O, I> {
    fn drop(&mut self) {
        if self.llawnsgrin {
            let _ = self.allbwn.write(b"\x1b[1049l");
        }
        let _ = unsafe { tcsetattr(self.allbwn.as_raw_fd(), TCSANOW, &self.blaen) };
    }
}
pub struct Newidiad<'a, O: Write + AsRawFd, I: Read + AsRawFd> {
    targed: &'a mut TerfLleol<O, I>,
    blaen: termios,
    llawnsgrin: Option<bool>,
}

impl<'a, O: Write + AsRawFd, I: Read + AsRawFd> Newidiad<'a, O, I> {
    pub fn atod(&mut self) -> io::Result<()> {
        self.targed.cyfred = self.blaen;
        unsafe {
            if self.llawnsgrin.is_some() {
            self.targed.llawnsgrin = self.llawnsgrin.unwrap();
            if self.llawnsgrin.unwrap() {
                let _ = self.targed.allbwn.write(b"\x1b[1049h")?;
            } else {
                let _ = self.targed.allbwn.write(b"\x1b[1049l")?;
            }
            }
            io_result(tcsetattr(self.targed.allbwn.as_raw_fd(), TCSANOW, &self.blaen))
        }
    }
    pub fn canon(&mut self, value: bool) -> &mut Self {
        self.blaen.c_lflag &= !(ICANON);
        if value {
            self.blaen.c_lflag |= ICANON;
        }
        self
    }
    pub fn echo(&mut self, value: bool) -> &mut Self {
        self.blaen.c_lflag &= !(ECHO);
        if value {
            self.blaen.c_lflag |= ECHO;
        }
        self
    }
    pub fn stopi(&mut self, value: bool) -> &mut Self {
        self.blaen.c_cc[VMIN] = if value { 1 } else { 0 };
        self
    }
    pub fn llawnsgrin(&mut self, value: bool) -> &mut Self {
        self.llawnsgrin = Some(value);
        self
    }
}

impl<O: Write + AsRawFd, I: Read + AsRawFd> Read for TerfLleol<O, I> {
    fn read(&mut self, buff: &mut [u8]) -> io::Result<usize> {
        self.mewnbwn.read(buff)
    }
}
impl<O: Write + AsRawFd, I: Read + AsRawFd> Write for TerfLleol<O, I> {
    fn write(&mut self, buff: & [u8]) -> io::Result<usize> {
        self.allbwn.write(buff)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.allbwn.flush()
    }
}
impl<O: Write + AsRawFd, I: BufRead + AsRawFd> BufRead for TerfLleol<O, I> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.mewnbwn.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.mewnbwn.consume(amt)
    }
}

#[inline(always)]
fn io_result(result: i32) -> io::Result<()> {
    match result {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}
#[inline(always)]
pub fn utf8_start(val: u8) -> bool {
    (val & 0x80 == 0) | (val & 0xc0 != 0x80)
}
pub fn utf8_len(val: u8) -> usize {
    // returns the length of the utf8 encoded codepoint
    // returns 0 if the character is a continuation
    if val & 0x80 == 0x00 {
        return 1;
    }
    if val & 0xc0 == 0x80 {
        return 0;
    }
    assert!(val & 0xc0 == 0xc0);
    // here's the annoying
    let mut ones = 0;
    for i in (3..=7).rev() {
        if (val & (1 << i)) == 0 {
            break;
        }
        ones += 1;
    }
    ones
}
pub const VMIN: usize = 6;

const ICANON: u32 = 0o000002;
const ECHO: u32 = 0o000010;
const TCSANOW: i32 = 0;

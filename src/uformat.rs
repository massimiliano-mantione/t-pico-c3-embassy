use core::convert::Infallible;

use arrayvec::ArrayString;
use ufmt::{uDisplay, uWrite};

const TEXT_SIZE: usize = 256;

pub struct FormattedText {
    s: ArrayString<TEXT_SIZE>,
}

#[allow(unused)]
impl FormattedText {
    pub fn new() -> Self {
        Self {
            s: ArrayString::new(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.s.as_str()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Into<ArrayString<256>> for FormattedText {
    fn into(self) -> ArrayString<256> {
        self.s
    }
}

impl uWrite for FormattedText {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.s.push_str(s);
        Ok(())
    }
}

fn examine_f32(val: f32, threshold: f32) -> Option<(char, f32)> {
    if val > threshold {
        Some(('>', threshold))
    } else if val > 0.0 {
        Some(('+', val))
    } else if val == 0.0 {
        Some((' ', 0.0))
    } else if val >= -threshold {
        Some(('-', -val))
    } else if val < -threshold {
        Some(('<', -threshold))
    } else {
        None
    }
}

pub struct FormattedF32_5_2(f32);
#[allow(unused)]
pub fn f5_2(val: f32) -> FormattedF32_5_2 {
    FormattedF32_5_2(val)
}
impl uDisplay for FormattedF32_5_2 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        match examine_f32(self.0, 99999.99) {
            Some((s, v)) => {
                let mut i = (v * 100.0) as u32;
                let mut m = 1000000u32;
                f.write_char(s)?;
                for _ in 0..5 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                f.write_char('.')?;
                for _ in 0..2 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                Ok(())
            }
            None => f.write_str("   NaN   "),
        }
    }
}

pub struct FormattedF32_1_2(f32);
#[allow(unused)]
pub fn f1_2(val: f32) -> FormattedF32_1_2 {
    FormattedF32_1_2(val)
}
impl uDisplay for FormattedF32_1_2 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        match examine_f32(self.0, 9.99) {
            Some((s, v)) => {
                let mut i = (v * 100.0) as u32;
                let mut m = 100u32;
                f.write_char(s)?;
                for _ in 0..1 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                f.write_char('.')?;
                for _ in 0..2 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                Ok(())
            }
            None => f.write_str("NaN "),
        }
    }
}

pub struct FormattedF32_4_0(f32);
#[allow(unused)]
pub fn f4(val: f32) -> FormattedF32_4_0 {
    FormattedF32_4_0(val)
}
impl uDisplay for FormattedF32_4_0 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        match examine_f32(self.0, 9999.0) {
            Some((s, v)) => {
                let mut i = v as u32;
                let mut m = 1000u32;
                f.write_char(s)?;
                for _ in 0..4 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                Ok(())
            }
            None => f.write_str("   NaN   "),
        }
    }
}

pub struct FormattedF32_3_0(f32);
#[allow(unused)]
pub fn f3(val: f32) -> FormattedF32_3_0 {
    FormattedF32_3_0(val)
}
impl uDisplay for FormattedF32_3_0 {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        match examine_f32(self.0, 999.0) {
            Some((s, v)) => {
                let mut i = v as u32;
                let mut m = 100u32;
                f.write_char(s)?;
                for _ in 0..3 {
                    f.write_char(('0' as u8 + ((i / m) as u8)).into())?;
                    i %= m;
                    m /= 10;
                }
                Ok(())
            }
            None => f.write_str("   NaN   "),
        }
    }
}

#[macro_export]
macro_rules! uformat {
    // IMPORTANT use `tt` fragments instead of `expr` fragments (i.e. `$($exprs:expr),*`)
    ($($tt:tt)*) => {{
        let mut line = FormattedText::new();
        ufmt::uwrite!(&mut line, $($tt)*).ok();
        line
    }}
}

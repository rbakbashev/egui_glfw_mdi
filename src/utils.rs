use std::ffi::CString;
use std::fmt::Display;

pub trait CheckError<T>: Sized {
    #[track_caller]
    fn or_err(self, msg: impl Display) -> T;

    #[track_caller]
    fn try_to(self, action: impl Display) -> T {
        self.or_err(format!("failed to {action}"))
    }
}

impl<T> CheckError<T> for Option<T> {
    fn or_err(self, msg: impl Display) -> T {
        match self {
            Some(t) => t,
            None => panic!("{msg}"),
        }
    }
}

impl<T, E: Display> CheckError<T> for Result<T, E> {
    fn or_err(self, msg: impl Display) -> T {
        match self {
            Ok(t) => t,
            Err(err) => panic!("{msg}: {err}"),
        }
    }
}

impl<T> CheckError<*mut T> for *mut T {
    fn or_err(self, msg: impl Display) -> *mut T {
        if self.is_null() {
            panic!("{msg}");
        }

        self
    }
}

#[track_caller]
pub fn to_u32<T: TryInto<u32> + Display + Copy>(x: T) -> u32 {
    x.try_into().ok().try_to(format!("cast {x} to u32"))
}

#[track_caller]
pub fn to_i32<T: TryInto<i32> + Display + Copy>(x: T) -> i32 {
    x.try_into().ok().try_to(format!("cast {x} to i32"))
}

#[track_caller]
pub fn to_cstring<T: Into<Vec<u8>> + Display + Copy>(x: T) -> CString {
    CString::new(x).try_to(format!("convert \"{x}\" to CString"))
}

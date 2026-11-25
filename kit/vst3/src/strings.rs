//! Fixed-size string buffers the VST3 API hands over: C strings for
//! factory info, UTF-16 for everything a user reads.

use std::ffi::c_char;
use vst3::Steinberg::Vst::TChar;

/// Copies as much as fits, always leaving a terminating zero.
pub fn copy_cstring(src: &str, dst: &mut [c_char]) {
    let Some(room) = dst.len().checked_sub(1) else {
        return;
    };

    let n = src.len().min(room);
    src.bytes()
        .take(n)
        .zip(dst.iter_mut())
        .for_each(|(b, d)| *d = b as c_char);
    dst[n] = 0;
}

/// Copies as much as fits, always leaving a terminating zero.
pub fn copy_wstring(src: &str, dst: &mut [TChar]) {
    let Some(room) = dst.len().checked_sub(1) else {
        return;
    };

    let mut n = 0;
    src.encode_utf16()
        .take(room)
        .zip(dst.iter_mut())
        .for_each(|(u, d)| {
            *d = u as TChar;
            n += 1;
        });
    dst[n] = 0;
}

/// Reads a zero-terminated UTF-16 string, or `None` if it is not valid.
///
/// # Safety
/// `ptr` must point to a zero-terminated UTF-16 string.
pub unsafe fn read_wstring(ptr: *const TChar) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let len = (0..).take_while(|&i| unsafe { *ptr.add(i) } != 0).count();
    let units = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16(units).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(buf: &[c_char]) -> Vec<u8> {
        buf.iter().map(|&b| b as u8).collect()
    }

    #[test]
    fn copies_c_strings_with_room_for_the_zero() {
        // Expected bytes up to and including the terminating zero.
        let cases: [(&str, usize, &[u8]); 4] = [
            ("abc", 5, b"abc\0"),
            ("abcd", 4, b"abc\0"),
            ("", 2, b"\0"),
            ("abc", 1, b"\0"),
        ];

        for (src, size, want) in cases {
            let mut buf = vec![0x7F as c_char; size];
            copy_cstring(src, &mut buf);
            assert_eq!(&c(&buf)[..want.len()], want, "{src} into {size}");
        }
    }

    #[test]
    fn round_trips_wide_strings() {
        let cases = [
            ("Major", 16, "Major"),
            ("C#", 16, "C#"),
            ("Truncated", 6, "Trunc"),
            ("Ünï", 8, "Ünï"),
        ];

        for (src, size, want) in cases {
            let mut buf = vec![0 as TChar; size];
            copy_wstring(src, &mut buf);
            assert_eq!(
                unsafe { read_wstring(buf.as_ptr()) }.as_deref(),
                Some(want),
                "{src}"
            );
        }
    }

    #[test]
    fn reads_null_as_none() {
        assert_eq!(unsafe { read_wstring(std::ptr::null()) }, None);
    }
}

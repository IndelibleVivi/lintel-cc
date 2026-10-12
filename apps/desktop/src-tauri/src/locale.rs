//! Read only the App's language preference order. No shell, files or settings writes.

pub fn preferred_languages() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        macos::preferred_languages()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{c_char, c_void, CStr};

    const MAX_LANGUAGES: isize = 8;
    const LANGUAGE_BYTES: usize = 128;
    const UTF8: u32 = 0x0800_0100;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(array: *const c_void) -> isize;
        fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut c_char,
            capacity: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }

    struct OwnedArray(*const c_void);
    impl Drop for OwnedArray {
        fn drop(&mut self) {
            // SAFETY: only a non-null +1 array returned by the Copy API is owned.
            unsafe { CFRelease(self.0) }
        }
    }

    pub(super) fn preferred_languages() -> Vec<String> {
        // SAFETY: this system API takes no input and returns an owned CFArray
        // of CFStrings. Borrowed members remain alive until the array is released.
        let array = unsafe { CFLocaleCopyPreferredLanguages() };
        if array.is_null() {
            return Vec::new();
        }
        let array = OwnedArray(array);
        let count = unsafe { CFArrayGetCount(array.0) }.clamp(0, MAX_LANGUAGES);
        let mut languages = Vec::with_capacity(count as usize);
        for index in 0..count {
            let string = unsafe { CFArrayGetValueAtIndex(array.0, index) };
            if string.is_null() {
                continue;
            }
            let mut buffer = [0 as c_char; LANGUAGE_BYTES];
            let copied = unsafe {
                CFStringGetCString(string, buffer.as_mut_ptr(), buffer.len() as isize, UTF8)
            };
            if copied == 0 {
                continue;
            }
            // A successful CFStringGetCString includes the terminating NUL.
            let language = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_bytes();
            if !language.is_empty()
                && language
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                // The finite language identifier alphabet above is valid UTF-8.
                languages.push(String::from_utf8(language.to_vec()).unwrap());
            }
        }
        languages
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn language_projection_is_finite_identifiers_only() {
        let languages = super::preferred_languages();
        assert!(languages.len() <= 8);
        for language in languages {
            assert!(!language.is_empty() && language.len() < 128);
            assert!(language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_preferences_are_readable_and_repeatable() {
        let first = super::preferred_languages();
        assert!(
            !first.is_empty(),
            "macOS should provide a preferred language"
        );
        assert_eq!(first, super::preferred_languages());
    }
}

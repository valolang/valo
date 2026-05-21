//! Platform-specific FFI helpers and ABI definitions.

pub fn platform_libc() -> &'static str {
    #[cfg(windows)]
    {
        "msvcrt.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libSystem.B.dylib"
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    {
        "libc.so.6"
    }
    #[cfg(target_os = "android")]
    {
        "libc.so"
    }
}

pub fn platform_libm() -> &'static str {
    #[cfg(windows)]
    {
        "msvcrt.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libSystem.B.dylib"
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    {
        "libm.so.6"
    }
    #[cfg(target_os = "android")]
    {
        "libm.so"
    }
}

pub fn platform_test_math_symbol() -> &'static str {
    "cos"
}

pub fn platform_test_process_symbol() -> &'static str {
    #[cfg(windows)]
    {
        "GetCurrentProcessId" // Windows kernel32
    }
    #[cfg(unix)]
    {
        "getpid"
    }
}

pub fn platform_test_process_library() -> &'static str {
    #[cfg(windows)]
    {
        "kernel32.dll"
    }
    #[cfg(unix)]
    {
        platform_libc()
    }
}

pub fn platform_test_byref_library() -> &'static str {
    #[cfg(windows)]
    {
        "kernel32.dll"
    }
    #[cfg(unix)]
    {
        platform_libc()
    }
}

pub fn platform_test_byref_symbol() -> &'static str {
    #[cfg(windows)]
    {
        "GetSystemTimeAsFileTime"
    }
    #[cfg(unix)]
    {
        "time"
    }
}

pub fn pointer_width() -> usize {
    std::mem::size_of::<usize>()
}

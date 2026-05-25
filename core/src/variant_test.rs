#[cfg(windows)]
pub fn test_variant() {
    use windows::Win32::System::Com::IDispatch;
    use windows::Win32::System::Variant::VARIANT;
    use windows::core::{BSTR, HRESULT, PCWSTR};
    let mut v = VARIANT::default();
    let b = BSTR::from("test");
    v = b.into();

    // How to read it?
    let x: BSTR = (&v).try_into().unwrap();
}

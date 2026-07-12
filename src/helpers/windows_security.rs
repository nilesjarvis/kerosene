use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SetFileSecurityW,
};

/// Replace inherited permissions with an owner-only ACL.
///
/// `OW` is the Windows Owner Rights SID. `OICI` makes the ACE inheritable when
/// `path` is a directory and is harmless for files. SYSTEM and administrators
/// retain the normal ownership/recovery mechanisms without being granted
/// application-level access here.
pub(crate) fn restrict_path_to_owner(path: &Path) -> Result<(), String> {
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let sddl = "D:P(A;OICI;FA;;;OW)\0".encode_utf16().collect::<Vec<_>>();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if converted == 0 {
        return Err(format!(
            "create owner-only Windows security descriptor failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let applied = unsafe {
        SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let apply_error = (applied == 0).then(std::io::Error::last_os_error);
    unsafe {
        LocalFree(descriptor);
    }

    match apply_error {
        Some(error) => Err(format!("apply owner-only Windows ACL failed: {error}")),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn owner_only_acl_preserves_owner_file_access() {
        let counter = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kerosene-windows-acl-{}-{counter}.tmp",
            std::process::id()
        ));
        let mut file = std::fs::File::create(&path).expect("create ACL test file");

        restrict_path_to_owner(&path).expect("apply owner-only ACL");
        file.write_all(b"private").expect("write protected file");
        drop(file);

        let mut contents = String::new();
        std::fs::File::open(&path)
            .expect("open protected file")
            .read_to_string(&mut contents)
            .expect("read protected file");
        assert_eq!(contents, "private");
        std::fs::remove_file(path).expect("remove ACL test file");
    }
}

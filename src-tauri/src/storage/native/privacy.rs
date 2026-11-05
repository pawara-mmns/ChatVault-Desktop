use super::models::fail;
use crate::storage::error::Result;
use std::path::Path;
pub fn restrict(path: &Path) -> Result<()> {
    crate::storage::paths::verify_ancestors(path, false)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| fail("native_io"))?;
    }
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::{CloseHandle, LocalFree},
            Security::{Authorization::*, *},
            System::Threading::{GetCurrentProcess, OpenProcessToken},
        };
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(fail("native_io"));
        }
        let mut length = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut length);
        let mut info = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            info.as_mut_ptr().cast(),
            length,
            &mut length,
        );
        CloseHandle(token);
        if ok == 0 {
            return Err(fail("native_io"));
        }
        let user = &*info.as_ptr().cast::<TOKEN_USER>();
        let mut sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(fail("native_io"));
        }
        let mut n = 0;
        while n < 256 && *sid.add(n) != 0 {
            n += 1;
        }
        let sid_text = String::from_utf16_lossy(std::slice::from_raw_parts(sid, n));
        LocalFree(sid.cast());
        if n == 256 {
            return Err(fail("native_io"));
        }
        let sddl: Vec<u16> = format!("D:P(A;OICI;FA;;;{sid_text})(A;OICI;FA;;;SY)")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(fail("native_io"));
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = SetFileSecurityW(
            wide.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        );
        LocalFree(descriptor.cast());
        if ok == 0 {
            return Err(fail("native_io"));
        }
    }
    Ok(())
}

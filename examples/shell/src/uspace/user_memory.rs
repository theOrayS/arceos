use core::ffi::{CStr, c_char};
use core::mem::size_of;
use core::ptr;

use axerrno::LinuxError;
use axhal::paging::MappingFlags;
use memory_addr::VirtAddr;
use std::string::{String, ToString};

use super::{UserProcess, neg_errno};

pub(super) fn validate_user_read(
    process: &UserProcess,
    ptr: usize,
    len: usize,
) -> Result<(), LinuxError> {
    validate_user_access(process, ptr, len, false)
}

pub(super) fn validate_user_write(
    process: &UserProcess,
    ptr: usize,
    len: usize,
) -> Result<(), LinuxError> {
    validate_user_access(process, ptr, len, true)
}

fn validate_user_access(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    write: bool,
) -> Result<(), LinuxError> {
    if len == 0 {
        return Ok(());
    }
    let valid = if write {
        user_bytes_mut(process, ptr, len, true).is_some()
    } else {
        user_bytes(process, ptr, len, false).is_some()
    };
    if ptr == 0 || !valid {
        return Err(LinuxError::EFAULT);
    }
    Ok(())
}

fn user_range_fits(ptr: usize, len: usize) -> bool {
    len == 0 || ptr.checked_add(len).is_some()
}

fn user_range_accessible(process: &UserProcess, ptr: usize, len: usize, write: bool) -> bool {
    if !user_range_fits(ptr, len) {
        return false;
    }
    let flags = if write {
        MappingFlags::READ | MappingFlags::WRITE
    } else {
        MappingFlags::READ
    };
    let aspace = process.aspace.lock();
    aspace.can_access_range(VirtAddr::from(ptr), len, flags)
}

pub(super) fn user_bytes<'a>(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    write: bool,
) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    if !user_range_accessible(process, ptr, len, write) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(ptr as *const u8, len) })
}

pub(super) fn user_bytes_mut<'a>(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    write: bool,
) -> Option<&'a mut [u8]> {
    if len == 0 {
        return Some(&mut []);
    }
    if !user_range_accessible(process, ptr, len, write) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) })
}

pub(super) fn write_user_value<T: Copy>(process: &UserProcess, ptr: usize, value: &T) -> isize {
    let Some(dst) = user_bytes_mut(process, ptr, size_of::<T>(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    unsafe {
        ptr::copy_nonoverlapping(
            value as *const T as *const u8,
            dst.as_mut_ptr(),
            size_of::<T>(),
        );
    }
    0
}

pub(super) fn read_user_value<T: Copy>(process: &UserProcess, ptr: usize) -> Result<T, LinuxError> {
    let Some(src) = user_bytes(process, ptr, size_of::<T>(), false) else {
        return Err(LinuxError::EFAULT);
    };
    Ok(unsafe { ptr::read_unaligned(src.as_ptr() as *const T) })
}

pub(super) fn read_cstr(process: &UserProcess, ptr: usize) -> Result<String, LinuxError> {
    if ptr == 0 {
        return Err(LinuxError::EFAULT);
    }
    if !user_range_fits(ptr, 1) {
        return Err(LinuxError::EFAULT);
    }
    if !process
        .aspace
        .lock()
        .can_access_range(VirtAddr::from(ptr), 1, MappingFlags::READ)
    {
        return Err(LinuxError::EFAULT);
    }
    unsafe { CStr::from_ptr(ptr as *const c_char) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| LinuxError::EINVAL)
}

use core::mem::{MaybeUninit, size_of};

use axerrno::LinuxError;
use axhal::paging::MappingFlags;
use memory_addr::VirtAddr;
use std::string::{String, ToString};
use std::vec::Vec;

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
    if ptr == 0 || !user_range_accessible(process, ptr, size_of::<T>(), true) {
        return neg_errno(LinuxError::EFAULT);
    }

    let src =
        unsafe { core::slice::from_raw_parts(value as *const T as *const u8, size_of::<T>()) };
    process
        .aspace
        .lock()
        .write(VirtAddr::from(ptr), src)
        .map_or_else(|_| neg_errno(LinuxError::EFAULT), |_| 0)
}

pub(super) fn read_user_value<T: Copy>(process: &UserProcess, ptr: usize) -> Result<T, LinuxError> {
    if ptr == 0 || !user_range_accessible(process, ptr, size_of::<T>(), false) {
        return Err(LinuxError::EFAULT);
    }

    let mut value = MaybeUninit::<T>::uninit();
    let dst =
        unsafe { core::slice::from_raw_parts_mut(value.as_mut_ptr() as *mut u8, size_of::<T>()) };
    process
        .aspace
        .lock()
        .read(VirtAddr::from(ptr), dst)
        .map_err(|_| LinuxError::EFAULT)?;
    Ok(unsafe { value.assume_init() })
}

pub(super) fn read_cstr(process: &UserProcess, ptr: usize) -> Result<String, LinuxError> {
    const MAX_USER_CSTR_LEN: usize = 128 * 1024;

    if ptr == 0 {
        return Err(LinuxError::EFAULT);
    }
    if !user_range_fits(ptr, 1) {
        return Err(LinuxError::EFAULT);
    }

    let aspace = process.aspace.lock();
    let mut bytes = Vec::new();
    for offset in 0..MAX_USER_CSTR_LEN {
        let addr = ptr.checked_add(offset).ok_or(LinuxError::EFAULT)?;
        if !aspace.can_access_range(VirtAddr::from(addr), 1, MappingFlags::READ) {
            return Err(LinuxError::EFAULT);
        }
        let mut byte = [0u8; 1];
        aspace
            .read(VirtAddr::from(addr), &mut byte)
            .map_err(|_| LinuxError::EFAULT)?;
        if byte[0] == 0 {
            return String::from_utf8(bytes).map_err(|_| LinuxError::EINVAL);
        }
        bytes.push(byte[0]);
    }

    Err(LinuxError::EINVAL)
}

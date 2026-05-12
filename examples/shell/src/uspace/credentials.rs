use core::mem::size_of;

use axerrno::LinuxError;
use linux_raw_sys::general;
use std::string::String;
use std::vec::Vec;

use super::linux_abi::{ACCESS_R_OK, ACCESS_W_OK, ACCESS_X_OK, CHOWN_ID_UNCHANGED};
use super::user_memory::{read_user_value, write_user_value};
use super::{UserProcess, neg_errno};

pub(super) fn set_single_id<F>(id: usize, apply: F) -> isize
where
    F: FnOnce(u32),
{
    let Ok(id) = u32::try_from(id) else {
        return neg_errno(LinuxError::EINVAL);
    };
    apply(id);
    0
}

pub(super) fn id_arg_optional(id: usize) -> Result<Option<u32>, LinuxError> {
    if id == usize::MAX || id == CHOWN_ID_UNCHANGED as usize {
        return Ok(None);
    }
    u32::try_from(id).map(Some).map_err(|_| LinuxError::EINVAL)
}

fn parse_id_args<const N: usize>(ids: [usize; N]) -> Result<[Option<u32>; N], LinuxError> {
    let mut parsed = [None; N];
    for (dst, id) in parsed.iter_mut().zip(ids) {
        *dst = id_arg_optional(id)?;
    }
    Ok(parsed)
}

pub(super) fn set_re_ids<F>(real: usize, effective: usize, apply: F) -> isize
where
    F: FnOnce(Option<u32>, Option<u32>, Option<u32>),
{
    let [real, effective] = match parse_id_args([real, effective]) {
        Ok(ids) => ids,
        Err(err) => return neg_errno(err),
    };
    apply(real, effective, effective.or(real));
    0
}

pub(super) fn set_res_ids<F>(real: usize, effective: usize, saved: usize, apply: F) -> isize
where
    F: FnOnce(Option<u32>, Option<u32>, Option<u32>),
{
    let [real, effective, saved] = match parse_id_args([real, effective, saved]) {
        Ok(ids) => ids,
        Err(err) => return neg_errno(err),
    };
    apply(real, effective, saved);
    0
}

pub(super) fn set_fs_id<F>(old: u32, id: usize, apply: F) -> isize
where
    F: FnOnce(u32),
{
    if let Ok(Some(id)) = id_arg_optional(id) {
        apply(id);
    }
    old as isize
}

pub(super) fn write_id_triplet(process: &UserProcess, ptrs: [usize; 3], values: [u32; 3]) -> isize {
    for (ptr, value) in ptrs.into_iter().zip(values.into_iter()) {
        let ret = write_user_value(process, ptr, &value);
        if ret != 0 {
            return ret;
        }
    }
    0
}

pub(super) fn write_group_list(process: &UserProcess, list: usize, groups: &[u32]) -> isize {
    for (idx, gid) in groups.iter().enumerate() {
        let ret = write_user_value(process, list + idx * size_of::<u32>(), gid);
        if ret != 0 {
            return ret;
        }
    }
    groups.len() as isize
}

pub(super) fn read_group_list(
    process: &UserProcess,
    size: usize,
    list: usize,
) -> Result<Vec<u32>, LinuxError> {
    let mut groups = Vec::new();
    for idx in 0..size {
        groups.push(read_user_value::<u32>(
            process,
            list + idx * size_of::<u32>(),
        )?);
    }
    Ok(groups)
}

pub(super) fn access_allowed(st: &general::stat, mode: usize, uid: u32, gid: u32) -> bool {
    if mode == 0 {
        return true;
    }

    let permissions = (st.st_mode & 0o777) as u32;
    if uid == 0 {
        return (mode & ACCESS_X_OK == 0) || (permissions & 0o111 != 0);
    }

    let bits = if uid == st.st_uid as u32 {
        (permissions >> 6) & 0o7
    } else if gid == st.st_gid as u32 {
        (permissions >> 3) & 0o7
    } else {
        permissions & 0o7
    };

    if mode & ACCESS_R_OK != 0 && bits & 0o4 == 0 {
        return false;
    }
    if mode & ACCESS_W_OK != 0 && bits & 0o2 == 0 {
        return false;
    }
    if mode & ACCESS_X_OK != 0 && bits & 0o1 == 0 {
        return false;
    }
    true
}

pub(super) fn chown_ids(
    owner: usize,
    group: usize,
) -> Result<(Option<u32>, Option<u32>), LinuxError> {
    parse_id_args([owner, group]).map(|[owner, group]| (owner, group))
}

pub(super) fn apply_chown_metadata(
    process: &UserProcess,
    path: Option<String>,
    st: &general::stat,
    owner: Option<u32>,
    group: Option<u32>,
) -> isize {
    if !chown_allowed(process, st, owner, group) {
        return neg_errno(LinuxError::EPERM);
    }
    if let Some(path) = path {
        process.set_path_owner(path.clone(), owner, group);
        if owner.is_some() || group.is_some() {
            process.clear_path_chown_special_bits(path.as_str(), st.st_mode);
        }
    }
    0
}

fn chown_allowed(
    process: &UserProcess,
    st: &general::stat,
    owner: Option<u32>,
    group: Option<u32>,
) -> bool {
    if process.uid() == 0 {
        return true;
    }
    if let Some(owner) = owner {
        if owner != st.st_uid || owner != process.uid() {
            return false;
        }
    }
    if let Some(group) = group {
        if group != st.st_gid && !process.has_group(group) {
            return false;
        }
    }
    true
}

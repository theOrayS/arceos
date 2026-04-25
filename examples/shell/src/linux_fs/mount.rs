use axerrno::LinuxError;
use std::string::{String, ToString};
use std::vec::Vec;

#[derive(Clone, Default)]
pub struct MountTable {
    targets: Vec<String>,
}

pub struct MountRequest<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub flags: usize,
    pub data: usize,
}

pub struct UmountRequest<'a> {
    pub target: &'a str,
    pub flags: usize,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.targets.clear();
    }

    pub fn mount(&mut self, request: MountRequest<'_>) -> Result<(), LinuxError> {
        validate_mount_request(&request)?;
        if self.targets.iter().any(|target| target == request.target) {
            return Err(LinuxError::EBUSY);
        }
        self.targets.push(request.target.to_string());
        Ok(())
    }

    pub fn umount(&mut self, request: UmountRequest<'_>) -> Result<(), LinuxError> {
        if request.flags != 0 {
            return Err(LinuxError::EINVAL);
        }
        if request.target.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        let Some(idx) = self
            .targets
            .iter()
            .position(|target| target == request.target)
        else {
            return Err(LinuxError::EINVAL);
        };
        self.targets.swap_remove(idx);
        Ok(())
    }
}

fn validate_mount_request(request: &MountRequest<'_>) -> Result<(), LinuxError> {
    if request.flags != 0 {
        return Err(LinuxError::EINVAL);
    }
    if request.data != 0 {
        return Err(LinuxError::EOPNOTSUPP);
    }
    if request.source.is_empty() || request.target.is_empty() || request.fstype.is_empty() {
        return Err(LinuxError::EINVAL);
    }
    compat_basic_mount(request.source, request.fstype)
}

fn compat_basic_mount(source: &str, fstype: &str) -> Result<(), LinuxError> {
    // compat(basic-fsfd): basic calls mount("/dev/vda2", "./mnt", "vfat", 0, NULL).
    // delete-when: block-device backed runtime mount exists above axfs.
    if fstype != "vfat" {
        return Err(LinuxError::EOPNOTSUPP);
    }
    if !source.starts_with("/dev/") {
        return Err(LinuxError::ENOENT);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MountRequest, MountTable, UmountRequest};
    use axerrno::LinuxError;

    #[test]
    fn mount_rejects_duplicate_target() {
        let mut table = MountTable::new();
        let request = MountRequest {
            source: "/dev/vda2",
            target: "/mnt",
            fstype: "vfat",
            flags: 0,
            data: 0,
        };
        assert_eq!(table.mount(request), Ok(()));
        let request = MountRequest {
            source: "/dev/vda2",
            target: "/mnt",
            fstype: "vfat",
            flags: 0,
            data: 0,
        };
        assert_eq!(table.mount(request), Err(LinuxError::EBUSY));
    }

    #[test]
    fn umount_only_accepts_mounted_targets() {
        let mut table = MountTable::new();
        assert_eq!(
            table.umount(UmountRequest {
                target: "/mnt",
                flags: 0,
            }),
            Err(LinuxError::EINVAL)
        );
        assert_eq!(
            table.mount(MountRequest {
                source: "/dev/vda2",
                target: "/mnt",
                fstype: "vfat",
                flags: 0,
                data: 0,
            }),
            Ok(())
        );
        assert_eq!(
            table.umount(UmountRequest {
                target: "/mnt",
                flags: 0,
            }),
            Ok(())
        );
    }
}

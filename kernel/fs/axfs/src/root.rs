//! Root directory of the filesystem
//!
//! TODO: it doesn't work very well if the mount points have containment relationships.

use alloc::{string::String, sync::Arc, vec::Vec};
use axerrno::{AxError, AxResult, ax_err};
use axfs_vfs::{VfsDirEntry, VfsNodeAttr, VfsNodeOps, VfsNodeRef, VfsNodeType, VfsOps, VfsResult};
use axns::{ResArc, def_resource};
use axsync::Mutex;
use core::array;
use lazyinit::LazyInit;

use crate::{api::FileType, fs, mounts};

def_resource! {
    static CURRENT_DIR_PATH: ResArc<Mutex<String>> = ResArc::new();
    static CURRENT_DIR: ResArc<Mutex<VfsNodeRef>> = ResArc::new();
}

struct MountPoint {
    path: &'static str,
    fs: Arc<dyn VfsOps>,
}

struct MountAnchor {
    parent: VfsNodeRef,
}

struct RootDirectory {
    main_fs: Arc<dyn VfsOps>,
    mounts: Vec<MountPoint>,
}

static ROOT_DIR: LazyInit<Arc<RootDirectory>> = LazyInit::new();

impl MountPoint {
    pub fn new(path: &'static str, fs: Arc<dyn VfsOps>) -> Self {
        Self { path, fs }
    }

    fn name(&self) -> &str {
        self.path
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(self.path.trim_matches('/'))
    }
}

impl MountAnchor {
    fn new(parent: VfsNodeRef) -> Self {
        Self { parent }
    }
}

impl VfsNodeOps for MountAnchor {
    axfs_vfs::impl_vfs_dir_default! {}

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new_dir(0, 0))
    }

    fn parent(&self) -> Option<VfsNodeRef> {
        Some(self.parent.clone())
    }

    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        let path = path.trim_matches('/');
        if path.is_empty() || path == "." {
            Ok(self)
        } else {
            ax_err!(NotFound)
        }
    }

    fn read_dir(&self, _start_idx: usize, _dirents: &mut [VfsDirEntry]) -> VfsResult<usize> {
        Ok(0)
    }
}

impl Drop for MountPoint {
    fn drop(&mut self) {
        self.fs.umount().ok();
    }
}

impl RootDirectory {
    pub const fn new(main_fs: Arc<dyn VfsOps>) -> Self {
        Self {
            main_fs,
            mounts: Vec::new(),
        }
    }

    pub fn mount(&mut self, path: &'static str, fs: Arc<dyn VfsOps>) -> AxResult {
        if path == "/" {
            return ax_err!(InvalidInput, "cannot mount root filesystem");
        }
        if !path.starts_with('/') {
            return ax_err!(InvalidInput, "mount path must start with '/'");
        }
        if self.mounts.iter().any(|mp| mp.path == path) {
            return ax_err!(InvalidInput, "mount point already exists");
        }
        let mount_point = self.prepare_mount_point(path)?;
        fs.mount(path, mount_point)?;
        self.mounts.push(MountPoint::new(path, fs));
        Ok(())
    }

    pub fn _umount(&mut self, path: &str) {
        self.mounts.retain(|mp| mp.path != path);
    }

    pub fn contains(&self, path: &str) -> bool {
        self.mounts.iter().any(|mp| mp.path == path)
    }

    fn prepare_mount_point(&self, path: &'static str) -> AxResult<VfsNodeRef> {
        let root = self.main_fs.root_dir();
        if let Ok(node) = root.clone().lookup(path) {
            return Ok(node);
        }

        match root.create(path, FileType::Dir) {
            Ok(()) | Err(AxError::AlreadyExists) => match root.clone().lookup(path) {
                Ok(node) => return Ok(node),
                Err(AxError::NotFound) => {}
                Err(err) => return Err(err),
            },
            Err(AxError::Unsupported | AxError::ReadOnlyFilesystem | AxError::PermissionDenied) => {}
            Err(AxError::NotFound) => {}
            Err(err) => return Err(err),
        }

        Ok(Arc::new(MountAnchor::new(root)))
    }

    fn count_main_root_entries(&self) -> AxResult<usize> {
        let root = self.main_fs.root_dir();
        let mut count = 0;
        loop {
            let mut buf: [VfsDirEntry; 16] = array::from_fn(|_| VfsDirEntry::default());
            let read = root.read_dir(count, &mut buf)?;
            count += read;
            if read < buf.len() {
                return Ok(count);
            }
        }
    }

    fn synthetic_mount_entries(&self) -> Vec<VfsDirEntry> {
        let root = self.main_fs.root_dir();
        let mut entries = Vec::new();
        for mp in &self.mounts {
            if root.clone().lookup(mp.path).is_err() {
                entries.push(VfsDirEntry::new(mp.name(), VfsNodeType::Dir));
            }
        }
        entries
    }

    fn mount_match_len(path: &str, mount_path: &str) -> Option<usize> {
        let mount_path = mount_path.trim_start_matches('/');
        let rest = path.strip_prefix(mount_path)?;
        if rest.is_empty() || rest.starts_with('/') {
            Some(mount_path.len())
        } else {
            None
        }
    }

    fn lookup_mounted_fs<F, T>(&self, path: &str, f: F) -> AxResult<T>
    where
        F: FnOnce(Arc<dyn VfsOps>, &str) -> AxResult<T>,
    {
        debug!("lookup at root: {}", path);
        let path = path.trim_matches('/');
        if let Some(rest) = path.strip_prefix("./") {
            return self.lookup_mounted_fs(rest, f);
        }

        let mut idx = 0;
        let mut max_len = 0;

        // Find the filesystem that has the longest mounted path match
        // TODO: more efficient, e.g. trie
        for (i, mp) in self.mounts.iter().enumerate() {
            if let Some(matched_len) = Self::mount_match_len(path, mp.path) {
                if matched_len > max_len {
                    max_len = matched_len;
                    idx = i;
                }
            }
        }

        if max_len == 0 {
            f(self.main_fs.clone(), path) // not matched any mount point
        } else {
            f(self.mounts[idx].fs.clone(), &path[max_len..]) // matched at `idx`
        }
    }
}

impl VfsNodeOps for RootDirectory {
    axfs_vfs::impl_vfs_dir_default! {}

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        self.main_fs.root_dir().get_attr()
    }

    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        self.lookup_mounted_fs(path, |fs, rest_path| fs.root_dir().lookup(rest_path))
    }

    fn create(&self, path: &str, ty: VfsNodeType) -> VfsResult {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                Ok(()) // already exists
            } else {
                fs.root_dir().create(rest_path, ty)
            }
        })
    }

    fn remove(&self, path: &str) -> VfsResult {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(PermissionDenied) // cannot remove mount points
            } else {
                fs.root_dir().remove(rest_path)
            }
        })
    }

    fn rename(&self, src_path: &str, dst_path: &str) -> VfsResult {
        self.lookup_mounted_fs(src_path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(PermissionDenied) // cannot rename mount points
            } else {
                fs.root_dir().rename(rest_path, dst_path)
            }
        })
    }

    fn read_dir(&self, start_idx: usize, dirents: &mut [VfsDirEntry]) -> VfsResult<usize> {
        let main_root = self.main_fs.root_dir();
        let main_count = self.count_main_root_entries()?;
        let mut written = 0;

        if start_idx < main_count {
            written = main_root.read_dir(start_idx, dirents)?;
            if written == dirents.len() {
                return Ok(written);
            }
        }

        let synthetic_entries = self.synthetic_mount_entries();
        let synth_start = start_idx.saturating_sub(main_count);
        for entry in synthetic_entries
            .into_iter()
            .skip(synth_start)
            .take(dirents.len() - written)
        {
            dirents[written] = entry;
            written += 1;
        }
        Ok(written)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RootFileSystemKind {
    Fat,
    Ext4,
    Unknown,
}

fn detect_rootfs(disk: &mut crate::dev::Disk) -> RootFileSystemKind {
    fn read_signature(disk: &mut crate::dev::Disk, offset: u64) -> Option<[u8; 2]> {
        let mut sig = [0; 2];
        disk.set_position(offset);
        match disk.read_one(&mut sig) {
            Ok(2) => Some(sig),
            _ => None,
        }
    }

    let saved_pos = disk.position();
    let fat_sig = read_signature(disk, 510);
    let ext4_sig = read_signature(disk, 1024 + 56);
    disk.set_position(saved_pos);

    match (fat_sig, ext4_sig) {
        (_, Some([0x53, 0xef])) => RootFileSystemKind::Ext4,
        (Some([0x55, 0xaa]), _) => RootFileSystemKind::Fat,
        _ => RootFileSystemKind::Unknown,
    }
}

#[cfg(feature = "fatfs")]
fn init_fat_mainfs(disk: crate::dev::Disk) -> Arc<dyn VfsOps> {
    static FAT_FS: LazyInit<Arc<fs::fatfs::FatFileSystem>> = LazyInit::new();
    FAT_FS.init_once(Arc::new(fs::fatfs::FatFileSystem::new(disk)));
    FAT_FS.init();
    FAT_FS.clone()
}

#[cfg(not(feature = "fatfs"))]
fn init_fat_mainfs(_disk: crate::dev::Disk) -> Arc<dyn VfsOps> {
    panic!("root disk is FAT, but this build does not enable FAT support");
}

#[cfg(feature = "ext4fs")]
fn init_ext4_mainfs(disk: crate::dev::Disk) -> Arc<dyn VfsOps> {
    Arc::new(fs::ext4fs::Ext4FileSystem::new(disk))
}

#[cfg(not(feature = "ext4fs"))]
fn init_ext4_mainfs(_disk: crate::dev::Disk) -> Arc<dyn VfsOps> {
    panic!("root disk is ext4, but this build does not enable ext4 support");
}

pub(crate) fn init_rootfs(mut disk: crate::dev::Disk) {
    let detected_rootfs = detect_rootfs(&mut disk);
    info!("  detected root filesystem: {:?}", detected_rootfs);

    #[cfg(feature = "myfs")]
    let main_fs = fs::myfs::new_myfs(disk);

    #[cfg(not(feature = "myfs"))]
    let main_fs = match detected_rootfs {
        RootFileSystemKind::Ext4 => init_ext4_mainfs(disk),
        RootFileSystemKind::Fat | RootFileSystemKind::Unknown => init_fat_mainfs(disk),
    };

    let mut root_dir = RootDirectory::new(main_fs);

    #[cfg(feature = "devfs")]
    root_dir
        .mount("/dev", mounts::devfs())
        .expect("failed to mount devfs at /dev");

    #[cfg(feature = "ramfs")]
    root_dir
        .mount("/tmp", mounts::ramfs())
        .expect("failed to mount ramfs at /tmp");

    #[cfg(feature = "ramfs")]
    root_dir
        .mount("/var", mounts::ramfs())
        .expect("failed to mount ramfs at /var");

    // Mount another ramfs as procfs
    #[cfg(feature = "procfs")]
    root_dir // should not fail
        .mount("/proc", mounts::procfs().unwrap())
        .expect("fail to mount procfs at /proc");

    // Mount another ramfs as sysfs
    #[cfg(feature = "sysfs")]
    root_dir // should not fail
        .mount("/sys", mounts::sysfs().unwrap())
        .expect("fail to mount sysfs at /sys");

    ROOT_DIR.init_once(Arc::new(root_dir));
    CURRENT_DIR.init_new(Mutex::new(ROOT_DIR.clone()));
    CURRENT_DIR_PATH.init_new(Mutex::new("/".into()));
}

fn parent_node_of(dir: Option<&VfsNodeRef>, path: &str) -> VfsNodeRef {
    if path.starts_with('/') {
        ROOT_DIR.clone()
    } else {
        dir.cloned().unwrap_or_else(|| CURRENT_DIR.lock().clone())
    }
}

pub(crate) fn absolute_path(path: &str) -> AxResult<String> {
    if path.starts_with('/') {
        Ok(axfs_vfs::path::canonicalize(path))
    } else {
        let path = CURRENT_DIR_PATH.lock().clone() + path;
        Ok(axfs_vfs::path::canonicalize(&path))
    }
}

pub(crate) fn lookup(dir: Option<&VfsNodeRef>, path: &str) -> AxResult<VfsNodeRef> {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let node = parent_node_of(dir, path).lookup(path)?;
    if path.ends_with('/') && !node.get_attr()?.is_dir() {
        ax_err!(NotADirectory)
    } else {
        Ok(node)
    }
}

pub(crate) fn create_file(dir: Option<&VfsNodeRef>, path: &str) -> AxResult<VfsNodeRef> {
    if path.is_empty() {
        return ax_err!(NotFound);
    } else if path.ends_with('/') {
        return ax_err!(NotADirectory);
    }
    let parent = parent_node_of(dir, path);
    parent.create(path, VfsNodeType::File)?;
    parent.lookup(path)
}

pub(crate) fn create_dir(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    match lookup(dir, path) {
        Ok(_) => ax_err!(AlreadyExists),
        Err(AxError::NotFound) => parent_node_of(dir, path).create(path, VfsNodeType::Dir),
        Err(e) => Err(e),
    }
}

pub(crate) fn remove_file(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    let node = lookup(dir, path)?;
    let attr = node.get_attr()?;
    if attr.is_dir() {
        ax_err!(IsADirectory)
    } else if !attr.perm().owner_writable() {
        ax_err!(PermissionDenied)
    } else {
        parent_node_of(dir, path).remove(path)
    }
}

pub(crate) fn remove_dir(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let path_check = path.trim_matches('/');
    if path_check.is_empty() {
        return ax_err!(DirectoryNotEmpty); // rm -d '/'
    } else if path_check == "."
        || path_check == ".."
        || path_check.ends_with("/.")
        || path_check.ends_with("/..")
    {
        return ax_err!(InvalidInput);
    }
    if ROOT_DIR.contains(&absolute_path(path)?) {
        return ax_err!(PermissionDenied);
    }

    let node = lookup(dir, path)?;
    let attr = node.get_attr()?;
    if !attr.is_dir() {
        ax_err!(NotADirectory)
    } else if !attr.perm().owner_writable() {
        ax_err!(PermissionDenied)
    } else {
        parent_node_of(dir, path).remove(path)
    }
}

pub(crate) fn current_dir() -> AxResult<String> {
    Ok(CURRENT_DIR_PATH.lock().clone())
}

pub(crate) fn set_current_dir(path: &str) -> AxResult {
    let mut abs_path = absolute_path(path)?;
    if !abs_path.ends_with('/') {
        abs_path += "/";
    }
    if abs_path == "/" {
        *CURRENT_DIR.lock() = ROOT_DIR.clone();
        *CURRENT_DIR_PATH.lock() = "/".into();
        return Ok(());
    }

    let node = lookup(None, &abs_path)?;
    let attr = node.get_attr()?;
    if !attr.is_dir() {
        ax_err!(NotADirectory)
    } else if !attr.perm().owner_executable() {
        ax_err!(PermissionDenied)
    } else {
        *CURRENT_DIR.lock() = node;
        *CURRENT_DIR_PATH.lock() = abs_path;
        Ok(())
    }
}

pub(crate) fn rename(old: &str, new: &str) -> AxResult {
    if parent_node_of(None, new).lookup(new).is_ok() {
        warn!("dst file already exist, now remove it");
        remove_file(None, new)?;
    }
    parent_node_of(None, old).rename(old, new)
}

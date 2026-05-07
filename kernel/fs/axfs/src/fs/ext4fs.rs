use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    sync::Arc,
};
use axfs_vfs::{
    VfsDirEntry, VfsError, VfsNodeAttr, VfsNodeOps, VfsNodePerm, VfsNodeRef, VfsNodeType, VfsOps,
    VfsResult,
};
use axsync::Mutex;
use core::{cell::UnsafeCell, error::Error, fmt};
use ext4_view::{Ext4, Ext4Error, Ext4Read, FileType};

use crate::dev::Disk;

const BLOCK_SIZE: u64 = 512;

pub struct Ext4FileSystem {
    root_dir: Arc<Ext4DirNode>,
}

struct LockedExt4 {
    lock: Mutex<()>,
    inner: UnsafeCell<Ext4>,
}

struct Ext4Disk(Disk);

#[derive(Debug)]
struct Ext4DiskReadError;

struct Ext4FileNode {
    fs: Arc<LockedExt4>,
    path: String,
}

struct Ext4DirNode {
    fs: Arc<LockedExt4>,
    path: String,
}

impl Ext4FileSystem {
    pub fn new(disk: Disk) -> Self {
        let ext4 =
            Ext4::load(Box::new(Ext4Disk(disk))).expect("failed to initialize ext4 filesystem");
        let fs = Arc::new(LockedExt4::new(ext4));
        Self {
            root_dir: Arc::new(Ext4DirNode::new(fs, "/".into())),
        }
    }
}

impl LockedExt4 {
    fn new(inner: Ext4) -> Self {
        Self {
            lock: Mutex::new(()),
            inner: UnsafeCell::new(inner),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&Ext4) -> VfsResult<R>) -> VfsResult<R> {
        let _guard = self.lock.lock();
        // SAFETY: all access goes through this method and is serialized by `lock`.
        let fs = unsafe { &*self.inner.get() };
        f(fs)
    }
}

// SAFETY: the inner `Ext4` object is only accessed while holding `lock`.
unsafe impl Send for LockedExt4 {}
// SAFETY: the inner `Ext4` object is only accessed while holding `lock`.
unsafe impl Sync for LockedExt4 {}

impl fmt::Display for Ext4DiskReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to read from ext4 disk")
    }
}

impl Error for Ext4DiskReadError {}

impl Ext4Read for Ext4Disk {
    fn read(
        &mut self,
        start_byte: u64,
        dst: &mut [u8],
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        self.0.set_position(start_byte);
        let mut filled = 0;
        while filled < dst.len() {
            let read = self
                .0
                .read_one(&mut dst[filled..])
                .map_err(|_| Box::new(Ext4DiskReadError) as Box<dyn Error + Send + Sync>)?;
            if read == 0 {
                return Err(Box::new(Ext4DiskReadError));
            }
            filled += read;
        }
        Ok(())
    }
}

impl Ext4FileNode {
    fn new(fs: Arc<LockedExt4>, path: String) -> Self {
        Self { fs, path }
    }

    fn metadata(&self) -> VfsResult<ext4_view::Metadata> {
        self.fs
            .with(|fs| fs.metadata(self.path.as_str()).map_err(as_vfs_err))
    }
}

impl Ext4DirNode {
    fn new(fs: Arc<LockedExt4>, path: String) -> Self {
        Self { fs, path }
    }

    fn metadata(&self) -> VfsResult<ext4_view::Metadata> {
        self.fs
            .with(|fs| fs.metadata(self.path.as_str()).map_err(as_vfs_err))
    }

    fn child_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            axfs_vfs::path::canonicalize(path)
        } else if self.path == "/" {
            axfs_vfs::path::canonicalize(&format!("/{}", path))
        } else {
            axfs_vfs::path::canonicalize(&format!("{}/{}", self.path, path))
        }
    }

    fn parent_path(&self) -> Option<String> {
        if self.path == "/" {
            None
        } else {
            Some(axfs_vfs::path::canonicalize(&(self.path.clone() + "/..")))
        }
    }
}

impl VfsNodeOps for Ext4FileNode {
    axfs_vfs::impl_vfs_non_dir_default! {}

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        let metadata = self.metadata()?;
        Ok(vfs_attr_from_metadata(&metadata))
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        self.fs.with(|fs| {
            let mut file = fs.open(self.path.as_str()).map_err(as_vfs_err)?;
            file.seek_to(offset).map_err(as_vfs_err)?;
            file.read_bytes(buf).map_err(as_vfs_err)
        })
    }

    fn write_at(&self, _offset: u64, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    fn fsync(&self) -> VfsResult {
        Err(VfsError::ReadOnlyFilesystem)
    }

    fn truncate(&self, _size: u64) -> VfsResult {
        Err(VfsError::ReadOnlyFilesystem)
    }
}

impl VfsNodeOps for Ext4DirNode {
    axfs_vfs::impl_vfs_dir_default! {}

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        let metadata = self.metadata()?;
        Ok(vfs_attr_from_metadata(&metadata))
    }

    fn parent(&self) -> Option<VfsNodeRef> {
        self.parent_path()
            .map(|path| Arc::new(Self::new(self.fs.clone(), path)) as VfsNodeRef)
    }

    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        let path = path.trim_matches('/');
        if path.is_empty() || path == "." {
            return Ok(self);
        }
        if let Some(rest) = path.strip_prefix("./") {
            return self.lookup(rest);
        }

        let path = self.child_path(path);
        let file_type = self.fs.with(|fs| {
            fs.metadata(path.as_str())
                .map(|metadata| map_file_type(metadata.file_type()))
                .map_err(as_vfs_err)
        })?;
        if file_type.is_dir() {
            Ok(Arc::new(Self::new(self.fs.clone(), path)))
        } else {
            Ok(Arc::new(Ext4FileNode::new(self.fs.clone(), path)))
        }
    }

    fn create(&self, _path: &str, _ty: VfsNodeType) -> VfsResult {
        Err(VfsError::ReadOnlyFilesystem)
    }

    fn remove(&self, _path: &str) -> VfsResult {
        Err(VfsError::ReadOnlyFilesystem)
    }

    fn read_dir(&self, start_idx: usize, dirents: &mut [VfsDirEntry]) -> VfsResult<usize> {
        self.fs.with(|fs| {
            let mut iter = fs.read_dir(self.path.as_str()).map_err(as_vfs_err)?;
            for _ in 0..start_idx {
                match iter.next().transpose().map_err(as_vfs_err)? {
                    Some(_) => {}
                    None => return Ok(0),
                }
            }

            for (i, out_entry) in dirents.iter_mut().enumerate() {
                match iter.next().transpose().map_err(as_vfs_err)? {
                    Some(entry) => {
                        let name = entry.file_name().display().to_string();
                        let ty = map_file_type(entry.file_type().map_err(as_vfs_err)?);
                        *out_entry = VfsDirEntry::new(&name, ty);
                    }
                    None => return Ok(i),
                }
            }
            Ok(dirents.len())
        })
    }

    fn rename(&self, _src_path: &str, _dst_path: &str) -> VfsResult {
        Err(VfsError::ReadOnlyFilesystem)
    }
}

impl VfsOps for Ext4FileSystem {
    fn root_dir(&self) -> VfsNodeRef {
        self.root_dir.clone()
    }
}

fn vfs_attr_from_metadata(metadata: &ext4_view::Metadata) -> VfsNodeAttr {
    VfsNodeAttr::new(
        VfsNodePerm::from_bits_truncate(metadata.mode()),
        map_file_type(metadata.file_type()),
        metadata.len(),
        metadata.len().div_ceil(BLOCK_SIZE),
    )
}

fn map_file_type(file_type: FileType) -> VfsNodeType {
    match file_type {
        FileType::BlockDevice => VfsNodeType::BlockDevice,
        FileType::CharacterDevice => VfsNodeType::CharDevice,
        FileType::Directory => VfsNodeType::Dir,
        FileType::Fifo => VfsNodeType::Fifo,
        FileType::Regular => VfsNodeType::File,
        FileType::Socket => VfsNodeType::Socket,
        FileType::Symlink => VfsNodeType::SymLink,
    }
}

fn as_vfs_err(err: Ext4Error) -> VfsError {
    match err {
        Ext4Error::NotFound => VfsError::NotFound,
        Ext4Error::IsADirectory => VfsError::IsADirectory,
        Ext4Error::NotADirectory => VfsError::NotADirectory,
        Ext4Error::Encrypted => VfsError::PermissionDenied,
        Ext4Error::NotUtf8 => VfsError::InvalidData,
        Ext4Error::Io(_) => VfsError::Io,
        Ext4Error::Incompatible(_) | Ext4Error::Corrupt(_) | Ext4Error::FileTooLarge => {
            VfsError::InvalidData
        }
        Ext4Error::NotAbsolute
        | Ext4Error::NotASymlink
        | Ext4Error::IsASpecialFile
        | Ext4Error::MalformedPath
        | Ext4Error::PathTooLong
        | Ext4Error::TooManySymlinks => VfsError::InvalidInput,
        _ => VfsError::InvalidData,
    }
}

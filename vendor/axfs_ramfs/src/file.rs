use alloc::vec::Vec;
use axfs_vfs::{VfsError, VfsNodeAttr, VfsNodeOps, VfsResult, impl_vfs_non_dir_default};
use spin::RwLock;

const MAX_RAMFS_FILE_SIZE: usize = 128 * 1024 * 1024;

/// The file node in the RAM filesystem.
///
/// It implements [`axfs_vfs::VfsNodeOps`].
pub struct FileNode {
    content: RwLock<Vec<u8>>,
}

impl FileNode {
    pub(super) const fn new() -> Self {
        Self {
            content: RwLock::new(Vec::new()),
        }
    }
}

impl VfsNodeOps for FileNode {
    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new_file(self.content.read().len() as _, 0))
    }

    fn truncate(&self, size: u64) -> VfsResult {
        if size > MAX_RAMFS_FILE_SIZE as u64 {
            return Err(VfsError::StorageFull);
        }
        let mut content = self.content.write();
        if size < content.len() as u64 {
            content.truncate(size as _);
        } else {
            content.resize(size as _, 0);
        }
        Ok(())
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let content = self.content.read();
        let start = content.len().min(offset as usize);
        let end = content.len().min(offset as usize + buf.len());
        let src = &content[start..end];
        buf[..src.len()].copy_from_slice(src);
        Ok(src.len())
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let offset = offset as usize;
        let Some(end) = offset.checked_add(buf.len()) else {
            return Err(VfsError::StorageFull);
        };
        if end > MAX_RAMFS_FILE_SIZE {
            return Err(VfsError::StorageFull);
        }
        let mut content = self.content.write();
        if end > content.len() {
            content.resize(end, 0);
        }
        let dst = &mut content[offset..end];
        dst.copy_from_slice(&buf[..dst.len()]);
        Ok(buf.len())
    }

    impl_vfs_non_dir_default! {}
}

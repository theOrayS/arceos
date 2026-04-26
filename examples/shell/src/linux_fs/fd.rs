//! Linux fd-table and open-file-description helpers.

use axerrno::LinuxError;
use axfs::fops::{Directory, File, FileAttr};
use axio::SeekFrom;
use axsync::Mutex;
use std::sync::Arc;
use std::string::String;
use std::vec::Vec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FdFlags {
    raw: u32,
}

impl FdFlags {
    pub const CLOEXEC: u32 = 1;

    pub const fn empty() -> Self {
        Self { raw: 0 }
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self {
            raw: raw & Self::CLOEXEC,
        }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn cloexec(self) -> bool {
        self.raw & Self::CLOEXEC != 0
    }

    pub fn set_cloexec(&mut self, enabled: bool) {
        if enabled {
            self.raw |= Self::CLOEXEC;
        } else {
            self.raw &= !Self::CLOEXEC;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OpenStatusFlags {
    raw: u32,
}

impl OpenStatusFlags {
    pub const APPEND: u32 = 0o2000;
    pub const NONBLOCK: u32 = 0o4000;

    pub const fn from_raw(raw: u32) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub fn set_raw(&mut self, raw: u32) {
        self.raw = raw;
    }

    pub const fn append(self) -> bool {
        self.raw & Self::APPEND != 0
    }
}

pub struct FileBackend {
    pub file: Mutex<File>,
    pub path: String,
}

pub struct DirectoryBackend {
    pub dir: Mutex<Directory>,
    pub attr: FileAttr,
    pub path: String,
}

pub enum OpenFileBackend {
    File(FileBackend),
    Directory(DirectoryBackend),
}

pub struct OpenFileDescription {
    pub status_flags: Mutex<OpenStatusFlags>,
    pub offset: Mutex<u64>,
    pub backend: OpenFileBackend,
}

pub type SharedOpenFileDescription = Arc<OpenFileDescription>;

impl OpenFileDescription {
    pub fn new_file(file: File, path: String, status_flags: OpenStatusFlags) -> Self {
        Self {
            status_flags: Mutex::new(status_flags),
            offset: Mutex::new(0),
            backend: OpenFileBackend::File(FileBackend {
                file: Mutex::new(file),
                path,
            }),
        }
    }

    pub fn new_directory(dir: Directory, attr: FileAttr, path: String) -> Self {
        Self {
            status_flags: Mutex::new(OpenStatusFlags::default()),
            offset: Mutex::new(0),
            backend: OpenFileBackend::Directory(DirectoryBackend {
                dir: Mutex::new(dir),
                attr,
                path,
            }),
        }
    }

    pub fn path(&self) -> &str {
        match &self.backend {
            OpenFileBackend::File(file) => file.path.as_str(),
            OpenFileBackend::Directory(dir) => dir.path.as_str(),
        }
    }

    pub fn attr(&self) -> Result<FileAttr, LinuxError> {
        match &self.backend {
            OpenFileBackend::File(file) => file.file.lock().get_attr().map_err(LinuxError::from),
            OpenFileBackend::Directory(dir) => Ok(dir.attr),
        }
    }

    pub fn read_file(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::EISDIR);
        };
        let mut offset = self.offset.lock();
        let mut file = file.file.lock();
        file.seek(SeekFrom::Start(*offset))
            .map_err(LinuxError::from)?;
        let n = file.read(dst).map_err(LinuxError::from)?;
        *offset += n as u64;
        Ok(n)
    }

    pub fn write_file(&self, src: &[u8]) -> Result<usize, LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::EBADF);
        };
        let append = self.status_flags.lock().append();
        let mut offset = self.offset.lock();
        let mut file = file.file.lock();
        if append {
            let end = file.seek(SeekFrom::End(0)).map_err(LinuxError::from)?;
            *offset = end;
        } else {
            file.seek(SeekFrom::Start(*offset))
                .map_err(LinuxError::from)?;
        }
        let n = file.write(src).map_err(LinuxError::from)?;
        *offset += n as u64;
        Ok(n)
    }

    pub fn seek_file(&self, offset: i64, whence: u32) -> Result<u64, LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::ESPIPE);
        };
        let pos = match whence {
            0 => {
                if offset < 0 {
                    return Err(LinuxError::EINVAL);
                }
                SeekFrom::Start(offset as u64)
            }
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => return Err(LinuxError::EINVAL),
        };
        let mut file = file.file.lock();
        let new_offset = file.seek(pos).map_err(LinuxError::from)?;
        *self.offset.lock() = new_offset;
        Ok(new_offset)
    }

    pub fn pread_file(&self, dst: &mut [u8], offset: u64) -> Result<usize, LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::EISDIR);
        };
        let mut file = file.file.lock();
        file.seek(SeekFrom::Start(offset))
            .map_err(LinuxError::from)?;
        file.read(dst).map_err(LinuxError::from)
    }

    pub fn read_file_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::EBADF);
        };
        let mut buf = vec![0u8; len];
        let mut filled = 0usize;
        let file = file.file.lock();
        while filled < buf.len() {
            let read = file
                .read_at(offset + filled as u64, &mut buf[filled..])
                .map_err(LinuxError::from)?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        buf.truncate(filled);
        Ok(buf)
    }

    pub fn truncate_file(&self, size: u64) -> Result<(), LinuxError> {
        let OpenFileBackend::File(file) = &self.backend else {
            return Err(LinuxError::EINVAL);
        };
        file.file.lock().truncate(size).map_err(LinuxError::from)
    }
}

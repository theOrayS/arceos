use axfs::fops::{Directory, File, FileAttr};
use std::string::String;
use std::sync::Arc;
use std::vec::Vec;

use super::fd_pipe::PipeEndpoint;
use super::fd_socket::{LocalSocketEntry, SocketEntry};

pub(super) struct FdTable {
    pub(super) entries: Vec<Option<FdEntry>>,
    pub(super) fd_flags: Vec<u32>,
}

pub(super) enum FdEntry {
    Stdin,
    Stdout,
    Stderr,
    DevNull,
    Rtc,
    File(FileEntry),
    Directory(DirectoryEntry),
    Path(PathEntry),
    MemoryFile(MemoryFileEntry),
    Pipe(PipeEndpoint),
    Socket(SocketEntry),
    LocalSocket(LocalSocketEntry),
}

#[derive(Clone)]
pub(super) struct FileEntry {
    pub(super) file: File,
    pub(super) path: String,
}

#[derive(Clone)]
pub(super) struct DirectoryEntry {
    pub(super) dir: Directory,
    pub(super) attr: FileAttr,
    pub(super) path: String,
}

#[derive(Clone)]
pub(super) struct PathEntry {
    pub(super) path: String,
    pub(super) mode: u32,
    pub(super) size: u64,
    pub(super) blocks: u64,
}

#[derive(Clone)]
pub(super) struct MemoryFileEntry {
    pub(super) path: String,
    pub(super) data: Arc<Vec<u8>>,
    pub(super) offset: usize,
}

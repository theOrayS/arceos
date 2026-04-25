//! Future home for Linux fd-table and open-file-description semantics.
//!
//! Phase one intentionally keeps `FdTable` in `uspace.rs` to avoid fd ownership
//! churn while path, mount, and statx semantics are split out.

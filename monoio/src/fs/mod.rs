//! Filesystem manipulation operations.

mod file;
pub use file::File;

mod open_options;
pub use open_options::OpenOptions;

/// Metadata information about a file.
pub struct Metadata(crate::driver::op::FileAttr);

impl Metadata {
    /// Returns the size of the file, in bytes, this metadata is for.
    pub fn len(&self) -> u64 {
        self.0.size()
    }
}

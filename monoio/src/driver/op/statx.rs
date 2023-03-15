#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};

use super::{Op, OpAble};
use crate::driver::shared_fd::SharedFd;
#[cfg(all(unix, feature = "legacy"))]
use crate::{driver::legacy::ready::Direction, syscall_u32};

pub(crate) struct Statx {
    fd: SharedFd,
    // Put it into the box, such that we pas pointer to io-uring rather than bitwise-copy
    buf: Box<libc::statx>,
}

impl Statx {
    pub(crate) fn read(&self) -> FileAttr {
        // We cannot fill `stat64` exhaustively because of private padding fields.
        let mut stat: libc::stat64 = unsafe { std::mem::zeroed() };
        // `c_ulong` on gnu-mips, `dev_t` otherwise
        stat.st_dev = libc::makedev(self.buf.stx_dev_major, self.buf.stx_dev_minor) as _;
        stat.st_ino = self.buf.stx_ino as libc::ino64_t;
        stat.st_nlink = self.buf.stx_nlink as libc::nlink_t;
        stat.st_mode = self.buf.stx_mode as libc::mode_t;
        stat.st_uid = self.buf.stx_uid as libc::uid_t;
        stat.st_gid = self.buf.stx_gid as libc::gid_t;
        stat.st_rdev = libc::makedev(self.buf.stx_rdev_major, self.buf.stx_rdev_minor) as _;
        stat.st_size = self.buf.stx_size as libc::off64_t;
        stat.st_blksize = self.buf.stx_blksize as libc::blksize_t;
        stat.st_blocks = self.buf.stx_blocks as libc::blkcnt64_t;
        stat.st_atime = self.buf.stx_atime.tv_sec as libc::time_t;
        // `i64` on gnu-x86_64-x32, `c_ulong` otherwise.
        stat.st_atime_nsec = self.buf.stx_atime.tv_nsec as _;
        stat.st_mtime = self.buf.stx_mtime.tv_sec as libc::time_t;
        stat.st_mtime_nsec = self.buf.stx_mtime.tv_nsec as _;
        stat.st_ctime = self.buf.stx_ctime.tv_sec as libc::time_t;
        stat.st_ctime_nsec = self.buf.stx_ctime.tv_nsec as _;

        let extra = StatxExtraFields {
            stx_mask: self.buf.stx_mask,
            stx_btime: self.buf.stx_btime,
            // Store full times to avoid 32-bit `time_t` truncation.
            #[cfg(target_pointer_width = "32")]
            stx_atime: self.buf.stx_atime,
            #[cfg(target_pointer_width = "32")]
            stx_ctime: self.buf.stx_ctime,
            #[cfg(target_pointer_width = "32")]
            stx_mtime: self.buf.stx_mtime,
        };
        FileAttr {
            stat,
            statx_extra_fields: Some(extra),
        }
    }
}

impl Op<Statx> {
    /// Statx syscall on file
    pub(crate) fn statx(fd: &SharedFd) -> std::io::Result<Op<Statx>> {
        Op::submit_with(Statx {
            fd: fd.clone(),
            buf: unsafe { Box::new(std::mem::zeroed()) },
        })
    }
}

impl OpAble for Statx {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        opcode::Statx::new(
            types::Fd(self.fd.raw_fd()),
            b"\0" as *const _ as *const libc::c_char,
            self.buf.as_mut() as *mut libc::statx as *mut _,
        )
        .flags(libc::AT_EMPTY_PATH)
        .mask(libc::STATX_ALL)
        .build()
        .user_data(0x99)
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        None
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> std::io::Result<u32> {
        syscall_u32!(statx(
            self.fd.raw_fd(),
            b"\0" as *const _ as *const libc::c_char,
            libc::AT_EMPTY_PATH | libc::AT_STATX_SYNC_AS_STAT,
            libc::STATX_ALL,
            self.buf.as_mut(),
        ))
    }
}

/// FileAttr copied from std, because constructing Metadata is private
#[derive(Debug)]
pub(crate) struct FileAttr {
    stat: libc::stat64,
    statx_extra_fields: Option<StatxExtraFields>,
}

#[derive(Clone, Debug)]
struct StatxExtraFields {
    // This is needed to check if btime is supported by the filesystem.
    stx_mask: u32,
    stx_btime: libc::statx_timestamp,
    // With statx, we can overcome 32-bit `time_t` too.
    #[cfg(target_pointer_width = "32")]
    stx_atime: libc::statx_timestamp,
    #[cfg(target_pointer_width = "32")]
    stx_ctime: libc::statx_timestamp,
    #[cfg(target_pointer_width = "32")]
    stx_mtime: libc::statx_timestamp,
}

impl FileAttr {
    pub(crate) fn size(&self) -> u64 {
        self.stat.st_size as u64
    }
}

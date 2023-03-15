#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};
#[cfg(all(unix, feature = "legacy"))]
use {
    crate::{driver::legacy::ready::Direction, syscall_u32},
    std::os::unix::prelude::AsRawFd,
};

use super::OpAble;
use crate::driver::shared_fd::SharedFd;

pub(crate) struct Statx {
    fd: SharedFd,
    buf: libc::statx,
}

impl OpAble for Statx {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        opcode::Statx::new(
            types::Fd(self.fd.raw_fd()),
            b"\0" as *const _ as *const libc::c_char,
            &mut self.buf as *mut libc::statx as *mut _,
        )
        .flags(libc::AT_EMPTY_PATH | libc::AT_STATX_SYNC_AS_STAT)
        .mask(libc::STATX_ALL)
        .build()
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
            &mut self.buf,
        ))
    }
}

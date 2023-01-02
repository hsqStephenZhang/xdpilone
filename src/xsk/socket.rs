use alloc::sync::Arc;

use crate::Errno;
use crate::xsk::{XskSocket, IfInfo, SocketFd, XskUmem};

impl XskSocket {
    const SO_NETNS_COOKIE: libc::c_int = 71;
    const INIT_NS: u64 = 1;

    pub fn new(interface: &IfInfo) -> Result<Self, Errno> {
        let fd = Arc::new(SocketFd::new()?);
        Self::with_xdp_socket(interface, fd)
    }

    /// Create a socket using the FD of the `umem`.
    ///
    /// # Safety
    ///
    /// It's *not* (memory-)unsafe to run this twice with different interfaces but it's also
    /// incorrect. Please don't.
    pub fn with_shared(interface: &IfInfo, umem: &XskUmem) -> Result<Self, Errno> {
        Self::with_xdp_socket(interface, umem.fd.clone())
    }

    fn with_xdp_socket(interface: &IfInfo, fd: Arc<SocketFd>) -> Result<Self, Errno> {
        let mut info = Arc::new(interface.clone());

        let mut netnscookie: u64 = 0;
        let mut optlen: libc::socklen_t = core::mem::size_of_val(&netnscookie) as libc::socklen_t;
        let err = unsafe {
            libc::getsockopt(
                fd.0,
                libc::SOL_SOCKET,
                Self::SO_NETNS_COOKIE,
                (&mut netnscookie) as *mut _ as *mut libc::c_void,
                &mut optlen,
            )
        };

        match err {
            0 => {}
            libc::ENOPROTOOPT => netnscookie = Self::INIT_NS,
            err => return Err(Errno::new()),
        }

        // Won't reallocate in practice.
        Arc::make_mut(&mut info).ctx.netnscookie = netnscookie;

        Ok(XskSocket { fd, info })
    }
}

impl SocketFd {
    pub(crate) fn new() -> Result<Self, Errno> {
        let fd = unsafe { libc::socket(libc::AF_XDP, libc::SOCK_RAW, 0) };
        if fd < 0 {
            return Err(Errno::new());
        }
        Ok(SocketFd(fd))
    }
}

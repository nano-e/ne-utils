use std::fs::File;
use std::io::{Read, Write, ErrorKind};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::{Context, Poll, self};
use futures::Future;
use futures::{ready, stream::Stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, self};
use tokio::io::{unix::AsyncFd, AsyncRead, AsyncWrite, ReadBuf};
use tokio::task::yield_now;
use crate::io::TunIo;
use crate::tun_device::{TunDevice, TunIpAddr};
use std::os::unix::io::{FromRawFd, IntoRawFd};


macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}

pub struct AsyncTunDevice {
    #[cfg(target_os = "macos")]
    io: AsyncFd<TunIo>,

    #[cfg(target_os = "linux")]
    async_fd: AsyncFd<File>,

    tun_device: TunDevice,
}
impl AsRawFd for AsyncTunDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.io.as_raw_fd()
    }
}

impl AsyncTunDevice {
    #[cfg(target_os = "macos")]
    pub fn new(tun_device: TunDevice) -> Result<Self, std::io::Error> {
        
        let async_fd = AsyncFd::new(TunIo::from(RawFd::from(tun_device.fd)))?;
        Ok(Self {            
            tun_device,
            io: async_fd,
        })
    }

    pub async fn set_ip_address (&self, ip: &TunIpAddr) -> Result<(), Box<dyn std::error::Error>>{
        self.tun_device.set_ip_address(ip).await
    }
    #[cfg(target_os = "linux")]
    pub fn new(tun_device: TunDevice) -> Result<Self, std::io::Error> {
        let async_fd = AsyncFd::new(tun_device.fd)?;
        Ok(Self { async_fd })
    }

    // #[cfg(target_os = "macos")]
    // pub async fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
    //     loop {
    //         let mut guard = self.async_fd.readable().await?;

    //         match guard.try_io(|inner| {
    //             let fd = inner.get_ref().as_raw_fd();
    //             let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;
    //             let len = buf.len();
    //             let n = unsafe { libc::read(fd, buf_ptr, len) };
    //             if n < 0 {
    //                 Err(std::io::Error::last_os_error())
    //             } else {
    //                 Ok(n as usize)
    //             }
    //         }) {
    //             Ok(res) => return res,
    //             Err(_) => continue,
    //         }
    //     }
    // }
    // #[cfg(target_os = "macos")]
    // pub async fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
    //     loop {
    //         let mut guard = self.async_fd.writable().await?;

    //         match guard.try_io(|inner| {
    //             let fd = inner.get_ref().as_raw_fd();
    //             let buf_ptr = buf.as_ptr() as *const libc::c_void;
    //             let len = buf.len();
    //             let n = unsafe { libc::write(fd, buf_ptr, len) };
    //             if n < 0 {
    //                 Err(std::io::Error::last_os_error())
    //             } else {
    //                 Ok(n as usize)
    //             }
    //         }) {
    //             Ok(res) => return res,
    //             Err(_) => continue,
    //         }
    //     }
    // }

    /// Receives a packet from the Tun/Tap interface
    ///
    /// This method takes &self, so it is possible to call this method concurrently with other methods on this struct.
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.io.readable().await?;

            match guard.try_io(|inner| inner.get_ref().recv(buf)) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
    }

    /// Sends a packet to the Tun/Tap interface
    ///
    /// This method takes &self, so it is possible to call this method concurrently with other methods on this struct.
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.io.writable().await?;

            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
    }

    /// Try to receive a packet from the Tun/Tap interface
    ///
    /// When there is no pending data, `Err(io::ErrorKind::WouldBlock)` is returned.
    ///
    /// This method takes &self, so it is possible to call this method concurrently with other methods on this struct.
    pub fn try_recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.get_ref().recv(buf)
    }

    /// Try to send a packet to the Tun/Tap interface
    ///
    /// When the socket buffer is full, `Err(io::ErrorKind::WouldBlock)` is returned.
    ///
    /// This method takes &self, so it is possible to call this method concurrently with other methods on this struct.
    pub fn try_send(&self, buf: &[u8]) -> io::Result<usize> {
        self.io.get_ref().send(buf)
    }

    #[cfg(target_os = "linux")]
    pub async fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let mut guard = self.async_fd.readable().await?;

            match guard.try_io(|inner| {
                let mut file_ref = inner.get_ref();
                file_ref.read(buf)
            }) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub async fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            let mut guard = self.async_fd.writable().await?;

            match guard.try_io(|inner| {
                let mut file_ref = inner.get_ref();
                file_ref.write(buf)
            }) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl AsyncRead for AsyncTunDevice {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> task::Poll<io::Result<()>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.io.poll_read_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().read(buf.initialize_unfilled())) {
                Ok(Ok(n)) => {
                    buf.set_filled(buf.filled().len() + n);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_) => {
                    cx.waker().wake_by_ref(); // Signal that the task should be polled again
                    return Poll::Pending;
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl AsyncWrite for AsyncTunDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> task::Poll<io::Result<usize>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.io.poll_write_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> task::Poll<io::Result<()>> {
        let self_mut = self.get_mut();
        loop {
            let mut guard = ready!(self_mut.io.poll_write_ready_mut(cx))?;

            match guard.try_io(|inner| inner.get_mut().flush()) {
                Ok(result) => return Poll::Ready(result),
                Err(_) => continue,
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> task::Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(target_os = "linux")]
impl AsyncRead for AsyncTunDevice {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {

        let fut = self.async_fd.readable();
        futures::pin_mut!(fut);

        match fut.poll(cx) {
            std::task::Poll::Ready(res) => {
                if let Err(e) = res {
                    return Poll::Ready(Err(e));
                }
            }
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }

        let result = {
            self.async_fd.get_ref().read(buf.initialize_unfilled())
        };

        match result {
            Ok(bytes_read) => {
                buf.advance(bytes_read);
                Poll::Ready(Ok(()))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl AsyncWrite for AsyncTunDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let fut = self.async_fd.writable();
        futures::pin_mut!(fut);

        match fut.poll(cx) {
            std::task::Poll::Ready(res) => {
                if let Err(e) = res {
                    return Poll::Ready(Err(e));
                }
            }
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }

        let result = {
            self.async_fd.get_ref().write(buf)
        };

        match result {
            Ok(bytes_written) => Poll::Ready(Ok(bytes_written)),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

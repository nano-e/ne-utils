use std::fs::File;
use std::io::{Read, Write, ErrorKind};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Future;
use futures::{ready, stream::Stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, self};
use tokio::io::{unix::AsyncFd, AsyncRead, AsyncWrite, ReadBuf};
use crate::tun_device::TunDevice;
use std::os::unix::io::{FromRawFd, IntoRawFd};

pub struct AsyncTunDevice {
    #[cfg(target_os = "macos")]
    async_fd: AsyncFd<RawFd>,

    #[cfg(target_os = "linux")]
    async_fd: AsyncFd<File>,
}

impl AsyncTunDevice {
    #[cfg(target_os = "macos")]
    pub fn new(tun_device: TunDevice) -> Result<Self, std::io::Error> {
        let async_fd = AsyncFd::new(tun_device.fd)?;
        Ok(Self {
            async_fd,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn new(tun_device: TunDevice) -> Result<Self, std::io::Error> {
        let async_fd = AsyncFd::new(tun_device.fd)?;
        Ok(Self { async_fd })
    }

    #[cfg(target_os = "macos")]
    pub async fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let mut guard = self.async_fd.readable().await?;

            match guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;
                let len = buf.len();
                let n = unsafe { libc::read(fd, buf_ptr, len) };
                if n < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            }) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
    }
    #[cfg(target_os = "macos")]
    pub async fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            let mut guard = self.async_fd.writable().await?;

            match guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let buf_ptr = buf.as_ptr() as *const libc::c_void;
                let len = buf.len();
                let n = unsafe { libc::write(fd, buf_ptr, len) };
                if n < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            }) {
                Ok(res) => return res,
                Err(_) => continue,
            }
        }
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
    ) -> Poll<std::io::Result<()>> {
        let me = self.get_mut();

        let mut guard = ready!(me.async_fd.poll_read_ready(cx))?;

        // Safety: The poll_read_ready call above guarantees that a read operation will not block.
        let result = unsafe {
            let ptr = buf.unfilled_mut().as_mut_ptr();
            libc::read(
                me.async_fd.get_ref().as_raw_fd(),
                ptr as *mut libc::c_void,
                buf.remaining(),
            )
        };

        match result {
            n if n >= 0 => {
                let n = n as usize;
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            _ => Poll::Ready(Err(std::io::Error::last_os_error())),
        }
    }
}

#[cfg(target_os = "macos")]
impl AsyncWrite for AsyncTunDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let me = self.get_mut();

        let mut guard = ready!(me.async_fd.poll_write_ready(cx))?;

        // Safety: The poll_write_ready call above guarantees that a write operation will not block.
        let result = unsafe {
            libc::write(
                me.async_fd.get_ref().as_raw_fd(),
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
            )
        };

        match result {
            n if n >= 0 => Poll::Ready(Ok(n as usize)),
            _ => Poll::Ready(Err(std::io::Error::last_os_error())),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
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

use std::{
    fs::File,
    io::{self, Error},
    net::{IpAddr, Ipv4Addr},
    os::unix::prelude::RawFd,
   
};

#[cfg(feature = "async")]
use tokio::process::Command;

#[cfg(not(feature = "async"))]
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvError, Sender},
        Arc,
    },
};

#[cfg(target_os = "linux")]
extern "C" {
    fn tuntap_setup(
        fd: libc::c_int,
        name: *mut u8,
        mode: libc::c_int,
        packet_info: libc::c_int,
    ) -> libc::c_int;
}

#[cfg(target_os = "macos")]
extern "C" {
    fn tuntap_setup(num: libc::c_uint) -> libc::c_int;
}

#[cfg(target_os = "macos")]
fn get_available_utun() -> Option<u32> {
    use std::{collections::HashSet, process::Command};

    let output = Command::new("ifconfig")
        .args(&["-l"])
        .output()
        .expect("failed to execute ifconfig");
    let interfaces = String::from_utf8_lossy(&output.stdout).into_owned();
    let v = interfaces
        .split([' ', '\n'])
        .filter(|v| v.starts_with("utun"))
        .filter_map(|v| v.replace("utun", "").parse::<u32>().ok())
        .collect::<HashSet<u32>>();

    for i in 0..99 {
        if !v.contains(&i) {
            return Some(i);
        }
    }
    None
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Mode {
    /// TUN mode
    ///
    /// The packets returned are on the IP layer (layer 3), prefixed with 4-byte header (2 bytes
    /// are flags, 2 bytes are the protocol inside, eg one of
    /// <https://en.wikipedia.org/wiki/EtherType#Examples>.
    Tun = 1,
    /// TAP mode
    ///
    /// The packets are on the transport layer (layer 2), and start with ethernet frame header.
    Tap = 2,
}

pub enum SyncTunMessage {
    Data(Vec<u8>),
    Flush,
    Stop,
    IO_ERROR(std::io::Error),
    RECEIVE_ERROR,
}

#[derive(Debug)]
pub struct TunIpv6Addr {
    pub ip: std::net::Ipv6Addr,
    pub prefix_len: u32,
}
#[derive(Debug)]
pub struct TunIpv4Addr {
    pub ip: Ipv4Addr,

    #[cfg(target_os = "macos")]
    pub destination: Ipv4Addr,
    #[cfg(target_os = "linux")]
    pub subnet_mask: u8,
}

#[derive(Debug)]
pub enum TunIpAddr {
    Ipv4(TunIpv4Addr),
    Ipv6(TunIpv6Addr),
}
// #[derive(Clone)]
pub struct TunDevice {
    #[cfg(target_os = "macos")]
    pub(crate) fd: RawFd,
    #[cfg(target_os = "linux")]
    pub(crate) fd: File,
    pub name: String,
}

#[cfg(target_os = "macos")]
#[cfg(not(feature = "async"))]
fn set_ipv4_address(
    interface: &str,
    ip: &str,
    destination: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .arg("inet")
        .arg(ip)
        .arg(destination)
        .arg("up")
        .output()?;

    if !ifconfig_output.status.success() {
        println!("error : {:?}", ifconfig_output.stderr);
        return Err("Failed to set IP address using ifconfig".into());
    }

    Ok(())
}
#[cfg(target_os = "macos")]
#[cfg(feature = "async")]
async fn set_ipv4_address(
    interface: &str,
    ip: &str,
    destination: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .arg("inet")
        .arg(ip)
        .arg(destination)
        .arg("up")
        .output().await?;

    if !ifconfig_output.status.success() {
        println!("error : {:?}", ifconfig_output.stderr);
        return Err("Failed to set IP address using ifconfig".into());
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[cfg(not(feature = "async"))]
fn set_ipv6_address(
    interface: &str,
    ip: &str,
    prefix_length: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .arg("inet6")
        .arg(ip)
        .arg("prefixlen")
        .arg(prefix_length.to_string())
        .arg("up")
        .output()?;

    if !ifconfig_output.status.success() {
        return Err("Failed to set IPv6 address using ifconfig".into());
    }

    Ok(())
}
#[cfg(target_os = "macos")]
#[cfg(feature = "async")]
async fn set_ipv6_address(
    interface: &str,
    ip: &str,
    prefix_length: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .arg("inet6")
        .arg(ip)
        .arg("prefixlen")
        .arg(prefix_length.to_string())
        .arg("up")
        .output().await?;

    if !ifconfig_output.status.success() {
        return Err("Failed to set IPv6 address using ifconfig".into());
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn set_ipv4_address(
    interface: &str,
    ip: &str,
    netmask: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("sudo")
        .arg("ip")
        .arg("addr")
        .arg("add")
        .arg(format!("{}/{}", ip, netmask))
        .arg("dev")
        .arg(interface)
        .output()?;

    if !ifconfig_output.status.success() {
        println!("error: {:?}", ifconfig_output.stderr);
        return Err("Failed to set IP address using the ip command".into());
    }

    let output = Command::new("ip")
        .arg("link")
        .arg("set")
        .arg(interface)
        .arg("up")
        .output()?;
    if !output.status.success() {
        println!("error: {:?}", ifconfig_output.stderr);
        return Err("Failed to set IP address using the ip command".into());
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn set_ipv6_address(
    interface: &str,
    ip: &str,
    prefix_length: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ifconfig_output = Command::new("sudo")
        .arg("ip")
        .arg("-6")
        .arg("addr")
        .arg("add")
        .arg(format!("{}/{}", ip, prefix_length))
        .arg("dev")
        .arg(interface)
        .output()?;

    if !ifconfig_output.status.success() {
        return Err("Failed to set IPv6 address using the ip command".into());
    }

    Ok(())
}

impl TunDevice {
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self, io::Error> {
        use std::io::{Error, ErrorKind};

        if let Some(num) = get_available_utun() {
            let result = unsafe { tuntap_setup(num) };
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            let name = format!("utun{}", num);
            
            Ok(TunDevice {
                fd: result,
                name: name,
            })
        } else {
            Err(Error::new(ErrorKind::Other, "No available utun"))
        }
    }
    #[cfg(not(feature = "async"))]
    pub fn set_ip_address (&self, ip: &TunIpAddr) -> Result<(), Box<dyn std::error::Error>> {
        match ip {
            TunIpAddr::Ipv4(ip) => {
                set_ipv4_address(&self.name, &ip.ip.to_string(), &ip.destination.to_string())
                    
            }
            TunIpAddr::Ipv6(ip) => {
                set_ipv6_address(&self.name, &ip.ip.to_string(), ip.prefix_len)
            }
        }
    }
    #[cfg(feature = "async")]
    pub async fn set_ip_address (&self, ip: &TunIpAddr) -> Result<(), Box<dyn std::error::Error>> {
        match ip {
            TunIpAddr::Ipv4(ip) => {
                set_ipv4_address(&self.name, &ip.ip.to_string(), &ip.destination.to_string()).await
                    
            }
            TunIpAddr::Ipv6(ip) => {
                set_ipv6_address(&self.name, &ip.ip.to_string(), ip.prefix_len).await
            }
        }
    }


    #[cfg(target_os = "linux")]
    pub fn new() -> Result<Self, io::Error> {
        use std::ffi::CStr;
        use std::fs::OpenOptions;
        use std::io::Error;
        use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;
        // The buffer is larger than needed, but who cares… it is large enough.
        let ifname = "";
        let mut name_buffer = Vec::new();
        name_buffer.extend_from_slice(ifname.as_bytes());
        name_buffer.extend_from_slice(&[0; 33]);
        let name_ptr: *mut u8 = name_buffer.as_mut_ptr();
        let result = unsafe {
            tuntap_setup(
                fd.as_raw_fd(),
                name_ptr,
                Mode::Tun as libc::c_int,
                if false { 1 } else { 0 },
            )
        };
        if result < 0 {
            return Err(Error::last_os_error());
        }
        let name = unsafe {
            CStr::from_ptr(name_ptr as *const libc::c_char)
                .to_string_lossy()
                .into_owned()
        };
        match ip {
            TunIpAddr::Ipv4(ip) => {
                set_ipv4_address(&name, &ip.ip.to_string(), ip.subnet_mask).unwrap()
            }
            TunIpAddr::IpV6(ip) => {
                set_ipv6_address(&name, &ip.ip.to_string(), ip.prefix_len).unwrap()
            }
        };
        Ok(TunDevice { fd, name })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(target_os = "linux")]
    fn read(&self, buf: &mut [u8]) -> Result<usize, io::Error> {
        use std::io::Read;

        (&self.fd).read(buf)
    }
    #[cfg(target_os = "linux")]
    fn write(&self, buf: &[u8]) -> Result<usize, io::Error> {
        use std::io::Write;

        (&self.fd).write(buf)
    }

    #[cfg(target_os = "linux")]
    fn close(&self) {
        //TODO
    }

    #[cfg(target_os = "macos")]
   pub fn read(&self, buf: &mut [u8]) -> Result<usize, io::Error> {
        unsafe {
            let amount = libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len());

            if amount < 0 {
                return Err(io::Error::last_os_error().into());
            }

            Ok(amount as usize)
        }
    }
    #[cfg(target_os = "macos")]
    pub fn close(&self) {
        unsafe {
            libc::close(self.fd);
        }
    }
    #[cfg(target_os = "macos")]
    pub fn write(&self, buf: &[u8]) -> Result<usize, io::Error> {
        unsafe {
            let amount = libc::write(self.fd, buf.as_ptr() as *const _, buf.len());

            if amount < 0 {
                return Err(io::Error::last_os_error().into());
            }

            Ok(amount as usize)
        }
    }
    // #[cfg(not(feature = "async"))]
    // pub fn start(self, rx: Receiver<SyncTunMessage>) -> Receiver<SyncTunMessage> {
    //     let (sender, receiver) = channel();

    //     let b_running = Arc::new(AtomicBool::new(true));
    //     let b_running_check = b_running.clone();
    //     let device = Arc::new(self);
    //     let d_writer = device.clone();
    //     let d_reader = device.clone();

    //     let error_sender = sender.clone();
    //     std::thread::spawn(move || loop {
    //         match rx.recv() {
    //             Ok(message) => match message {
    //                 SyncTunMessage::Data(payload) => {
    //                     d_writer.write(&payload);
    //                 }
    //                 SyncTunMessage::Stop => {
    //                     b_running.store(false, Ordering::SeqCst);
    //                     d_writer.close(); //this will interrupt the read
    //                     break;
    //                 }
    //                 _ => {}
    //             },
    //             Err(e) => {
    //                 error_sender.send(SyncTunMessage::RECEIVE_ERROR(e));
    //             }
    //         }
    //     });

    //     std::thread::spawn(move || {
    //         let mut buf = vec![0u8; 2048];
    //         while b_running_check.load(Ordering::SeqCst) {
    //             match d_reader.read(&mut buf) {
    //                 Ok(size) => {
    //                     sender.send(SyncTunMessage::Data(buf[..size - 1].to_vec()));
    //                 }
    //                 Err(e) => {
    //                     sender.send(SyncTunMessage::IO_ERROR(e));
    //                 }
    //             }
    //         }
    //     });
    //     receiver
    // }
}

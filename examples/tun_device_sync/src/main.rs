use std::net::Ipv6Addr;
use std::{net::Ipv4Addr, sync::mpsc::channel};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use neutils::tun_device::{self, TunInterface, TunIpAddr, TunIpv4Addr, TunIpv6Addr};

pub fn ula_ipv6_addr_from_pan_id_short_addr(
    ula_net_prefix: &[u8],
    ula_host_prefix: &[u8],
    pan_id: u16,
    short_addr: u16,
) -> Option<Ipv6Addr> {
    let mut addr = Vec::with_capacity(16);
    addr.extend_from_slice(ula_net_prefix);
    // addr.extend_from_slice(pan_id.to_be_bytes().as_slice());
    addr.extend_from_slice(ula_host_prefix);
    addr.extend_from_slice(short_addr.to_be_bytes().as_slice());

    if addr.len() == 16 {
        let mut v = [0u8; 16];
        v.copy_from_slice(addr.as_slice());
        Some(Ipv6Addr::from(v))
    } else {
        None
    }
}

fn main() {
    println!("hello");
    let tun1 = TunInterface::new().unwrap();

    let ula = ula_ipv6_addr_from_pan_id_short_addr(&[0xfd, 0x00, 0x00, 0x00, 0x00, 0x02, 0x78, 0x1d], 
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66], 0x781d, 1).unwrap();

    tun_device::set_ip(tun1.name(), &TunIpAddr::Ipv6(TunIpv6Addr { ip: ula, prefix_len: 64 }));
    tun_device::set_mtu(tun1.name(), 1280);
    tun_device::inteface_up(tun1.name());
    // println!("tun : {}", tun1.unwrap().name);
    let mut buf = [0u8; 2048];
    loop {
        match tun1.recv(&mut buf) {
            Ok(size) => {
                let data = &buf[..size];
                println!("Received data : {:?}", data);
            }

            Err(_) => todo!(),
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

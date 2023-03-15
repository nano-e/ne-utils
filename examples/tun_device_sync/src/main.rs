use std::{net::Ipv4Addr, sync::mpsc::channel};

use neutils::tun_device::{TunDevice, TunIpv4Addr};

fn main() {
    println!("hello");
    let tun1 = TunDevice::new(neutils::tun_device::TunIpAddr::Ipv4(TunIpv4Addr {
        ip: Ipv4Addr::new(10, 5, 0, 2),
        #[cfg(target_os = "macos")]
        destination: Ipv4Addr::new(10, 5, 1, 2),
        #[cfg(target_os = "linux")]
        subnet_mask: 16
    }));

    let (tx, rx) = channel();
    let r1 = tun1.unwrap().start(rx);
    // println!("tun : {}", tun1.unwrap().name);
   loop {
    match r1.recv() {
        Ok(data) => {
            match data {
                neutils::tun_device::TunMessage::Data(data) => println!("received {:?}", data),
                neutils::tun_device::TunMessage::Flush => todo!(),
                neutils::tun_device::TunMessage::Stop => todo!(),
                neutils::tun_device::TunMessage::IO_ERROR(_) => todo!(),
                neutils::tun_device::TunMessage::RECEIVE_ERROR(e) => println!("receive error : {:?}", e),
            }        
        }

        Err(_) => todo!(),
    }
   }
}

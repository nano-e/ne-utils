use std::{net::{Ipv4Addr, Ipv6Addr}, sync::{mpsc::channel, Arc}};

use neutils::{tun_device::{TunDevice, TunIpv4Addr, TunIpv6Addr}, async_tun_device::AsyncTunDevice};
use tokio::{time, task};


async fn read_from_device(async_tun_device: Arc<AsyncTunDevice>) {
    let mut buf = vec![0u8; 4096];
    loop {
        let n = async_tun_device.recv(&mut buf).await.unwrap();
        println!("Read {} bytes from the TUN device: {:?}", n, &buf[..n]);
    }
}

async fn write_to_device(async_tun_device: Arc<AsyncTunDevice>) {
    let data_to_send = b"Hello, TUN device!";
    loop {
        async_tun_device.send(data_to_send).await.unwrap();
        println!("Wrote data to the TUN device");
        time::sleep(time::Duration::from_secs(1)).await;
    }
}


#[tokio::main]
async fn main() {
    start_ipv6().await;
    
}
async fn start_ipv4 () {
    let tun_device = TunDevice::new().unwrap();
    tun_device.set_ip_address(&neutils::tun_device::TunIpAddr::Ipv4(TunIpv4Addr {
        ip: Ipv4Addr::new(10, 5, 0, 2),
        #[cfg(target_os = "macos")]
        destination: Ipv4Addr::new(10, 5, 1, 2),
        #[cfg(target_os = "linux")]
        subnet_mask: 16
    }));

    let mut async_tun_device = AsyncTunDevice::new(tun_device).expect("Failed to create AsyncTunDevice");

    
    let async_tun_device = Arc::new(async_tun_device);

    let read_task = task::spawn(read_from_device(async_tun_device.clone()));
    let write_task = task::spawn(write_to_device(async_tun_device.clone()));

    // Wait for the read and write tasks to complete (in this example, they will run forever)
    tokio::select! {
        _ = read_task => {},
        _ = write_task => {},
    }
}

async fn start_ipv6() {
    let tun_device = TunDevice::new()
    .unwrap();

    tun_device.set_ip_address(&neutils::tun_device::TunIpAddr::Ipv6(TunIpv6Addr {
        ip: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1),
        #[cfg(target_os = "macos")]
        destination: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 2),
        prefix_len: 64,
    }));
    let mut async_tun_device = AsyncTunDevice::new(tun_device).expect("Failed to create AsyncTunDevice");

    let async_tun_device = Arc::new(async_tun_device);

    let read_task = task::spawn(read_from_device(async_tun_device.clone()));
    let write_task = task::spawn(write_to_device(async_tun_device.clone()));

    // Wait for the read and write tasks to complete (in this example, they will run forever)
    tokio::select! {
        _ = read_task => {},
        _ = write_task => {},
    }
}


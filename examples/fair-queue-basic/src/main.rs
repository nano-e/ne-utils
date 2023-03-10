use neutils::fair_queue::{Packet, FairQueue};
use rand::{distributions::Alphanumeric, Rng};
use std::{time::{Instant, Duration}, thread::sleep};

fn generate_packets() -> Vec<Packet> {
    let mut rng = rand::thread_rng();

    let destinations = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

    let mut packets = Vec::new();

    for _ in 0..1000 {
        let destination = destinations[rng.gen_range(0..destinations.len())].to_string();
        let data: Vec<u8> = (0..100).map(|_| rng.gen()).collect();
        let timestamp = Instant::now();
        let packet = Packet {
            destination,
            data,
            timestamp,
            dequeue_time: None,
        };
        packets.push(packet);
    }

    packets
}


fn main() {
    let packets = generate_packets();
    let mut fq = FairQueue::new(Duration::from_secs(30), Duration::from_secs(30));
    let mut rng = rand::thread_rng();
    
    for packet in packets {
        fq.enqueue(packet);
    }

    println!("queue size : {}", fq.size());
    let mut counter = 0;
    loop {
        let p = fq.dequeue();
        if p.is_none() {
            break;
        }
        
        let delay_ms = rng.gen_range(10..100);
        sleep(Duration::from_millis(delay_ms));
        if counter % 100 == 0 {
            println!("Latency : {:?} - counter : {}", fq.get_average_latency(), counter);
        }
        counter = counter + 1;
    }
    println!("queue size : {}", fq.size());
}
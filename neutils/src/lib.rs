pub mod fair_queue;




#[cfg(test)]
mod tests {
    use std::time::{Instant, Duration};

    use rand::Rng;

    use crate::fair_queue::{FairQueue, Packet};

    fn generate_packets(num_packets: usize) -> Vec<Packet> {
        let mut rng = rand::thread_rng();
    
        let destinations = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    
        let mut packets = Vec::new();
    
        for _ in 0..num_packets {
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
    #[test]
    fn test() {
        let num_packets = 1000;
        let packets = generate_packets(num_packets);
        let mut fq = FairQueue::new(Duration::from_secs(30));
        let mut rng = rand::thread_rng();
        
        for packet in packets {
            fq.enqueue(packet);
        }
        assert!(fq.size() == num_packets);
        let mut counter = 0usize;
        loop {
            let p = fq.dequeue();
            if p.is_none() {
                break;
            }         
            counter += 1;   
        }
        assert!(counter == num_packets);
        assert!(fq.size() == 0);
    }
}
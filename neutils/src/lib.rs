pub mod fair_queue;
pub mod tun_device;
pub mod circular_buffer;

mod io;
#[cfg(feature = "async_tun")]
pub mod async_tun_device;


#[cfg(test)]
mod tests {
    use std::{time::{Instant, Duration}, thread::sleep};

    use rand::Rng;

    use crate::fair_queue::{FairQueue, Data};

    fn generate_packets(num_packets: usize) -> Vec<Data> {
        let mut rng = rand::thread_rng();
    
        let destinations = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    
        let mut packets = Vec::new();
    
        for _ in 0..num_packets {
            let destination = destinations[rng.gen_range(0..destinations.len())].to_string();
            let data: Vec<u8> = (0..100).map(|_| rng.gen()).collect();
            let timestamp = Instant::now();
            let packet = Data {
                id: destination,
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
        let mut fq = FairQueue::new(Duration::from_secs(30), Duration::from_secs(30));
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

        sleep(Duration::from_secs(32));
        fq.dequeue();
        assert!(fq.get_average_latency().len() == 0);
        assert!(fq.queue_sizes() == (0usize, 0usize, 0usize));
    }
}
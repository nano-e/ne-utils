
use neutils::fair_queue::{FairQueue, Data, HasLen};
use rand::Rng;
use std::time::Instant;
use tokio::{
    select,
    time::{interval, Duration},
};
use tokio_stream::StreamExt;

#[derive(Debug)]
struct MyData(Vec<u8>);

impl HasLen for MyData {
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[tokio::main]
async fn main() {
    let stats_interval = Duration::from_secs(60);
    let idle_duration = Duration::from_secs(10);

    let mut fair_queue = FairQueue::<MyData>::new(stats_interval, idle_duration);

    // Enqueue some initial data
    let data1 = Data {
        id: "A".to_string(),
        data: Box::pin(MyData(vec![1, 2, 3])),
        timestamp: Instant::now(),
        dequeue_time: None,
    };

    let data2 = Data {
        id: "B".to_string(),
        data: Box::pin(MyData(vec![4, 5, 6])),
        timestamp: Instant::now(),
        dequeue_time: None,
    };

    fair_queue.enqueue(data1);
    fair_queue.enqueue(data2);

    let mut interval_tick = interval(Duration::from_secs(1));
    let mut queue_stream = fair_queue;

    loop {
        select! {
            _ = interval_tick.tick() => {
                // Generate random data
                let mut rng = rand::thread_rng();
                let random_id: char = rng.gen_range('A'..='Z');
                let random_data_len = rng.gen_range(1..=5);
                let random_data: Vec<u8> = (0..random_data_len).map(|_| rng.gen()).collect();

                let data = Data {
                    id: random_id.to_string(),
                    data: Box::pin(MyData(random_data)),
                    timestamp: Instant::now(),
                    dequeue_time: None,
                };
                queue_stream.enqueue(data);
                println!("Added random data to the queue.");
            }
            Some(data) = queue_stream.next() => {
                println!("Dequeued data: {:?}", data);
            }
        }
    }
}

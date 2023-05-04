use std::collections::HashMap;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::Poll;
use std::time::{Duration, Instant};

pub trait HasLen {
    fn len(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct Data<T: HasLen> {
    pub id: String,
    pub data: Pin<Box<T>>,
    pub timestamp: Instant,
    pub dequeue_time: Option<Instant>,
}

pub struct FairQueue<T: HasLen> {
    queues: HashMap<String, VecDeque<Data<T>>>,
    deficit_counters: HashMap<String, (usize, Instant)>,
    stats_interval: Duration,
    latency_counters: HashMap<String, VecDeque<(f64, usize, Instant)>>,
    num_items: usize,
    idle_duration: Duration, //Duration to trim empty queues
    idle_run: Option<Instant>, //Last time idle check is run
}

impl<T: HasLen> FairQueue<T> {
    pub fn new(stats_interval: Duration, idle_duration: Duration) -> FairQueue<T> {
        FairQueue {
            queues: HashMap::new(),
            deficit_counters: HashMap::new(),
            stats_interval,
            latency_counters: HashMap::new(),
            num_items: 0,
            idle_duration: idle_duration,
            idle_run: None
        }
    }

    pub fn size(&self) -> usize {
        self.num_items
    }
    pub fn queue_sizes(&self) -> (usize, usize, usize) {
        (self.queues.len(), self.deficit_counters.len(), self.latency_counters.len())
    }
    // Add a new packet to the queue for the given destination
    pub fn enqueue(&mut self, packet: Data<T>) {
        let destination = packet.id.clone();

        if let Some(queue) = self.queues.get_mut(&destination) {
            queue.push_back(packet);
        } else {
            let mut new_queue = VecDeque::new();
            new_queue.push_back(packet);
            self.queues.insert(destination.clone(), new_queue);
            self.deficit_counters
                .insert(destination, (0, Instant::now()));
        }
        self.num_items = self.num_items + 1;
    }

    pub fn dequeue(&mut self) -> Option<Data<T>> {
        let mut result = None;

        if let Some((destination, queue)) = self.get_next_queue() {
            result = queue.pop_front();
        }

        if let Some(ref mut p) = result {
            p.dequeue_time = Some(Instant::now());
            let deficit_counter = self
                .deficit_counters
                .entry(p.id.clone())
                .or_insert((0, Instant::now()));

            let elapsed_time = p.timestamp.elapsed().as_secs_f32().max(1.0);
            let decay_factor = 1.0 / elapsed_time;
            deficit_counter.0 = (deficit_counter.0 as f32 * decay_factor) as usize;
            deficit_counter.0 += p.data.len();
            deficit_counter.1 = p.timestamp;

            let latency = p
                .dequeue_time
                .unwrap()
                .duration_since(p.timestamp)
                .as_millis() as f64;
            if let Some(counter) = self.latency_counters.get_mut(&p.id) {
                counter.push_back((latency, p.data.len(), Instant::now()));
                counter.retain(|&(_, _, timestamp)| timestamp >= Instant::now() - self.stats_interval);
            } else {
                let mut counter = VecDeque::new();
                counter.push_back((latency, p.data.len(), Instant::now()));
                self.latency_counters.insert(p.id.clone(), counter);
            }
            self.num_items = self.num_items - 1;
        }
        if let Some(idle_run) = self.idle_run {
            if Instant::now().duration_since(idle_run) > self.idle_duration {
                self.remove_idle_destinations(self.idle_duration);
                self.idle_run = Some(Instant::now());
            }
        }
        else {
            self.remove_idle_destinations(self.idle_duration);
            self.idle_run = Some(Instant::now());
        }

        result
    }

    fn get_next_queue(&mut self) -> Option<(&String, &mut VecDeque<Data<T>>)> {
        let mut min_deficit = std::usize::MAX;
        let mut next_queue = None;
        for (destination, queue) in &mut self.queues {
            if queue.len() > 0 {
                let deficit_counter = self
                    .deficit_counters
                    .entry(destination.to_owned())
                    .or_insert((0, Instant::now()));
                let elapsed_time = deficit_counter.1.elapsed().as_secs_f32().max(1.0);
                let decay_factor = 1.0 / elapsed_time;
                deficit_counter.0 = (deficit_counter.0 as f32 * decay_factor) as usize;
                if deficit_counter.0 < min_deficit {
                    min_deficit = deficit_counter.0;
                    next_queue = Some((destination, queue));
                }
            }
        }

        next_queue
    }
}
impl<T: HasLen> FairQueue<T> {
    // Remove destinations where the queue is empty and the last packet received is older than the given duration
    pub fn remove_idle_destinations(&mut self, max_idle_time: Duration) {
        let now = Instant::now();
        self.queues.retain(|destination, queue| {
            if queue.is_empty() {
                match self.latency_counters.get(destination) {
                    Some(latency_counter) => {
                        if let Some((_, _, last_received)) = latency_counter.back() {
                            if now.duration_since(*last_received) > max_idle_time {
                                self.deficit_counters.remove(destination);
                                self.latency_counters.remove(destination);
                                return false;
                            }
                        }
                    }
                    None => {}
                }
            }
            true
        });
    }

    pub fn get_average_latency(&self) -> HashMap<String, (f64, usize, u64)> {
        let mut result = HashMap::new();

        for (destination, counter) in &self.latency_counters {
            let mut total_latency = 0.0;
            let mut count = 0u64;
            let mut total_data = 0usize;
            for &(latency, data_len, _) in counter.iter() {
                total_latency += latency;
                count += 1;
                total_data += data_len;
            }
            if count > 0 {
                let avg_latency = total_latency / (count as f64);
                result.insert(destination.clone(), (avg_latency, total_data, count));
            }
        }

        result
    }
}

#[cfg(feature = "async")]
use tokio_stream::Stream;

#[cfg(feature = "async")]
impl<T: HasLen> Stream for FairQueue<T> {
    type Item = Data<T>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(item) = this.dequeue() {
            std::task::Poll::Ready(Some(item))
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

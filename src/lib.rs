use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

// matches the handler closure that will be sent in
type Job = Box<dyn FnOnce() + Send + 'static>;

// holds a thread and awaits for job before spawning thread
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        // on worker creation, creates thread that (on loop) blocks execution until job is received or receiver is disconnected

        let thread = thread::spawn(move || loop {
            // recv blocks execution of thread until something is received
            // Q: why doesn't lock stop the other threads?
            let received = receiver.lock().unwrap().recv();

            match received {
                Ok(job) => {
                    println!("Worker {id} received job, now executing");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected from sender");
                    break;
                }
            }
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

// holds multiple workers and one sender
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

#[derive(Debug)]
pub struct PoolCreationError {
    pub message: String,
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size == 0 {
            Err(PoolCreationError {
                message: "Can't create zero pools".to_string(),
            })
        } else {
            // creates transmitting and receiving channel
            let (sender, receiver) = mpsc::channel();
            // allows for shared receiving memory between threads
            let receiver = Arc::new(Mutex::new(receiver));

            // creates vector of new worker instances, each with a clone
            let mut workers = Vec::with_capacity(size);
            for id in 0..size {
                workers.push(Worker::new(id, Arc::clone(&receiver)));
            }
            Ok(ThreadPool {
                workers,
                sender: Some(sender),
            })
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        // if sender exists, sends job down channel to be received
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // swaps out the sender with None, meaning all receivers return errors
        drop(self.sender.take());
        for worker in &mut self.workers {
            println!("Stopping worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

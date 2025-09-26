use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Simple heartbeat system to detect application freezes
pub struct HeartbeatMonitor {
    last_beat: Arc<Mutex<Instant>>,
    _monitor_thread: thread::JoinHandle<()>,
}

impl HeartbeatMonitor {
    pub fn new() -> Self {
        let last_beat = Arc::new(Mutex::new(Instant::now()));
        let last_beat_clone = Arc::clone(&last_beat);
        
        let monitor_thread = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(5));
                
                let time_since_last_beat = {
                    let last = last_beat_clone.lock().unwrap();
                    last.elapsed()
                };
                
                if time_since_last_beat > Duration::from_secs(10) {
                    println!("⚠️  WARNING: No heartbeat for {:.1} seconds - possible freeze!", time_since_last_beat.as_secs_f64());
                    eprintln!("❄️  FREEZE DETECTED: Application may be frozen or very slow");
                } else {
                    println!("💓 HEARTBEAT: Application responsive ({:.1}s ago)", time_since_last_beat.as_secs_f64());
                }
            }
        });
        
        Self {
            last_beat,
            _monitor_thread: monitor_thread,
        }
    }
    
    /// Update the heartbeat - call this regularly from main thread
    pub fn beat(&self) {
        let mut last_beat = self.last_beat.lock().unwrap();
        *last_beat = Instant::now();
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new()
    }
}

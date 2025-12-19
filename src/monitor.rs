use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use sysinfo::{CpuExt, System, SystemExt, DiskExt, NetworkExt};

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage: Vec<f32>, // Per core
    pub total_cpu_usage: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub timestamp: Instant,
}

pub enum MonitorEvent {
    Stats(SystemStats),
}

pub struct Monitor {
    tx: Sender<MonitorEvent>,
    sys: System,
    target_interval: Duration,
}

impl Monitor {
    pub fn new(tx: Sender<MonitorEvent>) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        Self {
            tx,
            sys,
            target_interval: Duration::from_micros(1000), // 1ms (0.001s)
        }
    }

    pub fn run(mut self) {
        thread::spawn(move || {
            let mut last_tick = Instant::now();
            
            loop {
                let now = Instant::now();
                if now.duration_since(last_tick) >= self.target_interval {
                    // Critical section: Fast refresh
                    // Note: refresh_cpu() is lighter than refresh_all()
                    self.sys.refresh_cpu();
                    self.sys.refresh_memory();
                    
                    // Construct stats
                    let stats = SystemStats {
                        cpu_usage: self.sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect(),
                        total_cpu_usage: self.sys.global_cpu_info().cpu_usage(),
                        ram_used: self.sys.used_memory(),
                        ram_total: self.sys.total_memory(),
                        swap_used: self.sys.used_swap(),
                        swap_total: self.sys.total_swap(),
                        timestamp: now,
                    };

                    // Send to UI (ignore error if receiver is dropped)
                    let _ = self.tx.send(MonitorEvent::Stats(stats));
                    
                    last_tick = now;
                }
                
                // Sleep a tiny bit to prevent 100% CPU usage on the monitor thread itself
                // if the work takes less than 1ms.
                thread::sleep(Duration::from_micros(100));
            }
        });
    }
}

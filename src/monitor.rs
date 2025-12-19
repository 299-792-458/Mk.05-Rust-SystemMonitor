use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use sysinfo::System;

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
                    // Update CPU and Memory
                    self.sys.refresh_cpu_all();
                    self.sys.refresh_memory();
                    
                    let cpus = self.sys.cpus();
                    let cpu_usage: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
                    
                    // Calculate global CPU usage manually if needed
                    let total_cpu_usage: f32 = if !cpu_usage.is_empty() {
                        cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
                    } else {
                        0.0
                    };

                    // Construct stats
                    let stats = SystemStats {
                        cpu_usage,
                        total_cpu_usage,
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
                
                thread::sleep(Duration::from_micros(100));
            }
        });
    }
}
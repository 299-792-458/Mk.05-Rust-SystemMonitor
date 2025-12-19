use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use sysinfo::{System, Networks, Disks, Components};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem: u64,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage: Vec<f32>,
    pub total_cpu_usage: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_speed: u64,
    pub tx_speed: u64,
    pub temperatures: Vec<(String, f32)>,
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<(String, u64, u64)>,
    pub timestamp: Instant,
    // NEW FIELDS
    pub uptime: u64,
    pub load_avg: (f64, f64, f64),
}

pub enum MonitorEvent {
    Stats(SystemStats),
}

pub struct Monitor {
    tx: Sender<MonitorEvent>,
    sys: System,
    networks: Networks,
    disks: Disks,
    components: Components,
    target_interval: Duration,
}

impl Monitor {
    pub fn new(tx: Sender<MonitorEvent>) -> Self {
        let mut sys = System::new_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        sys.refresh_all();
        
        Self {
            tx,
            sys,
            networks,
            disks,
            components,
            target_interval: Duration::from_micros(1000), // 1ms
        }
    }

    pub fn run(mut self) {
        thread::spawn(move || {
            let mut last_fast_tick = Instant::now();
            let mut last_slow_tick = Instant::now();
            
            let mut prev_rx = 0;
            let mut prev_tx = 0;
            let mut last_net_check = Instant::now();

            loop {
                let now = Instant::now();
                
                // 1. FAST LOOP (CPU, RAM)
                if now.duration_since(last_fast_tick) >= self.target_interval {
                    self.sys.refresh_cpu_all();
                    self.sys.refresh_memory();
                    last_fast_tick = now;
                }

                // 2. SLOW LOOP (Processes, Disk, Net, Temp)
                let slow_interval = Duration::from_millis(500);
                if now.duration_since(last_slow_tick) >= slow_interval {
                    self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                    self.networks.refresh(true);
                    self.disks.refresh(true);
                    self.components.refresh(true);
                    last_slow_tick = now;
                }

                // --- DATA AGGREGATION ---
                
                let cpus = self.sys.cpus();
                let cpu_usage: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
                let total_cpu_usage = if !cpu_usage.is_empty() {
                    cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
                } else { 0.0 };

                let time_delta = now.duration_since(last_net_check).as_secs_f64();
                let (mut curr_rx, mut curr_tx) = (0, 0);
                for (_, data) in &self.networks {
                    curr_rx += data.total_received();
                    curr_tx += data.total_transmitted();
                }
                
                let rx_speed = if time_delta > 0.0 { ((curr_rx - prev_rx) as f64 / time_delta) as u64 } else { 0 };
                let tx_speed = if time_delta > 0.0 { ((curr_tx - prev_tx) as f64 / time_delta) as u64 } else { 0 };
                
                if time_delta >= 0.5 {
                    prev_rx = curr_rx;
                    prev_tx = curr_tx;
                    last_net_check = now;
                }

                let mut procs: Vec<ProcessInfo> = self.sys.processes().iter()
                    .map(|(pid, p)| ProcessInfo {
                        pid: pid.as_u32(),
                        name: p.name().to_string_lossy().to_string(),
                        cpu: p.cpu_usage(),
                        mem: p.memory(),
                    })
                    .collect();
                procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
                procs.truncate(50); // Keep more for scrolling

                let disks_info = self.disks.iter().map(|d| {
                    (d.name().to_string_lossy().to_string(), d.total_space() - d.available_space(), d.total_space())
                }).collect();

                let temps = self.components.iter().map(|c| {
                    (c.label().to_string(), c.temperature().unwrap_or(0.0))
                }).collect();
                
                // Load Average
                let load = System::load_average();

                let stats = SystemStats {
                    cpu_usage,
                    total_cpu_usage,
                    ram_used: self.sys.used_memory(),
                    ram_total: self.sys.total_memory(),
                    swap_used: self.sys.used_swap(),
                    swap_total: self.sys.total_swap(),
                    rx_bytes: curr_rx,
                    tx_bytes: curr_tx,
                    rx_speed,
                    tx_speed,
                    temperatures: temps,
                    processes: procs,
                    disks: disks_info,
                    timestamp: now,
                    uptime: System::uptime(),
                    load_avg: (load.one, load.five, load.fifteen),
                };

                let _ = self.tx.send(MonitorEvent::Stats(stats));
                thread::sleep(Duration::from_micros(500)); 
            }
        });
    }
}
use std::collections::VecDeque;
use crate::monitor::{SystemStats, ProcessInfo};

pub struct App {
    pub should_quit: bool,
    // History for Charts (X: Time/Tick, Y: Value)
    pub cpu_history_total: VecDeque<(f64, f64)>, 
    pub ram_history: VecDeque<(f64, f64)>,
    pub net_rx_history: VecDeque<(f64, f64)>,
    pub net_tx_history: VecDeque<(f64, f64)>,
    
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<(String, u64, u64)>,
    pub temps: Vec<(String, f32)>,
    
    pub max_history_len: usize,
    pub last_stats: Option<SystemStats>,
    
    // X-Axis counter for charts
    pub tick_count: f64,
}

impl App {
    pub fn new(max_history: usize) -> Self {
        Self {
            should_quit: false,
            cpu_history_total: VecDeque::with_capacity(max_history),
            ram_history: VecDeque::with_capacity(max_history),
            net_rx_history: VecDeque::with_capacity(max_history),
            net_tx_history: VecDeque::with_capacity(max_history),
            processes: Vec::new(),
            disks: Vec::new(),
            temps: Vec::new(),
            max_history_len: max_history,
            last_stats: None,
            tick_count: 0.0,
        }
    }

    pub fn on_tick(&mut self, stats: SystemStats) {
        self.tick_count += 1.0;

        // CPU Total Chart
        if self.cpu_history_total.len() >= self.max_history_len {
            self.cpu_history_total.pop_front();
        }
        self.cpu_history_total.push_back((self.tick_count, stats.total_cpu_usage as f64));

        // RAM Chart
        if self.ram_history.len() >= self.max_history_len {
            self.ram_history.pop_front();
        }
        let ram_percent = (stats.ram_used as f64 / stats.ram_total as f64) * 100.0;
        self.ram_history.push_back((self.tick_count, ram_percent));

        // Network Chart
        if self.net_rx_history.len() >= self.max_history_len {
            self.net_rx_history.pop_front();
            self.net_tx_history.pop_front();
        }
        // Convert to KB/s for better graph scale potentially, or keep raw bytes
        self.net_rx_history.push_back((self.tick_count, stats.rx_speed as f64));
        self.net_tx_history.push_back((self.tick_count, stats.tx_speed as f64));

        // Update Snapshot Data
        self.processes = stats.processes.clone();
        self.disks = stats.disks.clone();
        self.temps = stats.temperatures.clone();
        self.last_stats = Some(stats);
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' || c == 'Q' {
            self.should_quit = true;
        }
    }
}
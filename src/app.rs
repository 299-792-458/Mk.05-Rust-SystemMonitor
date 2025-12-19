use std::collections::VecDeque;
use std::time::Instant;
use crate::monitor::{SystemStats, ProcessInfo};

pub struct App {
    pub should_quit: bool,
    
    // Charts History
    pub cpu_history_total: VecDeque<(f64, f64)>, 
    pub ram_history: VecDeque<(f64, f64)>,
    pub net_rx_history: VecDeque<(f64, f64)>,
    pub net_tx_history: VecDeque<(f64, f64)>,
    
    // Snapshot Data (Most recent 1ms sample)
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<(String, u64, u64)>,
    pub temps: Vec<(String, f32)>,
    pub last_stats: Option<SystemStats>,

    pub max_history_len: usize,
    
    // Aggregation Logic for 1s updates
    accumulated_stats: Vec<SystemStats>, // Buffer for 1ms stats
    last_chart_update: Instant,
    chart_tick_count: f64,
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
            last_stats: None,
            max_history_len: max_history,
            
            accumulated_stats: Vec::with_capacity(1000),
            last_chart_update: Instant::now(),
            chart_tick_count: 0.0,
        }
    }

    pub fn on_tick(&mut self, stats: SystemStats) {
        // 1. Always update real-time snapshot info (Processes, Sensors should show latest state)
        self.processes = stats.processes.clone();
        self.disks = stats.disks.clone();
        self.temps = stats.temperatures.clone();
        self.last_stats = Some(stats.clone());

        // 2. Accumulate for Chart (1ms resolution data)
        self.accumulated_stats.push(stats);

        // 3. Update Chart every 1.0 second
        if self.last_chart_update.elapsed().as_secs_f64() >= 1.0 {
            self.update_charts();
            self.last_chart_update = Instant::now();
        }
    }

    fn update_charts(&mut self) {
        if self.accumulated_stats.is_empty() {
            return;
        }

        self.chart_tick_count += 1.0;
        let count = self.accumulated_stats.len() as f32;

        // Calculate Averages / Max from the accumulated 1ms samples
        let avg_cpu: f32 = self.accumulated_stats.iter().map(|s| s.total_cpu_usage).sum::<f32>() / count;
        
        let avg_ram_used: u64 = (self.accumulated_stats.iter().map(|s| s.ram_used as f64).sum::<f64>() / count as f64) as u64;
        let total_ram = self.accumulated_stats[0].ram_total; // Assuming constant
        let avg_ram_percent = (avg_ram_used as f64 / total_ram as f64) * 100.0;

        let avg_rx_speed: u64 = (self.accumulated_stats.iter().map(|s| s.rx_speed as f64).sum::<f64>() / count as f64) as u64;
        let avg_tx_speed: u64 = (self.accumulated_stats.iter().map(|s| s.tx_speed as f64).sum::<f64>() / count as f64) as u64;

        // Push to History
        // CPU
        if self.cpu_history_total.len() >= self.max_history_len { self.cpu_history_total.pop_front(); }
        self.cpu_history_total.push_back((self.chart_tick_count, avg_cpu as f64));

        // RAM
        if self.ram_history.len() >= self.max_history_len { self.ram_history.pop_front(); }
        self.ram_history.push_back((self.chart_tick_count, avg_ram_percent));

        // Net
        if self.net_rx_history.len() >= self.max_history_len { 
            self.net_rx_history.pop_front(); 
            self.net_tx_history.pop_front();
        }
        self.net_rx_history.push_back((self.chart_tick_count, avg_rx_speed as f64));
        self.net_tx_history.push_back((self.chart_tick_count, avg_tx_speed as f64));

        // Clear accumulator for next second
        self.accumulated_stats.clear();
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' || c == 'Q' {
            self.should_quit = true;
        }
    }
}

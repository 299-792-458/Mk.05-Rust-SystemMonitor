use std::collections::VecDeque;
use crate::monitor::SystemStats;

pub struct App {
    pub should_quit: bool,
    pub cpu_history: Vec<VecDeque<u64>>, // Core -> History (0-100 integer for simple TUI)
    pub ram_history: VecDeque<u64>,
    pub max_history_len: usize,
    pub last_stats: Option<SystemStats>,
    pub avg_cpu_1s: f32, // Average over 1 second (1000 samples approx)
    cpu_accumulator: VecDeque<f32>,
}

impl App {
    pub fn new(max_history: usize) -> Self {
        Self {
            should_quit: false,
            cpu_history: Vec::new(),
            ram_history: VecDeque::with_capacity(max_history),
            max_history_len: max_history,
            last_stats: None,
            avg_cpu_1s: 0.0,
            cpu_accumulator: VecDeque::with_capacity(1000),
        }
    }

    pub fn on_tick(&mut self, stats: SystemStats) {
        // Initialize cpu history vectors if first run
        if self.cpu_history.is_empty() {
            self.cpu_history = vec![VecDeque::with_capacity(self.max_history_len); stats.cpu_usage.len()];
        }

        // Update CPU History per core
        for (i, usage) in stats.cpu_usage.iter().enumerate() {
            if let Some(core_hist) = self.cpu_history.get_mut(i) {
                if core_hist.len() >= self.max_history_len {
                    core_hist.pop_front();
                }
                core_hist.push_back(*usage as u64);
            }
        }

        // Update RAM History
        if self.ram_history.len() >= self.max_history_len {
            self.ram_history.pop_front();
        }
        // Convert to MB for display or keep raw. Let's keep percentage for sparkline (0-100) relative to total?
        // Sparkline expects u64, usually.
        let ram_percent = (stats.ram_used as f64 / stats.ram_total as f64 * 100.0) as u64;
        self.ram_history.push_back(ram_percent);

        // Calculate Average CPU (Real-time moving average over 1000 samples/1s approx)
        if self.cpu_accumulator.len() >= 1000 {
            self.cpu_accumulator.pop_front();
        }
        self.cpu_accumulator.push_back(stats.total_cpu_usage);
        
        let sum: f32 = self.cpu_accumulator.iter().sum();
        self.avg_cpu_1s = sum / self.cpu_accumulator.len() as f32;

        self.last_stats = Some(stats);
    }

    pub fn on_key(&mut self, c: char) {
        if c == 'q' || c == 'Q' {
            self.should_quit = true;
        }
    }
}

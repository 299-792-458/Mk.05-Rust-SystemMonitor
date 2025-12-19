use std::collections::VecDeque;
use std::time::Instant;
use crate::monitor::{SystemStats, ProcessInfo};

pub struct App {
    pub should_quit: bool,
    
    // Charts History (Global)
    pub cpu_history_total: VecDeque<(f64, f64)>, 
    pub ram_history: VecDeque<(f64, f64)>,
    pub net_rx_history: VecDeque<(f64, f64)>,
    pub net_tx_history: VecDeque<(f64, f64)>,
    
    // HEATMAP DATA: Per-core history [CoreIndex][TimeStep]
    // Storing as u8 (0-100) to save memory
    pub cpu_core_history: Vec<VecDeque<u8>>, 

    // Snapshot Data
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<(String, u64, u64)>,
    pub temps: Vec<(String, f32)>,
    pub last_stats: Option<SystemStats>,

    pub max_history_len: usize,
    
    // Aggregation
    accumulated_stats: Vec<SystemStats>,
    last_chart_update: Instant,
    pub chart_tick_count: f64,

    // Interaction
    pub process_scroll_state: usize, // Selected row index
    pub process_sort_by_cpu: bool,   // Toggle sort mode
}

impl App {
    pub fn new(max_history: usize) -> Self {
        Self {
            should_quit: false,
            cpu_history_total: VecDeque::with_capacity(max_history),
            ram_history: VecDeque::with_capacity(max_history),
            net_rx_history: VecDeque::with_capacity(max_history),
            net_tx_history: VecDeque::with_capacity(max_history),
            cpu_core_history: Vec::new(), // Init dynamically
            processes: Vec::new(),
            disks: Vec::new(),
            temps: Vec::new(),
            last_stats: None,
            max_history_len: max_history,
            
            accumulated_stats: Vec::with_capacity(1000),
            last_chart_update: Instant::now(),
            chart_tick_count: 0.0,

            process_scroll_state: 0,
            process_sort_by_cpu: true,
        }
    }

    pub fn on_tick(&mut self, stats: SystemStats) {
        // 1. Snapshot Update
        self.disks = stats.disks.clone();
        self.temps = stats.temperatures.clone();
        
        // Process Sorting & Selection
        let mut procs = stats.processes.clone();
        if self.process_sort_by_cpu {
            procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            procs.sort_by(|a, b| b.mem.cmp(&a.mem));
        }
        self.processes = procs;
        self.last_stats = Some(stats.clone());

        // 2. Heatmap Update (Every tick or throttled?)
        // Let's update heatmap every tick for "flow" visual, or every chart update?
        // Updating every 1ms is too fast for visual, let's do it with chart update (1s) or faster (e.g. 100ms)?
        // For visual smoothness, 1s is choppy. Let's do it every ~100ms if possible.
        // For now, let's sync with Chart Update (1s) to match the other graphs.
        // OR better: Update accumulation logic.

        self.accumulated_stats.push(stats);

        if self.last_chart_update.elapsed().as_secs_f64() >= 0.1 { // 10 FPS updates for smoother visuals
            self.update_charts();
            self.last_chart_update = Instant::now();
        }
    }

    fn update_charts(&mut self) {
        if self.accumulated_stats.is_empty() { return; }

        self.chart_tick_count += 1.0;
        let count = self.accumulated_stats.len() as f32;

        // Averages
        let avg_cpu: f32 = self.accumulated_stats.iter().map(|s| s.total_cpu_usage).sum::<f32>() / count;
        
        // --- Heatmap Logic ---
        // We need average per core.
        // Assuming all stats have same number of cores.
        if let Some(first) = self.accumulated_stats.first() {
            let core_count = first.cpu_usage.len();
            if self.cpu_core_history.len() != core_count {
                self.cpu_core_history = vec![VecDeque::with_capacity(100); core_count]; // 100 cols wide
            }

            for i in 0..core_count {
                let core_sum: f32 = self.accumulated_stats.iter().map(|s| s.cpu_usage.get(i).cloned().unwrap_or(0.0)).sum();
                let core_avg = core_sum / count;
                
                if self.cpu_core_history[i].len() >= 100 { // Heatmap width
                    self.cpu_core_history[i].pop_front();
                }
                self.cpu_core_history[i].push_back(core_avg as u8);
            }
        }

        // Global Charts
        if self.cpu_history_total.len() >= self.max_history_len { self.cpu_history_total.pop_front(); }
        self.cpu_history_total.push_back((self.chart_tick_count, avg_cpu as f64));
        
        // RAM
        let avg_ram: f64 = self.accumulated_stats.iter().map(|s| s.ram_used as f64).sum::<f64>() / count as f64;
        let total = self.accumulated_stats[0].ram_total as f64;
        if self.ram_history.len() >= self.max_history_len { self.ram_history.pop_front(); }
        self.ram_history.push_back((self.chart_tick_count, (avg_ram / total) * 100.0));

        // Net
        let avg_rx: f64 = self.accumulated_stats.iter().map(|s| s.rx_speed as f64).sum::<f64>() / count as f64;
        let avg_tx: f64 = self.accumulated_stats.iter().map(|s| s.tx_speed as f64).sum::<f64>() / count as f64;
        if self.net_rx_history.len() >= self.max_history_len { self.net_rx_history.pop_front(); self.net_tx_history.pop_front(); }
        self.net_rx_history.push_back((self.chart_tick_count, avg_rx));
        self.net_tx_history.push_back((self.chart_tick_count, avg_tx));

        self.accumulated_stats.clear();
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' | 'Q' => self.should_quit = true,
            'j' | 'J' => { // Down
                if !self.processes.is_empty() {
                    self.process_scroll_state = (self.process_scroll_state + 1).min(self.processes.len() - 1);
                }
            }
            'k' | 'K' => { // Up (or Kill? Let's use K for up and x for kill to be safer, or just K for up context)
                // Vim style navigation
                if self.process_scroll_state > 0 {
                    self.process_scroll_state -= 1;
                }
            }
            's' | 'S' => { // Sort Toggle
                self.process_sort_by_cpu = !self.process_sort_by_cpu;
                self.process_scroll_state = 0; // Reset scroll
            }
            'x' | 'X' => { // Kill Process
                // Real kill logic would go here. For safety in demo, we just print or log.
                // In real app: sys.process(pid).kill();
            }
            _ => {}
        }
    }
    
    // Special handling for arrow keys if they were passed as chars (not happening in main.rs currently)
    // We need to update main.rs to pass KeyCode enum or handle arrows there.
    pub fn on_key_code(&mut self, code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.processes.is_empty() {
                    self.process_scroll_state = (self.process_scroll_state + 1).min(self.processes.len().saturating_sub(1));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.process_scroll_state > 0 {
                    self.process_scroll_state -= 1;
                }
            }
            KeyCode::Char('s') => {
                self.process_sort_by_cpu = !self.process_sort_by_cpu;
                self.process_scroll_state = 0;
            }
            _ => {}
        }
    }
}
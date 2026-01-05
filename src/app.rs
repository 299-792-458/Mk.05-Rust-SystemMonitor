use std::collections::VecDeque;
use ratatui::layout::Rect;
use sysinfo::{Pid, ProcessesToUpdate, System};
use crate::monitor::{ProcessInfo, SystemStats};

pub struct App {
    pub should_quit: bool,
    pub tick_ms: u64,
    
    // Charts History (Global)
    pub cpu_history_total: VecDeque<(f64, f64)>, 
    pub ram_history: VecDeque<(f64, f64)>,
    pub net_rx_history: VecDeque<(f64, f64)>,
    pub net_tx_history: VecDeque<(f64, f64)>,
    pub temp_history: VecDeque<(f64, f64)>, // Max Temp History
    
    // HEATMAP DATA: Per-core history [CoreIndex][TimeStep]
    // Storing as u8 (0-100) to save memory
    pub cpu_core_history: Vec<VecDeque<u8>>, 

    // Snapshot Data
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<(String, u64, u64)>,
    pub temps: Vec<(String, f32)>,
    pub last_stats: Option<SystemStats>,

    pub max_history_len: usize,
    
    pub chart_tick_count: f64,

    // Interaction
    pub process_scroll_state: usize, // Selected row index
    pub process_sort_by_cpu: bool,   // Toggle sort mode
    pub search_query: String,
    pub search_active: bool,
    pub search_bar_area: Option<Rect>,
    pub kill_pending: bool,
}

impl App {
    pub fn new(max_history: usize) -> Self {
        let mut app = Self {
            should_quit: false,
            tick_ms: 2000,
            cpu_history_total: VecDeque::with_capacity(max_history),
            ram_history: VecDeque::with_capacity(max_history),
            net_rx_history: VecDeque::with_capacity(max_history),
            net_tx_history: VecDeque::with_capacity(max_history),
            temp_history: VecDeque::with_capacity(max_history),
            cpu_core_history: Vec::new(), // Init dynamically
            processes: Vec::new(),
            disks: Vec::new(),
            temps: Vec::new(),
            last_stats: None,
            max_history_len: max_history,
            
            chart_tick_count: 0.0,

            process_scroll_state: 0,
            process_sort_by_cpu: true,
            search_query: String::new(),
            search_active: false,
            search_bar_area: None,
            kill_pending: false,
        };
        app.recompute_history_len();
        app
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
        self.clamp_process_selection();
        self.last_stats = Some(stats.clone());

        self.update_charts(&stats);
    }

    fn update_charts(&mut self, stats: &SystemStats) {
        self.chart_tick_count += 1.0;
        let core_count = stats.cpu_usage.len();
        if self.cpu_core_history.len() != core_count {
            self.cpu_core_history = vec![VecDeque::with_capacity(100); core_count]; // 100 cols wide
        }

        for i in 0..core_count {
            if self.cpu_core_history[i].len() >= 100 { // Heatmap width
                self.cpu_core_history[i].pop_front();
            }
            self.cpu_core_history[i].push_back(stats.cpu_usage.get(i).cloned().unwrap_or(0.0) as u8);
        }

        // Global Charts
        if self.cpu_history_total.len() >= self.max_history_len { self.cpu_history_total.pop_front(); }
        self.cpu_history_total.push_back((self.chart_tick_count, stats.total_cpu_usage as f64));
        
        // RAM
        let total = stats.ram_total as f64;
        if self.ram_history.len() >= self.max_history_len { self.ram_history.pop_front(); }
        self.ram_history.push_back((self.chart_tick_count, (stats.ram_used as f64 / total) * 100.0));

        // Net
        if self.net_rx_history.len() >= self.max_history_len { self.net_rx_history.pop_front(); self.net_tx_history.pop_front(); }
        self.net_rx_history.push_back((self.chart_tick_count, stats.rx_speed as f64));
        self.net_tx_history.push_back((self.chart_tick_count, stats.tx_speed as f64));

        // Temp (Max observed in this interval)
        if self.temp_history.len() >= self.max_history_len { self.temp_history.pop_front(); }
        let max_temp = stats.temperatures.iter().map(|(_, t)| *t).fold(0.0_f32, f32::max);
        self.temp_history.push_back((self.chart_tick_count, max_temp as f64));
    }

    // Special handling for arrow keys if they were passed as chars (not happening in main.rs currently)
    // We need to update main.rs to pass KeyCode enum or handle arrows there.
    pub fn on_key_event(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
                self.should_quit = true;
                return;
            }
        }
        if self.search_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.search_active = false;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.process_scroll_state = 0;
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.search_query.push(c);
                        self.process_scroll_state = 0;
                    }
                }
                _ => {}
            }
            return;
        }
        if self.kill_pending {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.kill_selected_process();
                    self.kill_pending = false;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.kill_pending = false;
                }
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !self.processes.is_empty() {
                    self.process_scroll_state = (self.process_scroll_state + 1).min(self.processes.len().saturating_sub(1));
                }
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.process_scroll_state > 0 {
                    self.process_scroll_state -= 1;
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.process_sort_by_cpu = !self.process_sort_by_cpu;
                self.process_scroll_state = 0;
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.process_scroll_state = 0;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_tick_ms(100);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.adjust_tick_ms(-100);
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.kill_pending = true;
            }
            _ => {}
        }
    }

    pub fn on_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            if let Some(area) = self.search_bar_area {
                let inside_x = mouse.column >= area.x && mouse.column < area.x.saturating_add(area.width);
                let inside_y = mouse.row >= area.y && mouse.row < area.y.saturating_add(area.height);
                if inside_x && inside_y {
                    self.search_active = true;
                    return;
                }
            }
            self.search_active = false;
        }
    }

    pub fn set_search_bar_area(&mut self, area: Rect) {
        self.search_bar_area = Some(area);
    }

    pub fn filtered_processes(&self) -> Vec<&ProcessInfo> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            return self.processes.iter().collect();
        }
        self.processes
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query)
            })
            .collect()
    }

    pub fn clamp_process_selection(&mut self) {
        let len = self.filtered_processes().len();
        if len == 0 {
            self.process_scroll_state = 0;
            return;
        }
        if self.process_scroll_state >= len {
            self.process_scroll_state = len - 1;
        }
    }

    fn kill_selected_process(&self) {
        let filtered = self.filtered_processes();
        let Some(proc) = filtered.get(self.process_scroll_state) else { return };
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        if let Some(process) = sys.process(Pid::from_u32(proc.pid)) {
            let _ = process.kill();
        }
    }

    fn adjust_tick_ms(&mut self, delta: i64) {
        const MIN_MS: i64 = 100;
        const MAX_MS: i64 = 10_000;
        let next = (self.tick_ms as i64 + delta).clamp(MIN_MS, MAX_MS);
        self.tick_ms = next as u64;
        self.recompute_history_len();
    }

    fn recompute_history_len(&mut self) {
        // Keep charts dense like the core matrix by using a fixed point count.
        const HISTORY_POINTS: usize = 100;
        self.max_history_len = HISTORY_POINTS;
        self.trim_histories();
    }

    fn trim_histories(&mut self) {
        while self.cpu_history_total.len() > self.max_history_len { self.cpu_history_total.pop_front(); }
        while self.ram_history.len() > self.max_history_len { self.ram_history.pop_front(); }
        while self.net_rx_history.len() > self.max_history_len { self.net_rx_history.pop_front(); }
        while self.net_tx_history.len() > self.max_history_len { self.net_tx_history.pop_front(); }
        while self.temp_history.len() > self.max_history_len { self.temp_history.pop_front(); }
    }
}

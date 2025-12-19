use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Rectangle},
        Axis, Block, Borders, BorderType, Chart, Dataset, Gauge, 
        GraphType, Paragraph, Row, Table, TableState
    },
    Frame,
    symbols,
};
use crate::app::App;

// --- PRO THEME PALETTE ---
const C_BG: Color = Color::Rgb(15, 17, 26);         // Deep Night Blue
const C_PANEL_BG: Color = Color::Rgb(15, 17, 26);
const C_BORDER: Color = Color::Rgb(80, 80, 100);    // Steel Grey
const C_BORDER_FOCUS: Color = Color::Rgb(0, 200, 255);

const C_ACCENT_MAIN: Color = Color::Rgb(0, 255, 255); // Cyan
const C_ACCENT_SEC: Color = Color::Rgb(180, 0, 255);  // Purple
const C_ACCENT_WARN: Color = Color::Rgb(255, 180, 0); // Amber
const C_ACCENT_CRIT: Color = Color::Rgb(255, 50, 80); // Red
const C_TEXT_DIM: Color = Color::Rgb(120, 130, 150);
const C_TEXT_LITE: Color = Color::Rgb(220, 230, 240);

// --- HELPER ---
fn format_speed(bytes: f64) -> String {
    if bytes < 1024.0 { format!("{:.0} B", bytes) }
    else if bytes < 1024.0 * 1024.0 { format!("{:.1} K", bytes / 1024.0) }
    else { format!("{:.1} M", bytes / 1024.0 / 1024.0) }
}

fn block_pro(title: &str, border_color: Color) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(border_color).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(C_PANEL_BG))
}

pub fn draw(f: &mut Frame, app: &App) {
    // Global Background
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), f.area());

    // Main Layout: Header vs Body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Compact Status Bar
            Constraint::Min(0),     // Content
        ].as_ref())
        .split(f.area());

    draw_status_bar(f, app, chunks[0]);
    draw_content_grid(f, app, chunks[1]);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let uptime = if let Some(s) = &app.last_stats { s.uptime } else { 0 };
    let h = uptime / 3600;
    let m = (uptime % 3600) / 60;
    
    let text = Line::from(vec![
        Span::styled(" ⚡ OMNI-MONITOR ", Style::default().fg(C_ACCENT_MAIN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("| HOST: {} | UPTIME: {:02}h {:02}m ", hostname.to_uppercase(), h, m), Style::default().fg(C_TEXT_DIM)),
        Span::styled(" | [Q] Quit [S] Sort", Style::default().fg(C_ACCENT_WARN)),
    ]);
    
    f.render_widget(Paragraph::new(text).alignment(Alignment::Left).style(Style::default().bg(Color::Rgb(10,12,20))), area);
}

fn draw_content_grid(f: &mut Frame, app: &App, area: Rect) {
    // Sidebar (Processes) vs Dashboard
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Left Sidebar (Processes)
            Constraint::Percentage(70), // Right Dashboard
        ].as_ref())
        .split(area);

    draw_sidebar(f, app, main_cols[0]);
    draw_dashboard(f, app, main_cols[1]);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("ACTIVE TASKS", C_BORDER);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Header
    let (cpu_c, mem_c) = if app.process_sort_by_cpu { (C_ACCENT_MAIN, C_TEXT_DIM) } else { (C_TEXT_DIM, C_ACCENT_SEC) };
    let header_cells = vec![
        ratatui::widgets::Cell::from("PID").style(Style::default().fg(C_TEXT_DIM)),
        ratatui::widgets::Cell::from("NAME").style(Style::default().fg(C_TEXT_LITE)),
        ratatui::widgets::Cell::from("CPU").style(Style::default().fg(cpu_c)),
        ratatui::widgets::Cell::from("MEM").style(Style::default().fg(mem_c)),
    ];
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Rows
    let rows = app.processes.iter().take(40).enumerate().map(|(i, p)| {
        let style = if i % 2 == 0 { Style::default().bg(Color::Rgb(20, 22, 35)) } else { Style::default() };
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()).style(Style::default().fg(C_TEXT_DIM)),
            ratatui::widgets::Cell::from(p.name.clone()).style(Style::default().fg(C_TEXT_LITE)),
            ratatui::widgets::Cell::from(format!("{:.1}", p.cpu)).style(Style::default().fg(C_ACCENT_MAIN)),
            ratatui::widgets::Cell::from(format!("{:.0}M", p.mem as f64 / 1024.0 / 1024.0)),
        ];
        Row::new(cells).style(style).height(1)
    });

    let table = Table::new(rows, [
        Constraint::Length(6),
        Constraint::Min(10), // Name flexible
        Constraint::Length(6),
        Constraint::Length(6),
    ]).header(header);

    let mut state = TableState::default();
    state.select(Some(app.process_scroll_state));
    f.render_stateful_widget(table.highlight_style(Style::default().bg(C_BORDER).add_modifier(Modifier::BOLD)), inner, &mut state);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    // 3 Rows:
    // 1. CPU Large Chart (40%)
    // 2. Mem & Net (30%)
    // 3. Heatmap & Info (30%)
    
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ].as_ref())
        .split(area);

    // Row 1: CPU
    draw_cpu_section(f, app, rows[0]);

    // Row 2: Mem + Net
    let row2_cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(rows[1]);
    draw_mem_section(f, app, row2_cols[0]);
    draw_net_section(f, app, row2_cols[1]);

    // Row 3: Heatmap + Sensors/Disk
    let row3_cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(rows[2]);
    draw_heatmap_section(f, app, row3_cols[0]);
    draw_info_section(f, app, row3_cols[1]);
}

fn draw_cpu_section(f: &mut Frame, app: &App, area: Rect) {
    // Title with Load Avg
    let load_str = if let Some(s) = &app.last_stats {
        format!("LOAD: {:.2} {:.2} {:.2}", s.load_avg.0, s.load_avg.1, s.load_avg.2)
    } else { "".to_string() };
    
    let title = format!("CPU ACTIVITY [{}]", load_str);
    let block = block_pro(&title, C_ACCENT_MAIN);
    let inner = block.inner(area);
    f.render_widget(block, area);

    draw_chart(f, &app.cpu_history_total, C_ACCENT_MAIN, inner, 0.0, 100.0);
}

fn draw_mem_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("MEMORY", C_ACCENT_SEC);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(inner);
    
    draw_chart(f, &app.ram_history, C_ACCENT_SEC, chunks[0], 0.0, 100.0);
    
    // Swap Tiny Gauge
    if let Some(stats) = &app.last_stats {
        let ratio = if stats.swap_total > 0 { stats.swap_used as f64 / stats.swap_total as f64 } else { 0.0 };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::DarkGray).bg(C_PANEL_BG))
            .ratio(ratio)
            .label(format!("SWP {:.0}%", ratio * 100.0));
        f.render_widget(gauge, chunks[1]);
    }
}

fn draw_net_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("NETWORK I/O", C_ACCENT_WARN);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rx: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max = rx.iter().chain(tx.iter()).map(|(_,v)| *v).fold(0.0, f64::max).max(1024.0);

    let datasets = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Green)).data(&rx),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Red)).data(&tx),
    ];
    
    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([get_x(&rx).0, get_x(&rx).1]))
        .y_axis(Axis::default().bounds([0.0, max]).labels(vec![Span::raw("0"), Span::raw("MAX")]));
    f.render_widget(chart, inner);
}

fn draw_heatmap_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("CORE MATRIX", C_TEXT_DIM);
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    let core_count = app.cpu_core_history.len();
    if core_count == 0 { return; }

    let canvas = Canvas::default()
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, core_count as f64])
        .paint(|ctx| {
            for (core_idx, history) in app.cpu_core_history.iter().enumerate() {
                for (time_idx, &load) in history.iter().enumerate() {
                    // Gradient: Dark Blue -> Cyan -> Green -> Yellow -> Red
                    let color = match load {
                        0..=5 => Color::Rgb(15, 20, 30),
                        6..=20 => Color::Rgb(0, 50, 100),
                        21..=40 => Color::Rgb(0, 150, 150),
                        41..=60 => Color::Rgb(0, 255, 100),
                        61..=80 => Color::Rgb(200, 200, 0),
                        _ => Color::Rgb(255, 0, 50),
                    };
                    ctx.draw(&Rectangle {
                        x: time_idx as f64,
                        y: (core_count - 1 - core_idx) as f64, 
                        width: 1.1, height: 1.1, color,
                    });
                }
            }
        });
    f.render_widget(canvas, inner);
}

fn draw_info_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("SYSTEM STATUS", C_TEXT_DIM);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    // Temp Chart
    draw_chart(f, &app.temp_history, C_ACCENT_CRIT, chunks[0], 0.0, 100.0);

    // Disk Gauges
    let disk_constraints = vec![Constraint::Length(1); app.disks.len().min(3)];
    let disk_layout = Layout::default().direction(Direction::Vertical).constraints(disk_constraints).split(chunks[1]);
    for (i, (name, used, total)) in app.disks.iter().take(3).enumerate() {
        if i >= disk_layout.len() { break; }
        let ratio = *used as f64 / *total as f64;
        let color = if ratio > 0.8 { C_ACCENT_CRIT } else { C_ACCENT_MAIN };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(C_BG))
            .ratio(ratio)
            .label(format!("{} {:.0}%", name, ratio * 100.0));
        f.render_widget(gauge, disk_layout[i]);
    }
}

fn draw_chart(f: &mut Frame, data: &std::collections::VecDeque<(f64, f64)>, color: Color, area: Rect, min: f64, max: f64) {
    let vec_data: Vec<(f64, f64)> = data.iter().cloned().collect();
    let (x_min, x_max) = get_x(&vec_data);

    let datasets = vec![
        Dataset::default().marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(color)).data(&vec_data),
    ];
    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([x_min, x_max]))
        .y_axis(Axis::default().bounds([min, max]).labels(vec![Span::raw(format!("{:.0}", min)), Span::raw(format!("{:.0}", max))]));
    f.render_widget(chart, area);
}

fn get_x(data: &[(f64, f64)]) -> (f64, f64) {
    let x_min = data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);
    (x_min, x_max)
}

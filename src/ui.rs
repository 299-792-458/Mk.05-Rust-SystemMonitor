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

// --- COLOR PALETTE ---
const C_BG: Color = Color::Rgb(10, 10, 15);
const C_BORDER: Color = Color::Rgb(60, 60, 80);
const C_CPU: Color = Color::Rgb(0, 255, 255);
const C_MEM: Color = Color::Rgb(255, 0, 150);
const C_NET_RX: Color = Color::Rgb(50, 255, 50);
const C_NET_TX: Color = Color::Rgb(255, 50, 50);
const C_TEMP: Color = Color::Rgb(255, 160, 0);
const C_TEXT_DIM: Color = Color::Rgb(100, 100, 100);

// --- HELPER ---
fn format_speed(bytes: f64) -> String {
    if bytes < 1024.0 { format!("{:.0} B", bytes) }
    else if bytes < 1024.0 * 1024.0 { format!("{:.1} K", bytes / 1024.0) }
    else { format!("{:.1} M", bytes / 1024.0 / 1024.0) }
}

fn create_block(title: &str, color: Color) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(color).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(C_BG))
}

pub fn draw(f: &mut Frame, app: &App) {
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Percentage(50), // Row 1
            Constraint::Percentage(50), // Row 2
        ].as_ref())
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_row_1(f, app, chunks[1]);
    draw_row_2(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(C_BORDER));
    f.render_widget(block, area);
    let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    
    let left = Paragraph::new(Line::from(vec![
        Span::styled("OMNI", Style::default().fg(C_CPU).add_modifier(Modifier::BOLD)),
        Span::styled("-MONITOR ", Style::default().fg(C_MEM)),
        Span::styled("v6.0", Style::default().fg(C_TEXT_DIM)),
    ]));
    f.render_widget(left, inner);

    let right = Paragraph::new("Q: Quit | S: Sort | ↑/↓: Select").alignment(Alignment::Right).style(Style::default().fg(C_TEXT_DIM));
    f.render_widget(right, inner);
}

fn draw_row_1(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
        .split(area);

    // Col 1: CPU + Load
    draw_cpu_panel(f, app, cols[0]);
    // Col 2: Memory + Swap
    draw_mem_panel(f, app, cols[1]);
    // Col 3: Network
    draw_net_panel(f, app, cols[2]);
}

fn draw_row_2(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
        .split(area);

    // Col 1: Heatmap
    draw_heatmap_panel(f, app, cols[0]);
    // Col 2: Processes
    draw_process_panel(f, app, cols[1]);
    // Col 3: Info (Disks + Temps + Uptime)
    draw_info_panel(f, app, cols[2]);
}

// --- PANEL DRAWING FUNCTIONS ---

fn draw_cpu_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("CPU", C_CPU);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(inner);
    
    // Chart
    draw_chart_inner(f, &app.cpu_history_total, C_CPU, chunks[0], 0.0, 100.0, true);
    
    // Load Avg
    if let Some(stats) = &app.last_stats {
        let (l1, l5, l15) = stats.load_avg;
        let text = format!("Load: {:.2}  {:.2}  {:.2}", l1, l5, l15);
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center).style(Style::default().fg(C_TEXT_DIM)), chunks[1]);
    }
}

fn draw_mem_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("MEMORY", C_MEM);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(inner);
    
    // RAM Chart
    draw_chart_inner(f, &app.ram_history, C_MEM, chunks[0], 0.0, 100.0, true);
    
    // Swap Gauge
    if let Some(stats) = &app.last_stats {
        let ratio = if stats.swap_total > 0 { stats.swap_used as f64 / stats.swap_total as f64 } else { 0.0 };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Magenta).bg(Color::DarkGray))
            .ratio(ratio)
            .label(format!("Swap: {:.1}%", ratio * 100.0));
        f.render_widget(gauge, chunks[1]);
    }
}

fn draw_net_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("NETWORK", C_NET_RX);
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    let rx: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max = rx.iter().chain(tx.iter()).map(|(_,v)| *v).fold(0.0, f64::max).max(1024.0);

    let datasets = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(C_NET_RX)).data(&rx),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(C_NET_TX)).data(&tx),
    ];
    
    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([get_x_bounds(&rx).0, get_x_bounds(&rx).1]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, max]).labels(vec![Span::raw("0"), Span::raw("MAX")]));
    f.render_widget(chart, inner);
}

fn draw_heatmap_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("CORES", C_CPU);
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
                    let color = match load {
                        0..=20 => Color::Rgb(20, 30, 50),
                        21..=40 => Color::Rgb(0, 100, 200),
                        41..=60 => Color::Rgb(0, 255, 200),
                        61..=80 => Color::Rgb(200, 200, 0),
                        _ => Color::Rgb(255, 50, 50),
                    };
                    ctx.draw(&Rectangle {
                        x: time_idx as f64,
                        y: (core_count - 1 - core_idx) as f64, 
                        width: 1.5,
                        height: 1.5,
                        color,
                    });
                }
            }
        });
    f.render_widget(canvas, inner);
}

fn draw_process_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("PROCESSES", Color::White);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (cpu_col, mem_col) = if app.process_sort_by_cpu { (C_CPU, C_TEXT_DIM) } else { (C_TEXT_DIM, C_MEM) };
    
    let header_cells = vec![
        ratatui::widgets::Cell::from("PID").style(Style::default().fg(C_TEXT_DIM)),
        ratatui::widgets::Cell::from("NAME").style(Style::default().fg(Color::White)),
        ratatui::widgets::Cell::from("CPU").style(Style::default().fg(cpu_col)),
        ratatui::widgets::Cell::from("MEM").style(Style::default().fg(mem_col)),
    ];
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows = app.processes.iter().take(20).enumerate().map(|(i, p)| {
        let style = if i % 2 == 0 { Style::default().bg(Color::Rgb(15, 15, 20)) } else { Style::default() };
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()).style(Style::default().fg(C_TEXT_DIM)),
            ratatui::widgets::Cell::from(p.name.clone()),
            ratatui::widgets::Cell::from(format!("{:.1}", p.cpu)),
            ratatui::widgets::Cell::from(format!("{:.0}M", p.mem as f64 / 1024.0 / 1024.0)),
        ];
        Row::new(cells).style(style).height(1)
    });

    let table = Table::new(rows, [Constraint::Length(6), Constraint::Percentage(40), Constraint::Length(10), Constraint::Length(10)]).header(header);
    
    let mut state = TableState::default();
    state.select(Some(app.process_scroll_state));
    f.render_stateful_widget(table.highlight_style(Style::default().bg(Color::Rgb(40,40,50))), inner, &mut state);
}

fn draw_info_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("SYSTEM INFO", C_TEMP);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(40), Constraint::Percentage(40), Constraint::Percentage(20)]).split(inner);

    // 1. Temp Chart
    draw_chart_inner(f, &app.temp_history, C_TEMP, chunks[0], 0.0, 100.0, false);

    // 2. Disks
    let disk_constraints = vec![Constraint::Length(1); app.disks.len().min(3)];
    let disk_layout = Layout::default().direction(Direction::Vertical).constraints(disk_constraints).split(chunks[1]);
    for (i, (name, used, total)) in app.disks.iter().take(3).enumerate() {
        if i >= disk_layout.len() { break; }
        let ratio = *used as f64 / *total as f64;
        let gauge = Gauge::default().gauge_style(Style::default().fg(C_MEM).bg(C_BG)).ratio(ratio).label(format!("{} {:.0}%", name, ratio * 100.0));
        f.render_widget(gauge, disk_layout[i]);
    }

    // 3. Uptime
    if let Some(stats) = &app.last_stats {
        let up = stats.uptime;
        let h = up / 3600;
        let m = (up % 3600) / 60;
        let s = up % 60;
        let text = format!("Uptime: {:02}:{:02}:{:02}", h, m, s);
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center).style(Style::default().fg(Color::Green)), chunks[2]);
    }
}

fn draw_chart_inner(f: &mut Frame, data: &std::collections::VecDeque<(f64, f64)>, color: Color, area: Rect, min: f64, max: f64, _show_pct: bool) {
    let vec_data: Vec<(f64, f64)> = data.iter().cloned().collect();
    let (x_min, x_max) = get_x_bounds(&vec_data);

    let datasets = vec![
        Dataset::default().marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(color)).data(&vec_data),
    ];
    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([min, max]).labels(vec![Span::raw(format!("{:.0}", min)), Span::raw(format!("{:.0}", max))]));
    f.render_widget(chart, area);
}

fn get_x_bounds(data: &[(f64, f64)]) -> (f64, f64) {
    let x_min = data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);
    (x_min, x_max)
}
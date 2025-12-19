use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Rectangle},
        Axis, Block, Borders, BorderType, Chart, Dataset, Gauge, 
        GraphType, Paragraph, Row, Table, TableState, Tabs
    },
    Frame,
    symbols,
};
use crate::app::App;

// --- COLOR PALETTE (VIBRANT & DARK) ---
const C_BG: Color = Color::Rgb(10, 10, 15);
const C_BLOCK_BG: Color = Color::Reset; // Let transparency handle it or same as BG
const C_BORDER: Color = Color::Rgb(60, 60, 80);
const C_BORDER_ACTIVE: Color = Color::Rgb(0, 255, 255);

const C_CPU: Color = Color::Rgb(0, 255, 255);      // Cyan
const C_MEM: Color = Color::Rgb(255, 0, 150);      // Magenta
const C_NET_RX: Color = Color::Rgb(50, 255, 50);   // Green
const C_NET_TX: Color = Color::Rgb(255, 50, 50);   // Red
const C_TEMP: Color = Color::Rgb(255, 160, 0);     // Orange
const C_TEXT_MAIN: Color = Color::White;
const C_TEXT_DIM: Color = Color::Rgb(100, 100, 100);

// --- HELPER ---
fn format_speed(bytes: f64) -> String {
    if bytes < 1024.0 { format!("{:.0} B", bytes) }
    else if bytes < 1024.0 * 1024.0 { format!("{:.1} K", bytes / 1024.0) }
    else { format!("{:.1} M", bytes / 1024.0 / 1024.0) }
}

fn create_block(title: &str, border_color: Color) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(border_color).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(C_BG))
}

pub fn draw(f: &mut Frame, app: &App) {
    // Background fill
    let bg = Block::default().style(Style::default().bg(C_BG));
    f.render_widget(bg, f.area());

    // Main Vertical Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Percentage(45), // Top Row (CPU/RAM + Net/Temp)
            Constraint::Percentage(55), // Bottom Row (Heatmap + Processes)
        ].as_ref())
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_top_row(f, app, chunks[1]);
    draw_bottom_row(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(C_BORDER));
    
    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("⚡ OMNI-MONITOR", Style::default().fg(C_CPU).add_modifier(Modifier::BOLD)),
        Span::styled(" v5.0", Style::default().fg(C_TEXT_DIM)),
    ]));
    f.render_widget(title, layout[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("Q: Quit | S: Sort | ↑/↓: Select ", Style::default().fg(C_TEXT_DIM)),
    ])).alignment(Alignment::Right);
    f.render_widget(help, layout[1]);
}

fn draw_top_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: CPU & RAM Charts Stacked
    let left_col = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[0]);
    
    draw_chart(f, &app.cpu_history_total, "CPU USAGE", C_CPU, left_col[0], 0.0, 100.0, true);
    draw_chart(f, &app.ram_history, "MEMORY USAGE", C_MEM, left_col[1], 0.0, 100.0, true);

    // Right: Network & Temp Stacked
    let right_col = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[1]);

    // Net
    let rx: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max_net = rx.iter().chain(tx.iter()).map(|(_,v)| *v).fold(0.0, f64::max).max(1024.0);
    draw_dual_chart(f, &rx, &tx, "NETWORK TRAFFIC", C_NET_RX, C_NET_TX, right_col[0], max_net, true);

    // Temp
    draw_chart(f, &app.temp_history, "TEMPERATURE", C_TEMP, right_col[1], 0.0, 100.0, false);
}

fn draw_bottom_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: CPU Heatmap + Disks
    let left_sub = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[0]);
    draw_heatmap(f, app, left_sub[0]);
    draw_disk_gauges(f, app, left_sub[1]);

    // Right: Processes
    draw_processes(f, app, chunks[1]);
}

fn draw_chart(f: &mut Frame, data: &std::collections::VecDeque<(f64, f64)>, title: &str, color: Color, area: Rect, min: f64, max: f64, show_pct: bool) {
    let vec_data: Vec<(f64, f64)> = data.iter().cloned().collect();
    let x_min = vec_data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = vec_data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);

    let current_val = vec_data.last().map(|x| x.1).unwrap_or(0.0);
    let title_full = format!("{} [{:.1}{}]", title, current_val, if show_pct {"%"} else {"C"});

    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color))
            .data(&vec_data),
    ];

    let chart = Chart::new(datasets)
        .block(create_block(&title_full, color))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([min, max]).labels(vec![
            Span::styled(format!("{:.0}", min), Style::default().fg(C_TEXT_DIM)),
            Span::styled(format!("{:.0}", max), Style::default().fg(C_TEXT_DIM)),
        ]));
    
    f.render_widget(chart, area);
}

fn draw_dual_chart(f: &mut Frame, d1: &[(f64, f64)], d2: &[(f64, f64)], title: &str, c1: Color, c2: Color, area: Rect, max: f64, format_bytes: bool) {
    let x_min = d1.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = d1.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);

    let cur1 = d1.last().map(|x| x.1).unwrap_or(0.0);
    let cur2 = d2.last().map(|x| x.1).unwrap_or(0.0);

    let title_full = if format_bytes {
        format!("{} [RX: {}/s | TX: {}/s]", title, format_speed(cur1), format_speed(cur2))
    } else {
        title.to_string()
    };

    let datasets = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(c1)).data(d1),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(c2)).data(d2),
    ];

    let labels = if format_bytes {
        vec![Span::raw("0"), Span::raw("MAX")]
    } else {
         vec![Span::raw("0"), Span::raw(format!("{:.0}", max))]
    };

    let chart = Chart::new(datasets)
        .block(create_block(&title_full, Color::White)) // Neutral title color for dual
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, max]).labels(labels));
    
    f.render_widget(chart, area);
}

fn draw_heatmap(f: &mut Frame, app: &App, area: Rect) {
    let core_count = app.cpu_core_history.len();
    let block = create_block("CORE HEATMAP", C_CPU);
    
    if core_count == 0 { 
        f.render_widget(block, area);
        return; 
    }

    let inner_area = block.inner(area);
    f.render_widget(block, area);

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
                        width: 1.5, // Gapless
                        height: 1.5,
                        color,
                    });
                }
            }
        });
    f.render_widget(canvas, inner_area);
}

fn draw_disk_gauges(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("STORAGE", C_MEM);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let constraints = vec![Constraint::Length(1); app.disks.len().min(4)];
    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

    for (i, (name, used, total)) in app.disks.iter().take(4).enumerate() {
        if i >= chunks.len() { break; }
        let ratio = *used as f64 / *total as f64;
        let label = format!("{} {:.0}%", name, ratio * 100.0);
        
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(C_MEM).bg(C_BG))
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, chunks[i]);
    }
}

fn draw_processes(f: &mut Frame, app: &App, area: Rect) {
    let block = create_block("PROCESSES", C_TEXT_MAIN);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (cpu_col, mem_col) = if app.process_sort_by_cpu { (C_CPU, C_TEXT_DIM) } else { (C_TEXT_DIM, C_MEM) };
    
    let header_cells = vec![
        ratatui::widgets::Cell::from("PID").style(Style::default().fg(C_TEXT_DIM)),
        ratatui::widgets::Cell::from("NAME").style(Style::default().fg(C_TEXT_MAIN).add_modifier(Modifier::BOLD)),
        ratatui::widgets::Cell::from("CPU%").style(Style::default().fg(cpu_col).add_modifier(Modifier::BOLD)),
        ratatui::widgets::Cell::from("MEM").style(Style::default().fg(mem_col).add_modifier(Modifier::BOLD)),
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

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Percentage(40),
        Constraint::Length(15),
        Constraint::Length(15),
    ]).header(header);

    // Render list (using render_stateful_widget if we want selection, or simple render for now)
    // To enable selection, we need to pass table state.
    let mut state = TableState::default();
    state.select(Some(app.process_scroll_state));
    f.render_stateful_widget(table.highlight_style(Style::default().bg(Color::Rgb(40,40,50))), inner, &mut state);
}

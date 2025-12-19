use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Borders, BorderType, Chart, Dataset, GraphType, Paragraph, Row, Table, Gauge},
    Frame,
    symbols,
};
use crate::app::App;

// --- CYBERPUNK THEME COLORS ---
const COLOR_NEON_CYAN: Color = Color::Rgb(0, 245, 255);
const COLOR_NEON_PINK: Color = Color::Rgb(255, 0, 150);
const COLOR_NEON_GREEN: Color = Color::Rgb(57, 255, 20);
const COLOR_DARK_BG: Color = Color::Rgb(10, 10, 15); // Very dark gray/blue
const COLOR_TEXT: Color = Color::Rgb(230, 230, 230);
const COLOR_DIM: Color = Color::Rgb(100, 100, 100);

pub fn draw(f: &mut Frame, app: &App) {
    // Main Container
    let bg_block = Block::default().style(Style::default().bg(Color::Reset)); // Transparent BG
    f.render_widget(bg_block, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Compact Header
            Constraint::Min(0),     // Main Dashboard
            Constraint::Length(1),  // Status Bar
        ].as_ref())
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_dashboard(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" ⚡ SYSTEM", Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("_", Style::default().fg(COLOR_DIM)),
        Span::styled("MONITOR ", Style::default().fg(COLOR_NEON_PINK).add_modifier(Modifier::BOLD)),
        Span::styled("//", Style::default().fg(COLOR_DIM)),
        Span::styled(" v2.0.77 ", Style::default().fg(COLOR_TEXT)),
    ]);
    
    let w = Paragraph::new(title).alignment(Alignment::Left);
    f.render_widget(w, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45), // Top: Graphs
            Constraint::Percentage(55), // Bottom: Details
        ].as_ref())
        .split(area);

    draw_graphs_row(f, app, main_layout[0]);
    draw_details_row(f, app, main_layout[1]);
}

fn draw_graphs_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU Chart
    let cpu_data: Vec<(f64, f64)> = app.cpu_history_total.iter().cloned().collect();
    let x_min = cpu_data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = cpu_data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);

    let cpu_dataset = vec![
        Dataset::default()
            .name(" CPU LOAD ")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(COLOR_NEON_CYAN))
            .data(&cpu_data),
    ];

    let cpu_chart = Chart::new(cpu_dataset)
        .block(Block::default()
            .title(Span::styled(" CPU ACTIVITY ", Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(vec![]).style(Style::default().fg(COLOR_DIM)))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![
            Span::styled("0%", Style::default().fg(COLOR_DIM)),
            Span::styled("100%", Style::default().fg(COLOR_DIM)),
        ]));
    
    f.render_widget(cpu_chart, cols[0]);

    // Memory & Net Chart (Overlay or Split) -> Let's Split Vertically in Right Col
    let sub_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    // RAM Chart
    let ram_data: Vec<(f64, f64)> = app.ram_history.iter().cloned().collect();
    let ram_dataset = vec![
        Dataset::default()
            .name(" RAM ")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(COLOR_NEON_PINK))
            .data(&ram_data),
    ];
    let ram_chart = Chart::new(ram_dataset)
        .block(Block::default()
            .title(Span::styled(" MEMORY MATRIX ", Style::default().fg(COLOR_NEON_PINK).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(vec![])) // Synced X
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![
            Span::styled("0", Style::default().fg(COLOR_DIM)),
            Span::styled("100", Style::default().fg(COLOR_DIM)),
        ]));
    f.render_widget(ram_chart, sub_rows[0]);

    // Network Chart
    let rx_data: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx_data: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    
    let max_val = rx_data.iter().chain(tx_data.iter()).map(|(_, v)| *v).fold(0.0, f64::max).max(1024.0);
    
    let net_datasets = vec![
        Dataset::default().name(" IN ").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(COLOR_NEON_GREEN)).data(&rx_data),
        Dataset::default().name(" OUT ").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Yellow)).data(&tx_data),
    ];
    let net_chart = Chart::new(net_datasets)
        .block(Block::default()
            .title(Span::styled(" NETWORK STREAM ", Style::default().fg(COLOR_NEON_GREEN).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(vec![]))
        .y_axis(Axis::default().bounds([0.0, max_val]).labels(vec![
            Span::styled("0", Style::default().fg(COLOR_DIM)),
            Span::styled("MAX", Style::default().fg(COLOR_DIM)),
        ]));
    f.render_widget(net_chart, sub_rows[1]);
}

fn draw_details_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(65), // Processes
            Constraint::Percentage(35), // Info Panel
        ].as_ref())
        .split(area);

    // Processes Table
    draw_processes(f, app, cols[0]);

    // System Info Panel
    draw_info_panel(f, app, cols[1]);
}

fn draw_processes(f: &mut Frame, app: &App, area: Rect) {
    let header_style = Style::default().fg(Color::Black).bg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD);
    
    let header = Row::new(vec![" PID", " NAME", " CPU%", " MEM(MB)"])
        .style(header_style)
        .height(1);

    let rows = app.processes.iter().enumerate().map(|(i, p)| {
        let color = if i % 2 == 0 { Color::Reset } else { Color::Rgb(20, 20, 30) }; // Zebra striping
        let cells = vec![
            ratatui::widgets::Cell::from(format!(" {:<6}", p.pid)),
            ratatui::widgets::Cell::from(format!(" {:<15}", p.name)),
            ratatui::widgets::Cell::from(format!(" {:>5.1}", p.cpu)),
            ratatui::widgets::Cell::from(format!(" {:>8}", p.mem / 1024 / 1024)),
        ];
        Row::new(cells).style(Style::default().bg(color)).height(1)
    });

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(12),
    ])
    .header(header)
    .block(Block::default()
        .title(Span::styled(" ACTIVE PROCESSES ", Style::default().fg(COLOR_NEON_CYAN)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_DIM)));

    f.render_widget(table, area);
}

fn draw_info_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3), // Disk Bar
        ].as_ref())
        .split(area);

    // Sensors & Info
    let mut lines = vec![];
    lines.push(Line::from(vec![Span::styled("KERNEL: ", Style::default().fg(COLOR_DIM)), Span::raw("Darwin Kernel")])); // Hardcoded for demo/portfolio
    lines.push(Line::from(""));
    
    lines.push(Line::from(Span::styled("SENSORS:", Style::default().fg(COLOR_NEON_PINK).add_modifier(Modifier::BOLD))));
    for (label, temp) in &app.temps {
        lines.push(Line::from(vec![
            Span::styled(format!("  • {}: ", label), Style::default().fg(COLOR_TEXT)),
            Span::styled(format!("{:.1}°C", temp), Style::default().fg(if *temp > 70.0 { Color::Red } else { COLOR_NEON_GREEN })),
        ]));
    }
    if app.temps.is_empty() {
        lines.push(Line::from(Span::styled("  [NO THERMAL SENSORS]", Style::default().fg(COLOR_DIM))));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("STORAGE:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    for (name, used, total) in &app.disks {
         let used_gb = used / 1024 / 1024 / 1024;
         let total_gb = total / 1024 / 1024 / 1024;
         lines.push(Line::from(vec![
            Span::styled(format!("  • {}: ", name), Style::default().fg(COLOR_TEXT)),
            Span::raw(format!("{} / {} GB", used_gb, total_gb)),
        ]));
    }

    let p = Paragraph::new(lines)
        .block(Block::default()
            .title(" SYSTEM STATUS ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_DIM)));
    f.render_widget(p, chunks[0]);

    // Disk Gauge (Overall)
    if let Some((_, used, total)) = app.disks.first() {
         let ratio = *used as f64 / *total as f64;
         let gauge = Gauge::default()
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM).border_style(Style::default().fg(COLOR_DIM)))
            .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
            .ratio(ratio)
            .label(format!("{:.1}%", ratio * 100.0));
         f.render_widget(gauge, chunks[1]);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tick_text = format!(" T-IDX: {:.0} ", app.chart_tick_count);
    let bar = Line::from(vec![
        Span::styled(" COMMANDS: ", Style::default().fg(Color::Black).bg(COLOR_TEXT)),
        Span::styled(" [Q] QUIT ", Style::default().fg(Color::Black).bg(COLOR_NEON_CYAN)),
        Span::raw(" "),
        Span::styled(tick_text, Style::default().fg(COLOR_DIM)),
    ]);
    f.render_widget(Paragraph::new(bar).alignment(Alignment::Right), area);
}
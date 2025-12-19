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

// --- THEME ---
const C_BG: Color = Color::Rgb(15, 15, 20);        // Deep Slate
const C_PANEL_BG: Color = Color::Rgb(20, 20, 25);  // Slightly lighter for panels
const C_ACCENT: Color = Color::Rgb(0, 255, 200);   // Neon Cyan
const C_SUB: Color = Color::Rgb(120, 120, 140);    // Muted Text
const C_HEADER_BG: Color = Color::Rgb(0, 200, 160); // Header BG
const C_HEADER_FG: Color = Color::Black;
const C_CRIT: Color = Color::Rgb(255, 50, 80);

// --- HELPER ---
fn format_speed(bytes: f64) -> String {
    if bytes < 1024.0 { format!("{:.0} B/s", bytes) }
    else if bytes < 1024.0 * 1024.0 { format!("{:.1} KB/s", bytes / 1024.0) }
    else { format!("{:.1} MB/s", bytes / 1024.0 / 1024.0) }
}

pub fn draw(f: &mut Frame, app: &App) {
    // Global Background
    let bg = Block::default().style(Style::default().bg(C_BG));
    f.render_widget(bg, f.area());

    // Main Layout (Padding around the edges)
    let main_area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Big Header
            Constraint::Min(0),     // Content
            Constraint::Length(1),  // Footer
        ].as_ref())
        .split(main_area);

    draw_header(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    let logo_text = vec![
        " ▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄ ▄▄▄ ▄▄▄▄▄▄▄ ",
        " █       █       █       █   █       █",
        " █   ▄   █  ▄ ▄  █    ▄  █   █▄     ▄█",
        " █▄▄▄█▄▄▄█▄▄█▄█▄▄█▄▄▄▄█▄▄█▄▄▄█ █▄▄▄▄█ ",
    ];
    
    let logo_width = logo_text[0].chars().count() as u16;
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(logo_width + 2),
            Constraint::Min(0),
        ].as_ref())
        .split(area);

    // Draw Logo
    let logo = Paragraph::new(logo_text.iter().map(|s| Line::from(Span::styled(*s, Style::default().fg(C_ACCENT)))).collect::<Vec<_>>());
    f.render_widget(logo, layout[0]);

    // Draw System Info / Hostname
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "UNKNOWN".to_string());
    let info_text = vec![
        Line::from(vec![
            Span::styled("SYSTEM MONITORING SUITE", Style::default().fg(C_SUB).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("TARGET: ", Style::default().fg(C_SUB)),
            Span::styled(hostname.to_uppercase(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("STATUS: ", Style::default().fg(C_SUB)),
            Span::styled("ONLINE", Style::default().fg(Color::Green)),
        ]),
    ];
    let info = Paragraph::new(info_text).alignment(Alignment::Right);
    f.render_widget(info, layout[1]);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45), // Visuals (Charts + Heatmap)
            Constraint::Length(1),      // Spacer
            Constraint::Percentage(55), // Data (Processes + Info)
        ].as_ref())
        .split(area);

    draw_visuals_section(f, app, chunks[0]);
    draw_data_section(f, app, chunks[2]);
}

fn draw_visuals_section(f: &mut Frame, app: &App, area: Rect) {
    // No borders, just clean layout
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // CPU
            Constraint::Percentage(33), // Heatmap
            Constraint::Percentage(34), // Net
        ].as_ref())
        .split(area);

    // 1. CPU Trend
    draw_chart_block(f, app.cpu_history_total.iter().cloned().collect(), "CPU LOAD", C_ACCENT, layout[0], 0.0, 100.0);

    // 2. Heatmap (Centerpiece)
    draw_heatmap(f, app, layout[1]);

    // 3. Network Trend
    let rx: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let max = rx.iter().map(|(_,v)| *v).fold(0.0, f64::max).max(1024.0);
    draw_chart_block(f, rx, "NETWORK I/O", Color::Magenta, layout[2], 0.0, max);
}

fn draw_chart_block(f: &mut Frame, data: Vec<(f64, f64)>, title: &str, color: Color, area: Rect, y_min: f64, y_max: f64) {
    let x_min = data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);

    // Header Style
    let header = Block::default()
        .style(Style::default().bg(C_PANEL_BG));
    f.render_widget(header.clone(), area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
    
    // Custom Header
    let title_line = Line::from(vec![
        Span::styled(format!(" {} ", title), Style::default().fg(C_HEADER_FG).bg(C_HEADER_BG).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(title_line), chunks[0]);

    // Chart
    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color))
            .data(&data),
    ];
    
    // Calculate Y Labels
    let y_labels = if y_max > 1000.0 {
        vec![Span::raw("0"), Span::raw(format_speed(y_max))]
    } else {
        vec![Span::raw("0"), Span::raw(format!("{:.0}", y_max))]
    };

    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([y_min, y_max]).labels(y_labels).style(Style::default().fg(C_SUB)));
    
    // Add inner margin for the chart so it doesn't touch edges
    let chart_area = chunks[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    f.render_widget(chart, chart_area);
}

fn draw_heatmap(f: &mut Frame, app: &App, area: Rect) {
    let header = Block::default().style(Style::default().bg(C_PANEL_BG));
    f.render_widget(header.clone(), area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
    
    let title_line = Line::from(vec![
        Span::styled(" CORE HEATMAP ", Style::default().fg(C_HEADER_FG).bg(C_HEADER_BG).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(title_line), chunks[0]);

    let core_count = app.cpu_core_history.len();
    if core_count == 0 { return; }

    let canvas_area = chunks[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 2 });
    let canvas = Canvas::default()
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, core_count as f64])
        .paint(|ctx| {
            for (core_idx, history) in app.cpu_core_history.iter().enumerate() {
                for (time_idx, &load) in history.iter().enumerate() {
                    let color = match load {
                        0..=10 => Color::Rgb(20, 30, 40), // Very dark (idle)
                        11..=30 => Color::Rgb(0, 100, 200),
                        31..=60 => Color::Rgb(0, 200, 200),
                        61..=80 => Color::Rgb(200, 200, 0),
                        _ => Color::Rgb(255, 50, 50),
                    };
                    ctx.draw(&Rectangle {
                        x: time_idx as f64,
                        y: (core_count - 1 - core_idx) as f64, 
                        width: 1.2, // Overlap slightly to remove gaps
                        height: 1.2,
                        color,
                    });
                }
            }
        });
    f.render_widget(canvas, canvas_area);
}

fn draw_data_section(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // 1. Process List
    draw_process_list(f, app, chunks[0]);

    // 2. Info Panel
    draw_info_panel(f, app, chunks[1]);
}

fn draw_process_list(f: &mut Frame, app: &App, area: Rect) {
    // Styled Table
    let bg = Block::default().style(Style::default().bg(C_PANEL_BG));
    f.render_widget(bg, area);
    
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
    
    let (cpu_arrow, mem_arrow) = if app.process_sort_by_cpu { ("▼", " ") } else { (" ", "▼") };
    let header_text = format!(" TOP PROCESSES [CPU{} MEM{}] ", cpu_arrow, mem_arrow);
    
    let title = Line::from(vec![
        Span::styled(header_text, Style::default().fg(C_HEADER_FG).bg(C_HEADER_BG).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Table Content
    let header_cells = ["PID", "NAME", "CPU", "MEM"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().fg(C_SUB).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);
    
    let rows = app.processes.iter().take(15).enumerate().map(|(i, p)| {
        let style = if i % 2 == 0 { Style::default().bg(Color::Rgb(25, 25, 30)) } else { Style::default() };
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()).style(Style::default().fg(C_ACCENT)),
            ratatui::widgets::Cell::from(p.name.clone()),
            ratatui::widgets::Cell::from(format!("{:.1}%", p.cpu)),
            ratatui::widgets::Cell::from(format!("{}M", p.mem / 1024 / 1024)),
        ];
        Row::new(cells).style(style).height(1)
    });
    
    let table_area = chunks[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let table = Table::new(rows, [
            Constraint::Length(6),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .header(header);
        // Removed borders for cleaner look

    f.render_widget(table, table_area);
}

fn draw_info_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    // Temp History
    draw_chart_block(f, app.temp_history.iter().cloned().collect(), "TEMPERATURE", C_CRIT, chunks[0], 0.0, 100.0);

    // Disk Usage
    let bg = Block::default().style(Style::default().bg(C_PANEL_BG));
    f.render_widget(bg, chunks[1]);
    
    let disk_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(chunks[1]);
    let title = Line::from(vec![Span::styled(" STORAGE ", Style::default().fg(C_HEADER_FG).bg(C_HEADER_BG).add_modifier(Modifier::BOLD))]);
    f.render_widget(Paragraph::new(title), disk_chunks[0]);

    let inner = disk_chunks[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let disk_rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(2)]).split(inner);

    for (i, (name, used, total)) in app.disks.iter().take(3).enumerate() {
        if i >= disk_rows.len() { break; }
        
        let ratio = *used as f64 / *total as f64;
        let pct = ratio * 100.0;
        let color = if pct > 85.0 { C_CRIT } else { C_ACCENT };
        
        let label = format!("{} {:.0}%", name, pct);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Rgb(30,30,35)))
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, disk_rows[i]);
    }
}

fn draw_footer(f: &mut Frame, _app: &App, area: Rect) {
    let footer = Paragraph::new(" OMNI // RUST TUI // 2025 ").style(Style::default().fg(C_SUB).bg(C_BG)).alignment(Alignment::Center);
    f.render_widget(footer, area);
}
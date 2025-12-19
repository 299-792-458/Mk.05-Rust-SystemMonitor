use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Axis, BarChart, Block, Borders, BorderType, Chart, Dataset, Gauge, 
        GraphType, Paragraph, Row, Table, Tabs
    },
    Frame,
    symbols,
};
use crate::app::App;

// --- THEME ---
const C_BG: Color = Color::Rgb(15, 15, 20);      // Deep Background
const C_ACCENT: Color = Color::Rgb(0, 255, 200); // Cyan-ish
const C_WARN: Color = Color::Rgb(255, 100, 0);   // Orange
const C_CRIT: Color = Color::Rgb(255, 0, 80);    // Red-Pink
const C_TEXT: Color = Color::Rgb(220, 220, 220);
const C_DIM: Color = Color::Rgb(80, 80, 80);

pub fn draw(f: &mut Frame, app: &App) {
    // Global Background
    let bg = Block::default().style(Style::default().bg(C_BG));
    f.render_widget(bg, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title Bar
            Constraint::Percentage(35), // Top Section: Gauges & Trends
            Constraint::Percentage(25), // Mid Section: CPU Spectrum
            Constraint::Percentage(40), // Bot Section: Processes & Details
        ].as_ref())
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_top_section(f, app, chunks[1]);
    draw_spectrum_section(f, app, chunks[2]);
    draw_bottom_section(f, app, chunks[3]);
}

fn draw_title_bar(f: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" ❖ OMNI-SYSTEM ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("ULTIMATE ", Style::default().fg(C_CRIT).add_modifier(Modifier::BOLD)),
        Span::styled("DASHBOARD v3.0 ", Style::default().fg(C_TEXT)),
    ]);
    f.render_widget(Paragraph::new(title).alignment(Alignment::Center), area);
}

fn draw_top_section(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Real-time Gauges
            Constraint::Percentage(75), // History Charts
        ].as_ref())
        .split(area);

    // 1. Gauges Column
    let gauge_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ].as_ref())
        .split(cols[0]);

    // CPU Gauge
    let cpu_val = app.last_stats.as_ref().map(|s| s.total_cpu_usage).unwrap_or(0.0);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU Load ").borders(Borders::ALL).border_style(Style::default().fg(C_DIM)))
        .gauge_style(Style::default().fg(if cpu_val > 80.0 { C_CRIT } else { C_ACCENT }))
        .percent(cpu_val as u16)
        .label(format!("{:.1}%", cpu_val));
    f.render_widget(cpu_gauge, gauge_chunks[0]);

    // RAM Gauge
    let (ram_used, ram_total) = app.last_stats.as_ref()
        .map(|s| (s.ram_used, s.ram_total))
        .unwrap_or((0, 1));
    let ram_ratio = ram_used as f64 / ram_total as f64;
    let ram_gauge = Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL).border_style(Style::default().fg(C_DIM)))
        .gauge_style(Style::default().fg(C_WARN))
        .ratio(ram_ratio)
        .label(format!("{:.1}%", ram_ratio * 100.0));
    f.render_widget(ram_gauge, gauge_chunks[1]);

    // SWAP Gauge
    let (swap_used, swap_total) = app.last_stats.as_ref()
        .map(|s| (s.swap_used, s.swap_total))
        .unwrap_or((0, 1));
    let swap_ratio = if swap_total > 0 { swap_used as f64 / swap_total as f64 } else { 0.0 };
    let swap_gauge = Gauge::default()
        .block(Block::default().title(" Swap ").borders(Borders::ALL).border_style(Style::default().fg(C_DIM)))
        .gauge_style(Style::default().fg(Color::Magenta))
        .ratio(swap_ratio)
        .label(format!("{:.1}%", swap_ratio * 100.0));
    f.render_widget(swap_gauge, gauge_chunks[2]);

    // 2. Charts Column (Mixed Line Chart)
    // Overlapping CPU and Network for "Cyber" look? No, split is cleaner.
    let chart_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    // CPU Trend
    let cpu_data: Vec<(f64, f64)> = app.cpu_history_total.iter().cloned().collect();
    let x_min = cpu_data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = cpu_data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);
    
    let ds_cpu = vec![
        Dataset::default().name("CPU").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(C_ACCENT)).data(&cpu_data),
    ];
    let chart_cpu = Chart::new(ds_cpu)
        .block(Block::default().title(" CPU History ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Span::raw("0"), Span::raw("100")]));
    f.render_widget(chart_cpu, chart_chunks[0]);

    // Network Trend
    let rx_data: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx_data: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max_net = rx_data.iter().chain(tx_data.iter()).map(|(_, v)| *v).fold(0.0, f64::max).max(1024.0);

    let ds_net = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Green)).data(&rx_data),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Red)).data(&tx_data),
    ];
    let chart_net = Chart::new(ds_net)
        .block(Block::default().title(" Network Flow ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, max_net]).labels(vec![Span::raw("0"), Span::raw("MAX")]));
    f.render_widget(chart_net, chart_chunks[1]);
}

fn draw_spectrum_section(f: &mut Frame, app: &App, area: Rect) {
    // BarChart for Core Loads
    // Need to convert data to correct format
    let mut bar_data: Vec<(&str, u64)> = Vec::new();
    
    // We need static strings for labels to satisfy BarChart lifetime or trick it.
    // Ratatui BarChart usually requires &str.
    // We can pre-allocate labels "0", "1", ... "32" etc.
    // For simplicity, let's limit to 16 cores visual or construct string vector in App if needed.
    // Here we use a trick: format! creates a String, we need &str. 
    // We will just generate blank bars with tooltips or simple numbers if possible.
    // Actually, let's just use empty labels or minimal ones to avoid lifetime hell in this quick logic.
    
    let core_usages = if let Some(stats) = &app.last_stats {
        stats.cpu_usage.clone()
    } else {
        vec![]
    };

    // Prepare labels. Since we are inside draw, we can't easily return references to local Strings.
    // We will use a fixed array of strings for labels "0".."127".
    const ID_STRS: [&str; 32] = ["0","1","2","3","4","5","6","7","8","9","10","11","12","13","14","15","16","17","18","19","20","21","22","23","24","25","26","27","28","29","30","31"];
    
    let data: Vec<(&str, u64)> = core_usages.iter().enumerate().take(32).map(|(i, &usage)| {
        let label = if i < ID_STRS.len() { ID_STRS[i] } else { "?" };
        (label, usage as u64)
    }).collect();

    let barchart = BarChart::default()
        .block(Block::default().title(" CORES SPECTRUM ").borders(Borders::ALL).border_type(BorderType::Thick).border_style(Style::default().fg(C_ACCENT)))
        .data(&data)
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(C_ACCENT))
        .value_style(Style::default().fg(Color::Black).bg(C_ACCENT));
        
    f.render_widget(barchart, area);
}

fn draw_bottom_section(f: &mut Frame, app: &App, area: Rect) {
     let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Processes
    let header_cells = ["PID", "Name", "CPU", "MEM"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().fg(C_BG).bg(C_DIM)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);
    
    let rows = app.processes.iter().take(15).map(|p| {
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()),
            ratatui::widgets::Cell::from(p.name.clone()),
            ratatui::widgets::Cell::from(format!("{:.1}%", p.cpu)),
            ratatui::widgets::Cell::from(format!("{}M", p.mem / 1024 / 1024)),
        ];
        Row::new(cells).style(Style::default().fg(C_TEXT)).height(1)
    });
    
    let table = Table::new(rows, [
            Constraint::Length(6),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default().title(" TASKS ").borders(Borders::ALL).border_style(Style::default().fg(C_DIM)));
    f.render_widget(table, cols[0]);

    // Info Box
    let info_block = Block::default().title(" SENSORS & DISK ").borders(Borders::ALL).border_style(Style::default().fg(C_DIM));
    f.render_widget(info_block, cols[1]);
    
    let info_area = cols[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let info_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(info_area);

    // Disk
    let mut disk_lines = vec![];
    for (name, used, total) in app.disks.iter().take(2) {
        let pct = (*used as f64 / *total as f64) * 100.0;
        disk_lines.push(Line::from(vec![
            Span::styled(format!("HDD {}: ", name), Style::default().fg(C_WARN)),
            Span::raw(format!("{:.1}%", pct)),
        ]));
    }
    f.render_widget(Paragraph::new(disk_lines), info_chunks[0]);

    // Temp
    let mut temp_lines = vec![];
    for (label, temp) in app.temps.iter().take(3) {
        temp_lines.push(Line::from(vec![
            Span::styled(format!("TEMP {}: ", label), Style::default().fg(C_CRIT)),
            Span::raw(format!("{:.0}°C", temp)),
        ]));
    }
    f.render_widget(Paragraph::new(temp_lines), info_chunks[1]);
}

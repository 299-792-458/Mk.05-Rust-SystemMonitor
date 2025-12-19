use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Borders, BorderType, Chart, Dataset, Paragraph, Row, Table},
    Frame,
    symbols,
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Top: CPU & RAM Charts
            Constraint::Percentage(30), // Mid: Net & Info
            Constraint::Percentage(30), // Bot: Processes
        ].as_ref())
        .split(f.area());

    draw_top_row(f, app, chunks[0]);
    draw_mid_row(f, app, chunks[1]);
    draw_bot_row(f, app, chunks[2]);
}

fn draw_top_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    // Data Preparation
    let cpu_data: Vec<(f64, f64)> = app.cpu_history_total.iter().cloned().collect();
    let ram_data: Vec<(f64, f64)> = app.ram_history.iter().cloned().collect();

    // 1. CPU Chart
    let x_min = if cpu_data.is_empty() { 0.0 } else { cpu_data.first().unwrap().0 };
    let x_max = if cpu_data.is_empty() { 10.0 } else { cpu_data.last().unwrap().0 };
    
    let datasets = vec![
        Dataset::default()
            .name("CPU Total %")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .data(&cpu_data),
    ];
    
    let chart = Chart::new(datasets)
        .block(Block::default().title(" CPU Usage ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .x_axis(Axis::default().bounds([x_min, x_max]))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![
            Span::styled("0", Style::default().fg(Color::Gray)),
            Span::styled("100", Style::default().fg(Color::Gray)),
        ]));
    f.render_widget(chart, chunks[0]);

    // 2. RAM Chart
    let datasets_ram = vec![
        Dataset::default()
            .name("RAM %")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Magenta))
            .data(&ram_data),
    ];

    let chart_ram = Chart::new(datasets_ram)
        .block(Block::default().title(" Memory Usage ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .x_axis(Axis::default().bounds([x_min, x_max]))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![
            Span::styled("0", Style::default().fg(Color::Gray)),
            Span::styled("100", Style::default().fg(Color::Gray)),
        ]));
    f.render_widget(chart_ram, chunks[1]);
}

fn draw_mid_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Data Prep
    let rx_data: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx_data: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();

    // 1. Network Chart
    let x_min = if rx_data.is_empty() { 0.0 } else { rx_data.first().unwrap().0 };
    let x_max = if rx_data.is_empty() { 10.0 } else { rx_data.last().unwrap().0 };
    
    let max_rx = rx_data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let max_tx = tx_data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let max_y = max_rx.max(max_tx).max(1024.0); 

    let datasets_net = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).style(Style::default().fg(Color::Green)).data(&rx_data),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).style(Style::default().fg(Color::Red)).data(&tx_data),
    ];
    
    let chart_net = Chart::new(datasets_net)
        .block(Block::default().title(" Network Traffic (B/s) ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .x_axis(Axis::default().bounds([x_min, x_max]))
        .y_axis(Axis::default().bounds([0.0, max_y]).labels(vec![
            Span::raw("0"),
            Span::raw(format!("{:.0}", max_y)),
        ]));
    f.render_widget(chart_net, chunks[0]);

    // 2. Info Panel
    let info_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[1]);
    
    let mut disk_text = vec![];
    for (name, used, total) in &app.disks {
        let usage_pct = (*used as f64 / *total as f64) * 100.0;
        let line = Line::from(vec![
            Span::styled(format!("{}: ", name), Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.1}% ", usage_pct)),
            Span::styled(format!("({}/{} GB)", used/1024/1024/1024, total/1024/1024/1024), Style::default().fg(Color::DarkGray)),
        ]);
        disk_text.push(line);
    }
    let p_disk = Paragraph::new(disk_text).block(Block::default().title(" Disks ").borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(p_disk, info_chunks[0]);

    let mut temp_text = vec![];
    for (label, temp) in &app.temps {
        let line = Line::from(vec![
            Span::styled(format!("{}: ", label), Style::default().fg(Color::Blue)),
            Span::raw(format!("{:.1}°C", temp)),
        ]);
        temp_text.push(line);
    }
    if temp_text.is_empty() {
        temp_text.push(Line::from(Span::raw("No sensors detected (macOS requires privileges/drivers?)")));
    }
    let p_temp = Paragraph::new(temp_text).block(Block::default().title(" Sensors ").borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(p_temp, info_chunks[1]);
}

fn draw_bot_row(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["PID", "Name", "CPU %", "Mem (MB)"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);
    
    let rows = app.processes.iter().map(|p| {
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()),
            ratatui::widgets::Cell::from(p.name.clone()),
            ratatui::widgets::Cell::from(format!("{:.1}", p.cpu)),
            ratatui::widgets::Cell::from(format!("{}", p.mem / 1024 / 1024)),
        ];
        Row::new(cells).height(1)
    });
    
    let table = Table::new(rows, [
            Constraint::Length(8),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default().title(" Top Processes ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
        
    f.render_widget(table, area);
}
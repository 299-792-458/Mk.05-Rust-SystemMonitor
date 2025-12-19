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
const C_BG: Color = Color::Rgb(10, 10, 15);
const C_ACCENT: Color = Color::Rgb(0, 255, 200);
const C_WARN: Color = Color::Rgb(255, 140, 0);
const C_CRIT: Color = Color::Rgb(255, 20, 60);
const C_TEXT: Color = Color::Rgb(220, 220, 220);
const C_DIM: Color = Color::Rgb(60, 60, 70);

pub fn draw(f: &mut Frame, app: &App) {
    let bg = Block::default().style(Style::default().bg(C_BG));
    f.render_widget(bg, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Percentage(30), // Top: Charts
            Constraint::Percentage(25), // Mid: HEATMAP
            Constraint::Percentage(45), // Bot: Processes
        ].as_ref())
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_top_charts(f, app, chunks[1]);
    draw_heatmap(f, app, chunks[2]);
    draw_process_table(f, app, chunks[3]);
}

fn draw_title_bar(f: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" OMNI-MONITOR ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" // SYSTEM V4 ", Style::default().fg(C_DIM)),
    ]);
    f.render_widget(Paragraph::new(title).alignment(Alignment::Left), area);
    
    let help = Line::from(vec![
        Span::styled(" [↑/↓] Select ", Style::default().fg(C_TEXT)),
        Span::styled(" [S] Sort ", Style::default().fg(C_ACCENT)),
        Span::styled(" [Q] Quit ", Style::default().fg(C_CRIT)),
    ]);
    f.render_widget(Paragraph::new(help).alignment(Alignment::Right), area);
}

fn draw_top_charts(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU Chart
    let cpu_data: Vec<(f64, f64)> = app.cpu_history_total.iter().cloned().collect();
    let x_min = cpu_data.first().map(|x| x.0).unwrap_or(0.0);
    let x_max = cpu_data.last().map(|x| x.0).unwrap_or(0.0).max(x_min + 10.0);
    
    let ds_cpu = vec![
        Dataset::default().name("CPU").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(C_ACCENT)).data(&cpu_data),
    ];
    let chart_cpu = Chart::new(ds_cpu)
        .block(Block::default().title(" CPU TREND ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Span::raw("0"), Span::raw("100")]));
    f.render_widget(chart_cpu, chunks[0]);

    // NET Chart
    let rx_data: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx_data: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max_net = rx_data.iter().chain(tx_data.iter()).map(|(_, v)| *v).fold(0.0, f64::max).max(1024.0);

    let ds_net = vec![
        Dataset::default().name("RX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Green)).data(&rx_data),
        Dataset::default().name("TX").marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Style::default().fg(Color::Magenta)).data(&tx_data),
    ];
    let chart_net = Chart::new(ds_net)
        .block(Block::default().title(" I/O STREAM ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(Vec::<Span>::new()))
        .y_axis(Axis::default().bounds([0.0, max_net]).labels(vec![Span::raw("0"), Span::raw("MAX")]));
    f.render_widget(chart_net, chunks[1]);
}

fn draw_heatmap(f: &mut Frame, app: &App, area: Rect) {
    // Canvas based heatmap
    // X axis: Time (0 to 100 steps)
    // Y axis: Core Index (0 to N)
    
    let core_count = app.cpu_core_history.len();
    if core_count == 0 { return; }
    
    let canvas = Canvas::default()
        .block(Block::default().title(" CORE LOAD HEATMAP ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, core_count as f64])
        .paint(|ctx| {
            for (core_idx, history) in app.cpu_core_history.iter().enumerate() {
                for (time_idx, &load) in history.iter().enumerate() {
                    let color = match load {
                        0..=20 => Color::Blue,
                        21..=40 => Color::Cyan,
                        41..=60 => Color::Green,
                        61..=80 => Color::Yellow,
                        _ => Color::Red,
                    };
                    
                    // Draw a rectangle for each data point
                    // Adjust width to fill gaps if needed
                    ctx.draw(&Rectangle {
                        x: time_idx as f64,
                        y: (core_count - 1 - core_idx) as f64, // Invert Y so Core 0 is top
                        width: 1.0,
                        height: 1.0,
                        color,
                    });
                }
            }
        });
        
    f.render_widget(canvas, area);
}

fn draw_process_table(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["PID", "Name", "CPU%", "MEM(MB)"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Style::default().fg(C_BG).bg(C_ACCENT).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);
    
    let rows = app.processes.iter().map(|p| {
        let cells = vec![
            ratatui::widgets::Cell::from(p.pid.to_string()),
            ratatui::widgets::Cell::from(p.name.clone()),
            ratatui::widgets::Cell::from(format!("{:.1}", p.cpu)),
            ratatui::widgets::Cell::from(format!("{}", p.mem / 1024 / 1024)),
        ];
        Row::new(cells).style(Style::default().fg(C_TEXT)).height(1)
    });
    
    let table = Table::new(rows, [
            Constraint::Length(8),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default().title(" PROCESS MANAGER ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_DIM)))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
        
    // Stateful widget render
    let mut state = TableState::default();
    state.select(Some(app.process_scroll_state));
    
    f.render_stateful_widget(table, area, &mut state);
}
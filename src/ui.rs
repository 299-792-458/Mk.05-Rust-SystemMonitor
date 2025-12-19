use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, BorderType, Gauge, Paragraph, Sparkline, Wrap},
    Frame,
};
use crate::app::App;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main Content
            Constraint::Length(3),  // Footer
        ].as_ref())
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header<B: Backend>(f: &mut Frame<B>, _app: &App, area: Rect) {
    let text = Spans::from(vec![
        Span::styled("OMNI-MONITOR", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled("v1.0.0", Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
        Span::styled("System: Darwin (macOS)", Style::default().fg(Color::Magenta)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" System Status ");
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(paragraph, area);
}

fn draw_body<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // CPU
            Constraint::Percentage(50), // Memory
        ].as_ref())
        .split(area);

    draw_cpu(f, app, chunks[0]);
    draw_memory(f, app, chunks[1]);
}

fn draw_cpu<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Gauge
            Constraint::Min(5),    // Sparklines
        ].as_ref())
        .split(area);

    // 1. CPU Gauge (Total Average)
    let label = format!("{:.2}%", app.avg_cpu_1s);
    let gauge = Gauge::default()
        .block(Block::default().title(" CPU Average (1s) ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC))
        .percent(app.avg_cpu_1s as u16)
        .label(label);
    f.render_widget(gauge, chunks[0]);

    // 2. CPU Sparklines (Per Core - showing only first 4 for demo or aggregated)
    // For a portfolio, showing individual core activity is cooler.
    // Let's create a block for Sparkline
    let block = Block::default().title(" CPU Real-time Activity (1ms sampling) ").borders(Borders::ALL).border_type(BorderType::Rounded);
    f.render_widget(block, chunks[1]);

    // Determine how many cores to show based on height
    let inner_area = chunks[1].inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    
    if !app.cpu_history.is_empty() {
        // Show Core 0 as representative or Sum
        let core_0_data: Vec<u64> = app.cpu_history[0].iter().cloned().collect();
        let sparkline = Sparkline::default()
            .block(Block::default().title(" Core 0 Activity "))
            .style(Style::default().fg(Color::Yellow))
            .data(&core_0_data)
            .bar_set(ratatui::symbols::bar::NINE_LEVELS);
        
        f.render_widget(sparkline, inner_area);
    }
}

fn draw_memory<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Gauge
            Constraint::Min(5),    // Sparkline
        ].as_ref())
        .split(area);

    if let Some(stats) = &app.last_stats {
        let percent = (stats.ram_used as f64 / stats.ram_total as f64 * 100.0) as u16;
        let label = format!("{}/{} MB", stats.ram_used / 1024 / 1024, stats.ram_total / 1024 / 1024);

        let gauge = Gauge::default()
            .block(Block::default().title(" Memory Usage ").borders(Borders::ALL).border_type(BorderType::Rounded))
            .gauge_style(Style::default().fg(Color::Magenta).bg(Color::Black))
            .percent(percent)
            .label(label);
        f.render_widget(gauge, chunks[0]);

        let ram_data: Vec<u64> = app.ram_history.iter().cloned().collect();
        let sparkline = Sparkline::default()
            .block(Block::default().title(" Memory Trend ").borders(Borders::ALL).border_type(BorderType::Rounded))
            .style(Style::default().fg(Color::Magenta))
            .data(&ram_data)
            .bar_set(ratatui::symbols::bar::NINE_LEVELS);
        f.render_widget(sparkline, chunks[1]);
    }
}

fn draw_footer<B: Backend>(f: &mut Frame<B>, _app: &App, area: Rect) {
    let text = Spans::from(vec![
        Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" to Quit | "),
        Span::styled("Sampling: 1ms", Style::default().fg(Color::Green)),
    ]);
    let p = Paragraph::new(text).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(p, area);
}

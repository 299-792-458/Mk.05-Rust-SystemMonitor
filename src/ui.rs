use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Rectangle},
        Block, Borders, BorderType, Paragraph, Row, Table, TableState
    },
    Frame,
};
use crate::app::App;

// --- PRO THEME PALETTE ---
const C_BG: Color = Color::Rgb(15, 17, 26);         // Deep Night Blue
const C_PANEL_BG: Color = Color::Rgb(15, 17, 26);
const C_BORDER: Color = Color::Rgb(80, 80, 100);    // Steel Grey

const C_ACCENT_MAIN: Color = Color::Rgb(0, 255, 255); // Cyan
const C_ACCENT_SEC: Color = Color::Rgb(180, 0, 255);  // Purple
const C_ACCENT_WARN: Color = Color::Rgb(255, 180, 0); // Amber
const C_ACCENT_CRIT: Color = Color::Rgb(255, 50, 80); // Red
const C_TEXT_DIM: Color = Color::Rgb(120, 130, 150);
const C_TEXT_LITE: Color = Color::Rgb(220, 230, 240);

fn block_pro(title: &str, border_color: Color) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(border_color).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(C_PANEL_BG))
}

pub fn draw(f: &mut Frame, app: &mut App) {
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
    
    let mut spans = vec![
        Span::styled(" OMNI-MONITOR ", Style::default().fg(C_ACCENT_MAIN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("| HOST: {} | UPTIME: {:02}h {:02}m ", hostname.to_uppercase(), h, m), Style::default().fg(C_TEXT_DIM)),
        Span::styled(" | [/] Search [X] Kill [S] Sort [+/-] Tick", Style::default().fg(C_ACCENT_WARN)),
    ];
    if app.kill_pending {
        spans.push(Span::styled(" | CONFIRM KILL? [Y/N]", Style::default().fg(C_ACCENT_CRIT).add_modifier(Modifier::BOLD)));
    }
    let text = Line::from(spans);
    let tick_label = format!("TICK {}ms", app.tick_ms);
    let tick_text = Line::from(Span::styled(tick_label, Style::default().fg(C_TEXT_LITE)));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(14)].as_ref())
        .split(area);

    let bar_style = Style::default().bg(Color::Rgb(10,12,20));
    f.render_widget(Paragraph::new(text).alignment(Alignment::Left).style(bar_style), cols[0]);
    f.render_widget(Paragraph::new(tick_text).alignment(Alignment::Right).style(bar_style), cols[1]);
}

fn draw_content_grid(f: &mut Frame, app: &mut App, area: Rect) {
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

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let block = block_pro("ACTIVE TASKS", C_BORDER);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let sidebar_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(inner);
    let search_area = sidebar_rows[0];
    app.set_search_bar_area(search_area);

    let search_title = if app.search_active { "SEARCH (TYPE)" } else { "SEARCH" };
    let search_text = if app.search_query.is_empty() {
        "Type to filter by name or PID"
    } else {
        app.search_query.as_str()
    };
    let search_style = if app.search_active {
        Style::default().fg(C_ACCENT_MAIN)
    } else {
        Style::default().fg(C_TEXT_DIM)
    };
    let search_box = Paragraph::new(search_text)
        .block(Block::default().borders(Borders::ALL).border_style(search_style).title(search_title))
        .style(Style::default().fg(C_TEXT_LITE));
    f.render_widget(search_box, search_area);

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
    let filtered = app.filtered_processes();
    let rows = filtered.iter().take(40).enumerate().map(|(i, p)| {
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
    let selected = if filtered.is_empty() { None } else { Some(app.process_scroll_state) };
    state.select(selected);
    f.render_stateful_widget(
        table.row_highlight_style(Style::default().bg(C_BORDER).add_modifier(Modifier::BOLD)),
        sidebar_rows[1],
        &mut state,
    );
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

    draw_chart_dots(f, &app.cpu_history_total, C_ACCENT_MAIN, inner, 0.0, 100.0, app.max_history_len);
}

fn draw_mem_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("MEMORY", C_ACCENT_SEC);
    let inner = block.inner(area);
    f.render_widget(block, area);
    draw_chart_dots(f, &app.ram_history, C_ACCENT_SEC, inner, 0.0, 100.0, app.max_history_len);
}

fn draw_net_section(f: &mut Frame, app: &App, area: Rect) {
    let block = block_pro("NETWORK I/O", C_ACCENT_WARN);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rx: Vec<(f64, f64)> = app.net_rx_history.iter().cloned().collect();
    let tx: Vec<(f64, f64)> = app.net_tx_history.iter().cloned().collect();
    let max = app
        .last_stats
        .as_ref()
        .map(|s| s.net_max_bps as f64)
        .unwrap_or(1.0)
        .max(1.0);

    draw_dot_canvas_dual_centered(
        f,
        &rx,
        &tx,
        inner,
        max,
        Color::Green,
        Color::Red,
        app.max_history_len,
    );
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Temp Chart
    let temp_block = block_pro("TEMPERATURE", C_ACCENT_CRIT);
    let temp_inner = temp_block.inner(chunks[0]);
    f.render_widget(temp_block, chunks[0]);
    draw_chart_dots(f, &app.temp_history, C_ACCENT_CRIT, temp_inner, 0.0, 100.0, app.max_history_len);

    // Disk Usage Chart
    let disk_block = block_pro("DISK USAGE", C_ACCENT_MAIN);
    let disk_inner = disk_block.inner(chunks[1]);
    f.render_widget(disk_block, chunks[1]);
    draw_disk_usage_dots(f, &app.disks, disk_inner);
}

fn draw_chart_dots(
    f: &mut Frame,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: Color,
    area: Rect,
    min: f64,
    max: f64,
    max_points: usize,
) {
    let vec_data: Vec<(f64, f64)> = data.iter().cloned().collect();
    draw_dot_canvas_single(f, &vec_data, area, min, max, color, max_points);
}

fn draw_dot_canvas_single(
    f: &mut Frame,
    data: &[(f64, f64)],
    area: Rect,
    min: f64,
    max: f64,
    color: Color,
    max_points: usize,
) {
    const DOT_STEP: f64 = 4.0;
    let max_points_f = max_points.max(1) as f64;
    let len = data.len() as f64;
    let step = 1.0 / max_points_f;
    let start_x = (max_points_f - len).max(0.0);
    let canvas = Canvas::default()
        .x_bounds([0.0, 1.0])
        .y_bounds([min, max.max(1.0)])
        .paint(|ctx| {
            for (i, (_, v)) in data.iter().enumerate() {
                let x = (start_x + i as f64) * step + (step * 0.5);
                let ratio = ((*v - min) / (max - min).max(1.0)).clamp(0.0, 1.0);
                let shade = scale_color(color, ratio);
                let mut y = min;
                while y <= *v {
                    ctx.draw(&Rectangle {
                        x,
                        y,
                        width: (step * 0.6).max(0.01),
                        height: 1.0,
                        color: shade,
                    });
                    y += DOT_STEP;
                }
            }
        });
    f.render_widget(canvas, area);
}

fn draw_dot_canvas_dual_centered(
    f: &mut Frame,
    left: &[(f64, f64)],
    right: &[(f64, f64)],
    area: Rect,
    max: f64,
    left_color: Color,
    right_color: Color,
    max_points: usize,
) {
    const DOT_STEP: f64 = 4.0;
    let len = left.len().max(right.len()) as f64;
    let max_points_f = max_points.max(1) as f64;
    let step = 1.0 / max_points_f;
    let start_x = (max_points_f - len).max(0.0);
    let canvas = Canvas::default()
        .x_bounds([0.0, 1.0])
        .y_bounds([-100.0, 100.0])
        .paint(|ctx| {
            for (i, (_, v)) in left.iter().enumerate() {
                let x = (start_x + i as f64) * step + (step * 0.35);
                let v = v.clamp(0.0, max);
                let ratio = (v / max.max(1.0)).clamp(0.0, 1.0);
                let v_norm = ratio * 100.0;
                let shade = scale_color(left_color, ratio);
                let bottom = -v_norm;
                let mut y = 0.0;
                while y >= bottom {
                    ctx.draw(&Rectangle {
                        x,
                        y,
                        width: (step * 0.3).max(0.01),
                        height: 1.0,
                        color: shade,
                    });
                    y -= DOT_STEP;
                }
            }
            for (i, (_, v)) in right.iter().enumerate() {
                let x = (start_x + i as f64) * step + (step * 0.7);
                let v = v.clamp(0.0, max);
                let ratio = (v / max.max(1.0)).clamp(0.0, 1.0);
                let v_norm = ratio * 100.0;
                let shade = scale_color(right_color, ratio);
                let top = v_norm;
                let mut y = 0.0;
                while y <= top {
                    ctx.draw(&Rectangle {
                        x,
                        y,
                        width: (step * 0.3).max(0.01),
                        height: 1.0,
                        color: shade,
                    });
                    y += DOT_STEP;
                }
            }
        });
    f.render_widget(canvas, area);
}

fn draw_disk_usage_dots(f: &mut Frame, disks: &[(String, u64, u64)], area: Rect) {
    const DOT_STEP: f64 = 4.0;
    let count = disks.len().min(3).max(1) as f64;
    let step = 1.0 / count;
    let canvas = Canvas::default()
        .x_bounds([0.0, 1.0])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            for (i, (_name, used, total)) in disks.iter().take(3).enumerate() {
                let ratio = if *total > 0 { *used as f64 / *total as f64 } else { 0.0 };
                let pct = (ratio * 100.0).clamp(0.0, 100.0);
                let base_color = if ratio > 0.8 { C_ACCENT_CRIT } else { C_ACCENT_MAIN };
                let shade = scale_color(base_color, ratio);
                let x = (i as f64 + 0.5) * step;
                let mut y = 0.0;
                while y <= pct {
                    ctx.draw(&Rectangle {
                        x,
                        y,
                        width: (step * 0.4).max(0.02),
                        height: 1.0,
                        color: shade,
                    });
                    y += DOT_STEP;
                }
            }
        });
    f.render_widget(canvas, area);
}

fn scale_color(color: Color, ratio: f64) -> Color {
    let ratio = ratio.clamp(0.35, 1.0);
    match color {
        Color::Rgb(r, g, b) => {
            let rf = (r as f64 * ratio).min(255.0) as u8;
            let gf = (g as f64 * ratio).min(255.0) as u8;
            let bf = (b as f64 * ratio).min(255.0) as u8;
            Color::Rgb(rf, gf, bf)
        }
        _ => color,
    }
}

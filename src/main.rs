mod app;
mod monitor;
mod ui;

use app::App;
use monitor::{Monitor, MonitorEvent};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::{Duration, Instant}};
use crossbeam_channel::unbounded;

fn main() -> Result<()> {
    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Setup App & Monitor
    // History length for sparklines (e.g., last 200 ticks)
    let app = App::new(200); 
    let (tx, rx) = unbounded();
    
    // Start Monitor Thread
    let monitor = Monitor::new(tx);
    monitor.run();

    // 3. Run Event Loop
    let res = run_app(&mut terminal, app, rx);

    // 4. Restore Terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    rx: crossbeam_channel::Receiver<MonitorEvent>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(30); // ~30 FPS UI refresh rate
    let mut last_tick = Instant::now();

    loop {
        // 1. Draw UI
        terminal.draw(|f| ui::draw(f, &app))?;

        // 2. Handle Input (with timeout for tick rate)
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
            
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char(c) = key.code {
                    app.on_key(c);
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        // 3. Process All Pending Data Events
        // The monitor thread sends data every 1ms. The UI draws every 30ms.
        // We must process ALL pending messages to keep the history accurate without lag.
        while let Ok(msg) = rx.try_recv() {
            match msg {
                MonitorEvent::Stats(stats) => {
                    app.on_tick(stats);
                }
            }
        }
        
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
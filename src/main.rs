mod app;

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new();
    let (tx, rx) = mpsc::channel::<Event>();
    let tx_handle = tx.clone();

    thread::spawn(move || {
        loop {
            if let Ok(event) = event::read()
                && tx_handle.send(event).is_err()
            {
                break;
            }
        }
    });

    while !app.should_quit {
        terminal.draw(|frame| app.render(frame))?;

        if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
            handle_event(&mut app, event)?;
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn handle_event(app: &mut App, event: Event) -> Result<()> {
    if let Event::Key(key) = event {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                app.should_quit = true;
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                app.previous();
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                app.next();
            }
            (KeyCode::Tab, _) => {
                app.next_tab();
            }
            (KeyCode::BackTab, _) | (KeyCode::Char('t'), KeyModifiers::SHIFT) => {
                app.previous_tab();
            }
            (KeyCode::PageDown, _) => {
                app.scroll_down();
            }
            (KeyCode::PageUp, _) => {
                app.scroll_up();
            }
            (KeyCode::Char(' '), _) if app.current_tab == 1 => {
                if let Err(e) = app.toggle_stage() {
                    eprintln!("Stage error: {e}");
                }
            }
            (KeyCode::Char('c'), _) if app.current_tab == 3 && app.focus == app::Focus::List => {
                if let Some(branch) = app.branches.get(app.selected_branch) {
                    let name = branch.name.clone();
                    if !branch.is_current
                        && let Err(e) = app.checkout_branch(&name)
                    {
                        eprintln!("Checkout error: {e}");
                    }
                }
            }
            (KeyCode::Enter, _) => {
                app.toggle_focus();
            }
            _ => {}
        }
    }
    Ok(())
}

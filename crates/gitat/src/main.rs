use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use tokio::sync::mpsc;

use gitat_core::runner::ProcessRunner;
use gitat_ui::app::{App, Mode, Tab};
use gitat_ui::event::handle_key;
use gitat_ui::theme::Theme;
use gitat_ui::util::centered_rect;
use gitat_ui::views;
use gitat_ui::views::commit::render_commit_popup;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let repo_path = std::env::current_dir().context("failed to get current directory")?;
    let runner = ProcessRunner::new(repo_path.clone());

    let mut terminal = ratatui::init();
    let mut app = App::new();
    app.refresh(&runner);
    app.log_list_state.select(Some(0));
    gitat_ui::event::load_log_preview(&mut app, &runner);

    let result = run_app(&mut terminal, &mut app, &runner, &repo_path).await;

    ratatui::restore();
    result
}

async fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    runner: &ProcessRunner,
    repo_path: &Path,
) -> Result<()> {
    // Spawn blocking task for crossterm key input
    let (key_tx, mut key_rx) = mpsc::unbounded_channel();
    let quit_flag = Arc::new(AtomicBool::new(false));
    let quit_flag_clone = quit_flag.clone();
    tokio::task::spawn_blocking(move || {
        loop {
            if quit_flag_clone.load(Ordering::Relaxed) {
                break;
            }
            if event::poll(Duration::from_millis(100)).unwrap_or(false)
                && let Ok(ev) = event::read()
                && key_tx.send(ev).is_err()
            {
                break;
            }
        }
    });

    // Setup filesystem watcher
    let (mut fs_rx, _debouncer) = setup_watcher(repo_path)?;

    loop {
        app.status_bar.clear_if_expired();
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // tabs
                    Constraint::Min(0),    // main content
                    Constraint::Length(1), // status bar
                ])
                .split(f.area());

            // Tab bar
            let tab_titles: Vec<Line> = [Tab::Log, Tab::Branches, Tab::Stash]
                .iter()
                .map(|t| {
                    let style = if *t == app.tab {
                        Theme::tab_active()
                    } else {
                        Theme::tab_inactive()
                    };
                    Line::from(Span::styled(t.title(), style))
                })
                .collect();

            let tabs = Tabs::new(tab_titles)
                .select(match app.tab {
                    Tab::Log => 0,
                    Tab::Branches => 1,
                    Tab::Stash => 2,
                })
                .highlight_style(Theme::tab_active());
            f.render_widget(tabs, chunks[0]);

            // Main content
            views::render_tab(f, app, chunks[1]);

            // Status bar
            let status_text = if let Some(ref msg) = app.status_bar.message {
                msg.clone()
            } else {
                "j/k: move  h/l: panel  Tab: switch  s: stage  c: commit  p: push  ?: help  q: quit"
                    .to_string()
            };
            let status_bar = Paragraph::new(status_text).style(Theme::status_bar());
            f.render_widget(status_bar, chunks[2]);

            // Conflict editor overlay
            if matches!(app.mode, Mode::Conflict { .. })
                && let (Some(file), Some(state)) = (&app.conflict.file, &mut app.conflict.editor_state)
            {
                let editor = gitat_ui::widgets::conflict_editor::ConflictEditor::new(file);
                f.render_stateful_widget(editor, chunks[1], state);
            }

            // Popups
            if matches!(app.mode, Mode::Commit { .. }) {
                render_commit_popup(f, app);
            }

            if matches!(app.mode, Mode::Help) {
                render_help_popup(f);
            }
        })?;

        if app.should_quit {
            quit_flag.store(true, Ordering::Relaxed);
            break;
        }

        tokio::select! {
            Some(ev) = key_rx.recv() => {
                match ev {
                    Event::Key(key) => handle_key(app, key, runner),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
            Some(()) = fs_rx.recv() => {
                app.refresh_status_and_log(runner);
            }
            else => break,
        }
    }

    Ok(())
}

fn setup_watcher(
    repo_path: &Path,
) -> Result<(
    mpsc::Receiver<()>,
    notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::RecommendedCache,
    >,
)> {
    let (tx, rx) = mpsc::channel::<()>(1);

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            if result.is_ok() {
                let _ = tx.try_send(());
            }
        },
    )?;

    debouncer.watch(repo_path, RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}

fn render_help_popup(f: &mut ratatui::Frame) {
    let area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        "j/k       Move up/down",
        "h/l       Switch panel",
        "Tab       Next tab",
        "Shift+Tab Previous tab",
        "Enter     View diff / expand",
        "s         Stage/unstage",
        "c         Commit",
        "p         Push",
        "P         Pull",
        "r         Refresh",
        "n/N       Next/prev hunk (diff)",
        "/         Search",
        "?         Toggle help",
        "q         Quit",
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());

    let paragraph = Paragraph::new(help_text.join("\n")).block(block);
    f.render_widget(paragraph, area);
}

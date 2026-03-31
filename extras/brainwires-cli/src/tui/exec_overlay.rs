//! Full-screen terminal overlay for executing commands
//!
//! Provides an interactive terminal that can handle stdin/stdout/stderr
//! for command execution, including interactive programs like sudo, vim, etc.

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Execute a command in a full-screen terminal overlay
///
/// NOTE: This function assumes it's being called from within a TUI that's already
/// in raw mode and alternate screen. It will NOT enter/exit those modes itself.
pub fn execute_command_overlay(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    command: &str,
) -> Result<(String, i32)> {
    // Clear the terminal buffer before showing overlay
    terminal.clear()?;

    // Execute command in background thread
    let command_str = command.to_string();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || execute_command_with_capture(&command_str, tx));

    // Display output in real-time
    let result = run_command_overlay(terminal, rx);

    // Get final result
    let (output, status) = handle
        .join()
        .map_err(|_| anyhow::anyhow!("Command thread panicked"))??;

    result?;

    // Clear terminal before returning to parent TUI
    terminal.clear()?;

    Ok((output, status))
}

/// Execute command and capture output, sending updates via channel
fn execute_command_with_capture(command: &str, tx: mpsc::Sender<String>) -> Result<(String, i32)> {
    use std::io::{BufRead, BufReader};
    use std::process::Command as StdCommand;
    use std::sync::{Arc, Mutex};

    // Execute command using shell for proper interpretation
    let mut child = StdCommand::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

    // Shared output buffer
    let full_output = Arc::new(Mutex::new(String::new()));

    // Read stdout in a separate thread
    if let Some(stdout) = child.stdout.take() {
        let tx_clone = tx.clone();
        let output_clone = Arc::clone(&full_output);

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let formatted = format!("{}\n", line);
                let _ = tx_clone.send(formatted.clone());
                if let Ok(mut out) = output_clone.lock() {
                    out.push_str(&formatted);
                }
            }
        });
    }

    // Read stderr in a separate thread
    if let Some(stderr) = child.stderr.take() {
        let output_clone = Arc::clone(&full_output);

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let formatted = format!("stderr: {}\n", line);
                let _ = tx.send(formatted.clone());
                if let Ok(mut out) = output_clone.lock() {
                    out.push_str(&formatted);
                }
            }
        });
    }

    // Wait for command to complete
    let status = child.wait().context("Failed to wait for command")?;
    let exit_code = status.code().unwrap_or(-1);

    // Extract final output
    let final_output = full_output
        .lock()
        .map(|out| out.clone())
        .unwrap_or_default();

    Ok((final_output, exit_code))
}

/// Run the command overlay and display output
fn run_command_overlay(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: mpsc::Receiver<String>,
) -> Result<()> {
    let mut output_lines: Vec<String> = Vec::new();
    let mut scroll: u16 = 0;
    let mut command_finished = false;
    let mut last_received_time = std::time::Instant::now();

    loop {
        // Try to receive new output
        let mut received_any = false;
        while let Ok(line) = rx.try_recv() {
            output_lines.push(line);
            received_any = true;
            last_received_time = std::time::Instant::now();
        }

        // Mark as finished if we haven't received output in 500ms
        if !command_finished
            && !received_any
            && last_received_time.elapsed() > Duration::from_millis(500)
        {
            command_finished = true;
        }

        terminal.draw(|f| draw_output(f, &output_lines, scroll, command_finished))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') if command_finished => break,
                KeyCode::Up => {
                    scroll = scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    scroll = scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    scroll = scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    scroll = scroll.saturating_add(10);
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

/// Draw command output
fn draw_output(f: &mut Frame, lines: &[String], scroll: u16, command_finished: bool) {
    // Clear the entire screen by filling with blank paragraph
    let blank = Paragraph::new("");
    f.render_widget(blank, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    // Output area
    let title = if command_finished {
        " Command Output - Completed "
    } else {
        " Command Output - Running... "
    };

    let border_color = if command_finished {
        Color::Green
    } else {
        Color::Cyan
    };

    let output_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let output_text: Vec<Line> = lines
        .iter()
        .skip(scroll as usize)
        .map(|l| Line::from(l.as_str()))
        .collect();

    let output = Paragraph::new(output_text)
        .block(output_block)
        .wrap(Wrap { trim: false });

    f.render_widget(output, chunks[0]);

    // Status bar
    let status_text = if command_finished {
        vec![Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Gray)),
            Span::raw(": scroll | "),
            Span::styled("PgUp/PgDn", Style::default().fg(Color::Gray)),
            Span::raw(": page | "),
            Span::styled(
                "Esc/q",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": exit"),
        ])]
    } else {
        vec![Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Gray)),
            Span::raw(": scroll | "),
            Span::styled("PgUp/PgDn", Style::default().fg(Color::Gray)),
            Span::raw(": page | "),
            Span::styled(
                "Running...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])]
    };

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .alignment(Alignment::Center);

    f.render_widget(status, chunks[1]);
}

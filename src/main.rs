use std::time::Duration;

use async_process::{Command, Stdio};
use bytesize::ByteSize;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_lite::io::BufReader;
use futures_lite::prelude::*;
use std::io;
use sysinfo::{Pid, Process, ProcessExt, System, SystemExt};
use tokio::time::Instant;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap},
    Frame, Terminal,
};

const INTERVAL_TIME: Duration = Duration::from_millis(300);

struct SysInfo {
    sys: System,
    process_start: Instant,
    // TODO this might need to track sub-process ids and be a vec
    pid: usize,
    total_memory: u64,
    num_cpus: usize,
    cpu_data: Vec<(f64, f64)>,
    stdout: String,
    stderr: String,
}

impl SysInfo {
    fn process_info(&self) -> &Process {
        self.sys
            .process(Pid::from(self.pid))
            .expect("process not spawned correctly")
    }
}

struct ShadowTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    sys_info: SysInfo,
}

impl ShadowTerminal {
    fn new(sys: System, pid: usize, process_start: Instant) -> anyhow::Result<Self> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let total_memory = sys.total_memory();
        let num_cpus = sys.cpus().len();

        Ok(Self {
            terminal,
            sys_info: SysInfo {
                sys,
                process_start,
                pid,
                total_memory,
                num_cpus,
                cpu_data: Vec::new(),
                stdout: String::new(),
                stderr: String::new(),
            },
        })
    }

    fn update_data(&mut self) -> anyhow::Result<()> {
        self.sys_info.sys.refresh_all();

        // Print data at intervals
        let time_elapsed = self.sys_info.process_start.elapsed().as_secs_f64();
        self.sys_info.cpu_data.push((
            time_elapsed,
            self.sys_info.process_info().cpu_usage() as f64,
        ));

        // // TODO fix what is output
        // println!("mem: {}", ByteSize(ps_info.memory()));
        // println!("CPU: {}", ps_info.cpu_usage());
        // let disk_usage = ps_info.disk_usage();
        // println!("Bytes read: {}", ByteSize(disk_usage.total_read_bytes));
        // println!(
        //     "Bytes written: {}",
        //     ByteSize(disk_usage.total_written_bytes)
        // );
        // println!();

        self.draw_ui()
    }

    fn push_stdout(&mut self, stdout: String) -> anyhow::Result<()> {
        self.sys_info
            .stdout
            .push_str(&[stdout.as_str(), "\n"].concat());
        self.draw_ui()
    }

    fn push_stderr(&mut self, stderr: String) -> anyhow::Result<()> {
        self.sys_info
            .stderr
            .push_str(&[stderr.as_str(), "\n"].concat());
        self.draw_ui()
    }

    fn draw_ui(&mut self) -> anyhow::Result<()> {
        self.terminal.draw(|f| terminal_ui(f, &self.sys_info))?;
        Ok(())
    }
}

impl Drop for ShadowTerminal {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
        self.terminal.show_cursor().unwrap();
    }
}

fn cpu_graph(sys_info: &SysInfo) -> Chart<'_> {
    let time_min = 0f64;
    let time_max = sys_info
        .cpu_data
        .last()
        .map(|(elapsed, _)| *elapsed)
        .unwrap_or_default();
    let x_labels = vec![
        Span::styled(
            format!("{} s", time_min),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{:.2} s", (time_min + time_max) / 2.0)),
        Span::styled(
            format!("{:.2} s", time_max),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    let cpu_min = 0f64;
    let cpu_max = 100f64 * sys_info.num_cpus as f64;
    let y_labels = vec![
        Span::styled(
            format!("{}%", cpu_min),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}%", (cpu_min + cpu_max) / 2.0)),
        Span::styled(
            format!("{}%", cpu_max),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    let datasets = vec![Dataset::default()
        .name("CPU % usage")
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(Color::Cyan))
        .data(&sys_info.cpu_data)];

    Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    "CPU",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .labels(x_labels)
                .bounds([time_min, time_max]),
        )
        .y_axis(
            Axis::default()
                .title("")
                .style(Style::default().fg(Color::Gray))
                .labels(y_labels)
                .bounds([cpu_min, cpu_max]),
        )
}

fn memory_graph(sys_info: &SysInfo) -> Chart<'_> {
    let time_min = 0f64;
    let time_max = sys_info
        .cpu_data
        .last()
        .map(|(elapsed, _)| *elapsed)
        .unwrap_or_default();
    let x_labels = vec![
        Span::styled(
            format!("{} s", time_min),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{:.2} s", (time_min + time_max) / 2.0)),
        Span::styled(
            format!("{:.2} s", time_max),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    let memory_min = 0;
    let memory_max = sys_info.total_memory;
    let y_labels = vec![
        Span::styled(
            format!("{}", ByteSize(memory_min)),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", ByteSize((memory_min + memory_max) / 2))),
        Span::styled(
            format!("{}", ByteSize(memory_max)),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    let datasets = vec![Dataset::default()
        .name("Memory usage")
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(Color::Cyan))
        .data(&sys_info.cpu_data)];

    Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    "Memory",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .labels(x_labels)
                .bounds([time_min, time_max]),
        )
        .y_axis(
            Axis::default()
                .title("")
                .style(Style::default().fg(Color::Gray))
                .labels(y_labels)
                .bounds([memory_min as f64, memory_max as f64]),
        )
}

fn stdout_ui(sys_info: &SysInfo) -> Paragraph<'_> {
    Paragraph::new(sys_info.stdout.as_str())
        .block(
            Block::default()
                .title(Span::styled(
                    "stdout",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false })
}

fn stderr_ui(sys_info: &SysInfo) -> Paragraph<'_> {
    Paragraph::new(sys_info.stderr.as_str())
        .block(
            Block::default()
                .title(Span::styled(
                    "stderr",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false })
}

fn cpu_ui(sys_info: &SysInfo) -> Paragraph<'_> {
    Paragraph::new(format!(
        "CPU usage: {:.2}%",
        sys_info.process_info().cpu_usage()
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().title(Span::styled("", Style::default())))
    .alignment(Alignment::Center)
}

fn storage_total_ui(label: &str, bytes: u64) -> Paragraph<'static> {
    Paragraph::new(format!("Storage {}: {}", label, ByteSize(bytes)))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().title(Span::styled("", Style::default())))
        .alignment(Alignment::Center)
}

fn storage_delta_ui(label: &str, bytes: u64) -> Paragraph<'static> {
    Paragraph::new(format!(
        "Storage {} /s: {}",
        label,
        ByteSize((bytes as u128 * 1000 / INTERVAL_TIME.as_millis()) as u64)
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().title(Span::styled("", Style::default())))
    .alignment(Alignment::Center)
}

fn memory_ui(sys_info: &SysInfo) -> Paragraph<'_> {
    Paragraph::new(format!(
        "Memory usage: {}",
        ByteSize(sys_info.process_info().memory())
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().title(Span::styled("", Style::default())))
    .alignment(Alignment::Center)
}

fn terminal_ui<B: Backend>(f: &mut Frame<B>, sys_info: &SysInfo) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 6),
                Constraint::Max(50),
            ]
            .as_ref(),
        )
        .split(size);

    let cpu_mem_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(chunks[0]);
    f.render_widget(cpu_ui(sys_info), cpu_mem_chunk[0]);
    f.render_widget(memory_ui(sys_info), cpu_mem_chunk[1]);

    let graphs_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(chunks[1]);
    f.render_widget(cpu_graph(sys_info), graphs_chunk[0]);
    f.render_widget(memory_graph(sys_info), graphs_chunk[1]);

    let disk_usage = sys_info.process_info().disk_usage();
    let storage_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(chunks[2]);
    let storage_read_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(storage_chunk[0]);
    f.render_widget(
        storage_total_ui("read", disk_usage.total_read_bytes),
        storage_read_chunk[0],
    );
    f.render_widget(
        storage_delta_ui("read", disk_usage.read_bytes),
        storage_read_chunk[1],
    );

    let storage_write_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(storage_chunk[1]);
    f.render_widget(
        storage_total_ui("written", disk_usage.total_written_bytes),
        storage_write_chunk[0],
    );
    f.render_widget(
        storage_delta_ui("written", disk_usage.written_bytes),
        storage_write_chunk[1],
    );

    let std_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(chunks[3]);
    f.render_widget(stdout_ui(sys_info), std_chunk[0]);
    f.render_widget(stderr_ui(sys_info), std_chunk[1]);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut sub_args = std::env::args().skip(1);
    let mut child = Command::new(sub_args.next().expect("must provide a command to run"))
        .args(sub_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sh command failed to start");
    let spawn_start = Instant::now();

    // Get output lines to show in TUI in future.
    let mut std_out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut std_err = BufReader::new(child.stderr.take().unwrap()).lines();

    // let mut sys = System::new_with_specifics(
    //     RefreshKind::new().with_processes(ProcessRefreshKind::new().with_cpu()),
    // );
    // TODO swap to only track relevant/used info (above).
    let sys = System::new_all();

    let mut terminal = ShadowTerminal::new(sys, child.id() as usize, spawn_start)?;

    let mut interval_timer = tokio::time::interval_at(spawn_start, INTERVAL_TIME);
    loop {
        tokio::select! {
            _ = interval_timer.tick() => {
                // polling for interrupt only on interval isn't ideal. Library is restrictive and
                // I'm lazy :)
                while crossterm::event::poll(Duration::from_millis(0))? {
                    if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                        if let crossterm::event::KeyCode::Char('q') = key.code {
                            return Ok(());
                        }
                    }
                }
                terminal.update_data()?;
            }
            status = child.status() => {
                println!("Process exited with status code: {}", status?);
                break;
            },
            Some(line) = std_out.next() => terminal.push_stdout(line?)?,
            Some(line) = std_err.next() => terminal.push_stderr(line?)?,
        }
    }

    // Flush remaining output and give summary
    while let Some(line) = std_out.next().await {
        terminal.push_stdout(line?)?;
    }
    while let Some(line) = std_err.next().await {
        terminal.push_stderr(line?)?;
    }

    Ok(())
}

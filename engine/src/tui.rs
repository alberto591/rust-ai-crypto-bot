use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, List, ListItem},
    Terminal,
};
use mev_core::ArbitrageOpportunity;

// Shared State Structure
pub struct AppState {
    pub total_simulated_pnl: u64,
    pub recent_opportunities: Vec<ArbitrageOpportunity>,
    pub recent_logs: Vec<String>,
    pub is_running: bool,
    pub start_time: std::time::Instant,
    pub pool_count: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            total_simulated_pnl: 0,
            recent_opportunities: Vec::new(),
            recent_logs: Vec::new(),
            is_running: true,
            start_time: std::time::Instant::now(),
            pool_count: 0,
        }
    }
}

pub struct TuiApp {
    state: Arc<Mutex<AppState>>,
}

impl TuiApp {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { state }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup Terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal);

        // Restore Terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
        let mut last_tick = std::time::Instant::now();
        let tick_rate = Duration::from_millis(200);

        loop {
            // Check for exit
            {
                let state = self.state.lock().unwrap();
                if !state.is_running {
                    return Ok(());
                }
            }

            terminal.draw(|f| self.ui(f))?;

            // Event Loop
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if let KeyCode::Char('q') = key.code {
                        let mut state = self.state.lock().unwrap();
                        state.is_running = false;
                        return Ok(());
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = std::time::Instant::now();
            }
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(4),  // Header
                Constraint::Percentage(50), // Main Content (Opp Table)
                Constraint::Percentage(50), // Logs
            ].as_ref())
            .split(f.size());

        let state = self.state.lock().unwrap();

        // 1. Header
        let pnl_sol = state.total_simulated_pnl as f64 / 1_000_000_000.0;
        let uptime = state.start_time.elapsed().as_secs();
        let pools = state.pool_count;
        
        // PnL Color Logic
        let pnl_style = if state.total_simulated_pnl > 0 {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        // Two-line header for better responsiveness
        let header_text = vec![
            Line::from(vec![
                Span::styled("Solana MEV Bot v0.3", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled("RUNNING", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("PnL: "),
                Span::styled(format!("{:.4} SOL", pnl_sol), pnl_style),
                Span::raw(" | Uptime: "),
                Span::styled(format!("{}s", uptime), Style::default().fg(Color::Blue)),
                Span::raw(" | Pools: "),
                Span::styled(format!("{}", pools), Style::default().fg(Color::Magenta)),
            ]),
        ];
        
        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).title("Dashboard"));
        f.render_widget(header, chunks[0]);

        // 2. Opportunity Table
        let header_cells = ["Timestamp", "Hops", "Profit (Lamports)", "Route"]
            .iter().map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
        let header_row = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = state.recent_opportunities.iter().rev().take(15).map(|opp| {
            let hops = opp.steps.len().to_string();
            let profit = opp.expected_profit_lamports.to_string();
            
            // Full Route Visualization
            let route_str = opp.steps.iter()
                .map(|s| {
                     // Truncate mints for display: "So11...1112" -> "So11.."
                     let m = s.input_mint.to_string();
                     format!("{}..", &m[0..4])
                })
                .collect::<Vec<_>>()
                .join(" -> ");
            
            Row::new(vec![
                Cell::from("Now"),
                Cell::from(hops),
                Cell::from(profit).style(Style::default().fg(Color::Green)),
                Cell::from(route_str),
            ])
        });

        let t = Table::new(rows, [
                Constraint::Percentage(10),
                Constraint::Percentage(5),
                Constraint::Percentage(15),
                Constraint::Percentage(70),
            ])
            .header(header_row)
            .block(Block::default().borders(Borders::ALL).title("Recent Arbitrage Opportunities (Live Feed)"))
            .column_spacing(2);
        
        f.render_widget(t, chunks[1]);

        // 3. logs
        let logs: Vec<ListItem> = state.recent_logs.iter().rev().take(20)
            .map(|l| ListItem::new(Line::from(vec![Span::raw(l)])))
            .collect();
        
        let log_list = List::new(logs)
            .block(Block::default().borders(Borders::ALL).title("Log Console"));
        f.render_widget(log_list, chunks[2]);
    }
}

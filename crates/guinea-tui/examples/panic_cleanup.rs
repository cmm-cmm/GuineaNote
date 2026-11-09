fn main() {
    let _guard = guinea_tui::TerminalGuard::enter().expect("interactive terminal");
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)
        .expect("mouse capture for cleanup probe");
    panic!("intentional terminal cleanup probe");
}

pub trait View {
    fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect);
}

impl<F: Fn(&mut ratatui::Frame, ratatui::layout::Rect)> View for F {
    fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        self(frame, area);
    }
}

impl View for () {
    fn render(&self, _frame: &mut ratatui::Frame, _area: ratatui::layout::Rect) {}
}

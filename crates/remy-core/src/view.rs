pub trait View {
    fn render(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect);
}

impl<F: Fn(&mut ratatui::buffer::Buffer, ratatui::layout::Rect)> View for F {
    fn render(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        self(buf, area);
    }
}

impl View for () {
    fn render(&self, _buf: &mut ratatui::buffer::Buffer, _area: ratatui::layout::Rect) {}
}

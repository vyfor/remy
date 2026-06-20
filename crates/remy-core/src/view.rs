use crate::cx::Rcx;

pub trait View {
    fn render(&self, rcx: Rcx, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect);
}

impl<F: Fn(Rcx, &mut ratatui::buffer::Buffer, ratatui::layout::Rect)> View for F {
    fn render(&self, rcx: Rcx, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        self(rcx, buf, area);
    }
}

impl View for () {
    fn render(&self, _rcx: Rcx, _buf: &mut ratatui::buffer::Buffer, _area: ratatui::layout::Rect) {}
}

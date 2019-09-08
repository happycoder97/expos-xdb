use pancurses::Window;

pub struct VBox {
    window: Window,
    col: i32,
    line: i32,
    padding: i32,
}

impl VBox {
    pub fn new(window: Window, padding: i32) -> Self {
        Self {
            window,
            line: 0,
            col: 0,
            padding,
        }
    }

    fn _pos(&self) {
        self.window.mv(self.line, self.padding + self.col);
    }

    pub fn text<T: AsRef<str>>(&mut self, text: T) {
        self._pos();
        self.window.addstr(text);
        self.line += 1;
        self.col = 0;
    }

    pub fn text_no_nl<T: AsRef<str>>(&mut self, text: T) {
        self._pos();
        self.window.addstr(text);
        self.col = self.window.get_cur_x();
    }

    pub fn empty_line(&mut self) {
        self.line += 1;
        self.col = 0;
    }

    pub fn hline(&mut self) {
        self.window.mv(self.line, 0);
        self.window.hline(pancurses::ACS_HLINE(), 1000);
        self.line += 1;
        self.col = 0;
    }

    pub fn get_remaining_lines(&self) -> i32 {
        let lines_rendered = self.line;
        let max_lines = self.window.get_max_y();
        let line_for_bottom_border = 2;
        max_lines - lines_rendered - line_for_bottom_border
    }

    pub fn clear(&mut self) {
        self.line = 1;  // border
        self.col = 1;   // border
        self.window.clear();
    }

    pub fn render(&self) {
        self.window.draw_box(0, 0);
        self.window.refresh();
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }
}

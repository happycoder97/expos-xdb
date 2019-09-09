use pancurses;
use pancurses::Window;
use std::thread;
use std::time::Duration;

use crate::layout::VBox;
use crate::theme;
use crate::xsm::XSM;

pub struct UI {
    stdscr: Window,
    xsm: XSM,
    window_code: VBox,
    window_regs1: VBox,
    window_regs2: VBox,
    window_page_table: VBox,
    window_output: VBox,
    window_errors: VBox,
}

impl UI {
    pub fn new(xsm: XSM) -> Self {
        let stdscr = pancurses::initscr();
        pancurses::start_color();
        pancurses::cbreak();
        pancurses::noecho();
        stdscr.keypad(true);
        stdscr.clear();
        stdscr.refresh();

        theme::init();

        let (lines, _cols) = stdscr.get_max_yx();
        let window_code = pancurses::newwin(lines, 30, 0, 0);
        let window_regs1 = pancurses::newwin(30, 30, 0, 30);
        let window_regs2 = pancurses::newwin(30, 30, 0, 60);
        let window_page_table = pancurses::newwin(30, 30, 0, 90);
        let window_output = pancurses::newwin(lines - 30, 60, 30, 30);
        let window_errors = pancurses::newwin(30, 60, 0, 120);

        Self {
            stdscr,
            xsm,
            window_code: VBox::new(window_code, 2),
            window_regs1: VBox::new(window_regs1, 2),
            window_regs2: VBox::new(window_regs2, 2),
            window_page_table: VBox::new(window_page_table, 2),
            window_output: VBox::new(window_output, 2),
            window_errors: VBox::new(window_errors, 2),
        }
    }

    fn render_code(&mut self) {
        let window = &mut self.window_code;
        window.clear();
        window.text("CODE");
        window.hline();

        let lines = window.get_remaining_lines() as usize;
        let (base, ip, code_lines) = self.xsm.get_code(lines);
        for (i, code) in code_lines.iter().enumerate() {
            let instr_addr = base + 2 * i;
            if instr_addr == ip {
                window
                    .get_window()
                    .color_set(theme::ColorPair::Selected as i16);
                window.text_no_nl(format!("{}: ", instr_addr));
                window.text(code);
                window
                    .get_window()
                    .color_set(theme::ColorPair::Normal as i16);
            } else {
                window.text_no_nl(format!("{}: ", instr_addr));
                window.text(code);
            }
        }
        window.render();
    }

    fn render_regs1(&mut self) {
        let window = &mut self.window_regs1;
        window.clear();

        window.text("REGISTERS");
        window.hline();

        for i in 0..=15usize {
            window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
        }

        for i in 15..20usize {
            window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
        }

        window.render();
    }

    fn render_regs2(&mut self) {
        let window = &mut self.window_regs2;
        window.clear();

        window.hline();
        window.text("PORTS");
        window.hline();

        for i in 0..4usize {
            window.text(format!("P{}: {}", i, &self.xsm.get_regs().p[i]));
        }

        window.hline();
        window.text("STACK");
        window.hline();
        window.text(format!("BP: {}", &self.xsm.get_regs().bp));
        window.text(format!("SP: {}", &self.xsm.get_regs().sp));

        window.hline();
        window.text("PAGE TABLE");
        window.hline();
        window.text(format!("PTBR: {}", &self.xsm.get_regs().ptbr));
        window.text(format!("PTLR: {}", &self.xsm.get_regs().ptlr));

        window.hline();
        window.text("OTHERS");
        window.hline();
        window.text(format!("IP: {}", &self.xsm.get_regs().ip));
        window.text(format!("EIP: {}", &self.xsm.get_regs().eip));
        window.text(format!("EC: {}", &self.xsm.get_regs().ec));
        window.text(format!("EPN: {}", &self.xsm.get_regs().epn));
        window.text(format!("EMA: {}", &self.xsm.get_regs().epn));

        window.render();
    }

    fn render_page_table(&mut self) {
        let window = &mut self.window_page_table;
        window.clear();

        window.text("PAGE TABLE");
        window.hline();

        for (i, entry) in self.xsm.get_page_table().iter().enumerate() {
            window.text(format!("{} -> {}    [{}]", i, entry.phy, entry.aux))
        }
        window.render();
    }

    fn render_errors(&mut self) {
        let window = &mut self.window_errors;
        window.clear();

        window.text("ERRORS");
        window.hline();

        for error in self.xsm.get_errors() {
            window.text(format!("{:#?}", error))
        }
        window.render();
    }

    fn render_output(&mut self) {
        let window = &mut self.window_output;
        window.clear();

        window.text("OUTPUT");
        window.hline();
        for line in self.xsm.get_output() {
            window.text(&line);
        }
        window.render();
    }

    fn render_all(&mut self) {
        self.render_code();
        self.render_regs1();
        self.render_regs2();
        self.render_page_table();
        self.render_errors();
        self.render_output();
    }

    pub fn render_loop(&mut self) {
        let mut i = 0;
        while !self.xsm.is_halted() {
            i += 20;
            self.xsm.step(20);
            self.render_all();
            thread::sleep(Duration::from_millis(100));
            // let ch: pancurses::Input = self.stdscr.getch().unwrap();
            // if ch == pancurses::Input::Character('s') {
            // } else {
            //     panic!("Testing panic");
            // }
        }
        self.render_all();
        self.stdscr.getch();
        self.exit()
    }

    #[allow(dead_code)]
    pub fn wait_for_keypress(&mut self) {
        self.stdscr.getch();
    }

    #[allow(dead_code)]
    pub fn exit(&mut self) {
        pancurses::endwin();
    }
}

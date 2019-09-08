use pancurses;
use pancurses::Window;
use std::thread;
use std::time::Duration;

use crate::layout::VBox;
use crate::xsm::XSM;

pub struct UI {
    stdscr: Window,
    xsm: XSM,
    window_code: VBox,
    window_regs: VBox,
}

enum ColorPairs {
    Normal = 0,
    Selected = 1,
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

        let (lines, _cols) = stdscr.get_max_yx();
        let window_code = pancurses::newwin(lines, 30, 0, 0);
        let window_regs = pancurses::newwin(lines, 30, 0, 30);

        pancurses::init_pair(
            ColorPairs::Normal as i16,
            pancurses::COLOR_WHITE,
            pancurses::COLOR_BLACK,
        );
        pancurses::init_pair(
            ColorPairs::Selected as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_WHITE,
        );

        Self {
            stdscr,
            xsm,
            window_code: VBox::new(window_code, 2),
            window_regs: VBox::new(window_regs, 2),
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
                window.get_window().color_set(ColorPairs::Selected as i16);
                window.text_no_nl(format!("{}: ", instr_addr));
                window.text(code);
                window.get_window().color_set(ColorPairs::Normal as i16);
            } else {
                window.text_no_nl(format!("{}: ", instr_addr));
                window.text(code);
            }
        }
        window.render();
    }

    fn render_regs(&mut self) {
        let window = &mut self.window_regs;
        window.clear();

        window.text("REGISTERS");
        window.hline();

        for i in 0..=15usize {
            window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
        }

        for i in 15..20usize {
            window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
        }

        for i in 0..4usize {
            window.text(format!("P{}: {}", i, &self.xsm.get_regs().p[i]));
        }
        window.render();
    }

    pub fn render_loop(&mut self) {
        for _ in 0..1000 {
            self.render_code();
            self.render_regs();
            self.xsm.step();
            thread::sleep(Duration::from_millis(500));
            // let ch: pancurses::Input = self.stdscr.getch().unwrap();
            // if ch == pancurses::Input::Character('s') {
            // } else {
            //     panic!("Testing panic");
            // }
        }
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

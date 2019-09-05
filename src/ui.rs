use pancurses;
use pancurses::Window;

use std::convert::TryInto;

use crate::xsm::XSM;

pub struct UI {
    stdscr: Window,
    xsm: XSM,
    window_code: Window,
    window_regs: Window,
}

enum ColorPairs {
    Normal = 0,
    Selected = 1,
}

impl UI {
    pub fn new(xsm: XSM) -> Self {
        let stdscr = pancurses::initscr();
        pancurses::cbreak();
        pancurses::noecho();
        stdscr.keypad(true);
        stdscr.clear();
        stdscr.refresh();

        let (lines, cols) = stdscr.get_max_yx();
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
            window_code,
            window_regs,
        }
    }

    fn render_code(&mut self) {
        self.window_code.clear();
        self.window_code.mvaddstr(1, 2, "CODE");
        self.window_code.mv(2, 0);
        self.window_code.hline(pancurses::ACS_HLINE(), 1000);
        let begin_y = 3;

        let lines = self.window_code.get_max_y() as usize - begin_y;
        let t = std::time::Instant::now();
        let (base, ip, code_lines) = self.xsm.get_code(lines);
        eprintln!("Get code: {}", t.elapsed().as_millis());
        for (i, code) in code_lines.iter().enumerate() {
            self.window_code.mv((begin_y + i) as i32, 2);
            if base + i == ip {
                self.window_code.color_set(ColorPairs::Selected as i16);
            }
            self.window_code.addstr(format!("{}: ", base + 2 * i));
            self.window_code.addstr(code);
            if base + i == ip {
                self.window_code.color_set(ColorPairs::Normal as i16);
            }
        }
        self.window_code.draw_box(0, 0);
        self.window_code.refresh();
    }

    fn render_regs(&mut self) {

    }

    pub fn render_loop(&mut self) {
        for i in 0..2 {
            eprintln!("----");
            let t = std::time::Instant::now();
            self.render_code();
            eprintln!("Render code: {}", t.elapsed().as_millis());
            let t = std::time::Instant::now();
            self.xsm.step();
            eprintln!("Step: {}", t.elapsed().as_millis());
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

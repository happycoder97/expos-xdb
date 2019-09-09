use imgui::{Condition, Ui};

use crate::xsm::XSM;

pub struct UI {
    xsm: XSM,
}

impl UI {
    pub fn new(xsm: XSM) -> Self {
        Self { xsm }
    }

    fn render_code(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Code"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                let [_, y_px] = ui.window_size();
                let lines = ((y_px - 70.0)/20.0) as usize;
                dbg!(y_px, lines);
                let (base, ip, code_lines) = self.xsm.get_code(lines);
                for (i, code) in code_lines.iter().enumerate() {
                    let instr_addr = base + 2 * i;
                    if instr_addr == ip {
                        ui.text(im_str!(">> {}: {}", instr_addr, code));
                    } else {
                        ui.text(im_str!("{}: {}", instr_addr, code));
                    }
                }
            });
    }

    // fn render_regs1(&mut self) {
    //     let window = &mut self.window_regs1;
    //     window.clear();

    //     window.text("REGISTERS");
    //     window.hline();

    //     for i in 0..=15usize {
    //         window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
    //     }

    //     for i in 15..20usize {
    //         window.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
    //     }

    //     window.render();
    // }

    // fn render_regs2(&mut self) {
    //     let window = &mut self.window_regs2;
    //     window.clear();

    //     window.hline();
    //     window.text("PORTS");
    //     window.hline();

    //     for i in 0..4usize {
    //         window.text(format!("P{}: {}", i, &self.xsm.get_regs().p[i]));
    //     }

    //     window.hline();
    //     window.text("STACK");
    //     window.hline();
    //     window.text(format!("BP: {}", &self.xsm.get_regs().bp));
    //     window.text(format!("SP: {}", &self.xsm.get_regs().sp));

    //     window.hline();
    //     window.text("PAGE TABLE");
    //     window.hline();
    //     window.text(format!("PTBR: {}", &self.xsm.get_regs().ptbr));
    //     window.text(format!("PTLR: {}", &self.xsm.get_regs().ptlr));

    //     window.hline();
    //     window.text("OTHERS");
    //     window.hline();
    //     window.text(format!("IP: {}", &self.xsm.get_regs().ip));
    //     window.text(format!("EIP: {}", &self.xsm.get_regs().eip));
    //     window.text(format!("EC: {}", &self.xsm.get_regs().ec));
    //     window.text(format!("EPN: {}", &self.xsm.get_regs().epn));
    //     window.text(format!("EMA: {}", &self.xsm.get_regs().epn));

    //     window.render();
    // }

    // fn render_page_table(&mut self) {
    //     let window = &mut self.window_page_table;
    //     window.clear();

    //     window.text("PAGE TABLE");
    //     window.hline();

    //     for (i, entry) in self.xsm.get_page_table().iter().enumerate() {
    //         window.text(format!("{} -> {}    [{}]", i, entry.phy, entry.aux))
    //     }
    //     window.render();
    // }

    // fn render_errors(&mut self) {
    //     let window = &mut self.window_errors;
    //     window.clear();

    //     window.text("ERRORS");
    //     window.hline();

    //     for error in self.xsm.get_errors() {
    //         window.text(format!("{:#?}", error))
    //     }
    //     window.render();
    // }

    // fn render_output(&mut self) {
    //     let window = &mut self.window_output;
    //     window.clear();

    //     window.text("OUTPUT");
    //     window.hline();
    //     for line in self.xsm.get_output() {
    //         window.text(&line);
    //     }
    //     window.render();
    // }

    pub fn render_all(&mut self, ui: &mut Ui) {
        self.render_code(ui);
        // self.render_regs1();
        // self.render_regs2();
        // self.render_page_table();
        // self.render_errors();
        // self.render_output();
    }

    // #[allow(dead_code)]
    // pub fn wait_for_keypress(&mut self) {
    //     self.stdscr.getch();
    // }

    // #[allow(dead_code)]
    // pub fn exit(&mut self) {
    //     pancurses::endwin();
    // }
}

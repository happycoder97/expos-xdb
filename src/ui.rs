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
                let lines = 30;
                let (base, ip, code_lines) = self.xsm.get_code(lines);
                for (i, code) in code_lines.iter().enumerate() {
                    let instr_addr = base + 2 * i;
                    if instr_addr == ip {
                        imgui::MenuItem::new(&im_str!("{}: {} ---- (IP)", instr_addr, code)).build(ui);
                    } else {
                        imgui::MenuItem::new(&im_str!("{}: {}", instr_addr, code)).build(ui);
                    }
                }
            });
    }

    fn render_regs1(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Registers"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                for i in 0..=15usize {
                    ui.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
                }
                ui.separator();
                for i in 15..20usize {
                    ui.text(format!("R{}: {}", i, &self.xsm.get_regs().r[i]));
                }
            });
    }

    fn render_regs2(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Registers Extra"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text("PORTS");
                ui.separator();

                for i in 0..4usize {
                    ui.text(format!("P{}: {}", i, &self.xsm.get_regs().p[i]));
                }

                ui.new_line();
                ui.text("STACK");
                ui.separator();
                ui.text(format!("BP: {}", &self.xsm.get_regs().bp));
                ui.text(format!("SP: {}", &self.xsm.get_regs().sp));

                ui.new_line();
                ui.text("PAGE TABLE");
                ui.separator();
                ui.text(format!("PTBR: {}", &self.xsm.get_regs().ptbr));
                ui.text(format!("PTLR: {}", &self.xsm.get_regs().ptlr));

                ui.new_line();
                ui.text("OTHERS");
                ui.separator();
                ui.text(format!("IP: {}", &self.xsm.get_regs().ip));
                ui.text(format!("EIP: {}", &self.xsm.get_regs().eip));
                ui.text(format!("EC: {}", &self.xsm.get_regs().ec));
                ui.text(format!("EPN: {}", &self.xsm.get_regs().epn));
                ui.text(format!("EMA: {}", &self.xsm.get_regs().epn));
            });
    }

    fn render_page_table(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Page Table"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                for (i, entry) in self.xsm.get_page_table().iter().enumerate() {
                    ui.text(format!("{} -> {}    [{}]", i, entry.phy, entry.aux))
                }
            });
    }

    fn render_errors(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Errors"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                for error in self.xsm.get_errors() {
                    ui.text(format!("{:#?}", error))
                }
            });
    }

    fn render_output(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Output"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                for line in self.xsm.get_output() {
                    ui.text(&line);
                }
            });
    }

    pub fn render_all(&mut self, ui: &mut Ui) {
        self.render_code(ui);
        self.render_regs1(ui);
        self.render_regs2(ui);
        self.render_page_table(ui);
        self.render_errors(ui);
        self.render_output(ui);
    }
}

use imgui::{Condition, Ui};

use crate::xsm::XSM;
use std::convert::TryInto;

pub struct UI {
    xsm: XSM,
    is_continue: bool,
    step: usize,
    update_delay: f64,
    step_size: usize,
    last_time: f64,
    input_cmd: imgui::ImString
}

impl UI {
    pub fn new(xsm: XSM) -> Self {
        Self {
            xsm,
            is_continue: true,
            step: 0,
            step_size: 1,
            last_time: 0.0,
            update_delay: 1.0,
            input_cmd: imgui::ImString::new("")
        }
    }

    fn render_code(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Code"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                let lines = 10;
                let (base, ip, code_lines) = self.xsm.get_code(lines);
                for (i, code) in code_lines.iter().enumerate() {
                    let instr_addr = base + 2 * i;
                    if instr_addr == ip {
                        imgui::MenuItem::new(&im_str!("[{}]: {}", instr_addr, code))
                            .build(ui);
                    } else {
                        imgui::MenuItem::new(&im_str!(" {} : {}", instr_addr, code)).build(ui);
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

    fn render_status(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Status"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                for line in self.xsm.get_status().lines() {
                    ui.text(&line);
                }
                ui.separator();
                if self.is_continue {
                    ui.text(im_str!("Pause execution to send commands to xsm"));
                    if ui.button(im_str!("Pause"), [0.0,0.0]) {
                        self.is_continue = false;
                    }
                } else {
                    if ui.button(im_str!("Resume"), [0.0,0.0]) {
                        self.is_continue = true;
                    }
                    ui.input_text(im_str!("debug>"), &mut self.input_cmd).build();
                    if ui.button(im_str!("Send"), [0.0, 0.0]) {
                    }
                }
            });
    }

    fn render_control_panel(&mut self, ui: &mut Ui) {
        imgui::Window::new(im_str!("Control Panel"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.checkbox(im_str!("Continue"), &mut self.is_continue);
                let mut step_size = self.step_size as i32;
                ui.input_int(im_str!("Update Delay"), &mut step_size)
                    .build();
                self.step_size = step_size.try_into().unwrap_or(1);
                let mut update_delay = self.update_delay as f32;
                ui.input_float(im_str!("Update Delay"), &mut update_delay)
                    .build();
                self.update_delay = update_delay.into();
                if ui.button(im_str!("Fast Forward"), [0.0, 0.0]) {
                    self.update_delay = 0.1;
                }
                ui.same_line(0.0);
                if ui.button(im_str!("Normal"), [0.0, 0.0]) {
                    self.update_delay = 0.8;
                }
                ui.same_line(0.0);
                if self.is_continue {
                    if ui.button(im_str!("Pause"), [0.0, 0.0]) {
                        self.is_continue = false;
                    }
                } else {
                    if ui.button(im_str!("Resume"), [0.0, 0.0]) {
                        self.is_continue = true;
                    }
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
        self.render_status(ui);
        self.render_control_panel(ui);

        if self.is_continue && ui.time() - self.last_time > self.update_delay {
            self.xsm.step(self.step_size);
            self.step += self.step_size;
            self.last_time = ui.time();
        }
    }
}

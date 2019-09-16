#![allow(dead_code)]

#[macro_use]
extern crate imgui;

use xsm::XSM;

mod xsm;
#[macro_use]
extern crate try_or;

mod ui;
mod ui_support;

static XSM_CMDLINE: &str = "xsm --disk-file disk.xfs --debug --timer 100";

fn main() {
    let xsm = XSM::spawn_new(XSM_CMDLINE).expect("Error loading xsm");
    let mut xsm_ui = ui::UI::new(xsm);
    let sys = ui_support::init("XDB - Visual Debugger for eXpOS");
    sys.main_loop(|_, ui| xsm_ui.render_all(ui));
}

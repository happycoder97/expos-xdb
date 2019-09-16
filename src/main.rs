#![allow(dead_code)]

#[macro_use]
extern crate imgui;
#[macro_use]
extern crate try_or;

use xsm::XSM;

mod xsm;

mod ui;
mod ui_support;

fn main() {
    let args :Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        println!("XDB: Visual debugger for XSM");
        println!("Syntax:");
        println!("xdb <xsm command line>");
        println!();
        println!("Example: ");
        println!("xdb xsm --debug --timer 100");
        return;
    }
    let command_line = args.iter().skip(1).fold(String::new(), |acc, x| {
        acc + " " + x
    });
    let xsm = try_or!(XSM::spawn_new(&command_line), ());
    let mut xsm_ui = ui::UI::new(xsm);
    let sys = ui_support::init("XDB - Visual Debugger for eXpOS");
    sys.main_loop(|_, ui| xsm_ui.render_all(ui));
}

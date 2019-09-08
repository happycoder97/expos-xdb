#![allow(dead_code)]

mod xsm;
use xsm::XSM;

mod ui;
mod layout;

static XSM_CMDLINE: &str = "xsm --disk-file disk.xfs --debug";

fn main() {
    let std_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        pancurses::endwin();
        std_panic_hook(panic_info);
    }));
    let xsm = XSM::spawn_new(XSM_CMDLINE).expect("Error loading xsm");
    let mut ui = ui::UI::new(xsm);
    ui.render_loop();
}

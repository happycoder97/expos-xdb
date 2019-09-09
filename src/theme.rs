use pancurses;

pub enum ColorPair {
    Normal = 0,
    Selected = 1,
}

pub fn init() {
    pancurses::init_pair(
        ColorPair::Normal as i16,
        pancurses::COLOR_WHITE,
        pancurses::COLOR_BLACK,
    );
    pancurses::init_pair(
        ColorPair::Selected as i16,
        pancurses::COLOR_BLACK,
        pancurses::COLOR_WHITE,
    );
}
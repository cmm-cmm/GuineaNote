use ratatui::style::{Color, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Color,
    pub panel: Color,
    pub raised: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub mint: Color,
    pub blue: Color,
    pub amber: Color,
    pub purple: Color,
    pub red: Color,
}

impl Theme {
    pub fn new(basic: bool) -> Self {
        if basic {
            Self {
                canvas: Color::Indexed(233),
                panel: Color::Indexed(234),
                raised: Color::Indexed(236),
                line: Color::Indexed(238),
                text: Color::Indexed(253),
                muted: Color::Indexed(245),
                faint: Color::Indexed(240),
                mint: Color::Indexed(85),
                blue: Color::Indexed(111),
                amber: Color::Indexed(221),
                purple: Color::Indexed(147),
                red: Color::Indexed(203),
            }
        } else {
            Self {
                canvas: Color::Rgb(12, 17, 28),
                panel: Color::Rgb(17, 24, 39),
                raised: Color::Rgb(25, 36, 54),
                line: Color::Rgb(37, 51, 72),
                text: Color::Rgb(216, 226, 240),
                muted: Color::Rgb(126, 146, 173),
                faint: Color::Rgb(65, 83, 108),
                mint: Color::Rgb(94, 231, 183),
                blue: Color::Rgb(131, 181, 255),
                amber: Color::Rgb(240, 190, 106),
                purple: Color::Rgb(185, 158, 255),
                red: Color::Rgb(255, 126, 144),
            }
        }
    }
    pub fn base(self) -> Style {
        Style::default().bg(self.canvas).fg(self.text)
    }
}

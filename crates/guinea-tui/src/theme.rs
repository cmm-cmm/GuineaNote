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
    pub accent: Color,
    pub secondary: Color,
    pub warm: Color,
    pub keyword: Color,
    pub danger: Color,
}

impl Theme {
    pub fn new(basic: bool) -> Self {
        if basic {
            Self {
                canvas: Color::Indexed(232),
                panel: Color::Indexed(233),
                raised: Color::Indexed(235),
                line: Color::Indexed(95),
                text: Color::Indexed(230),
                muted: Color::Indexed(144),
                faint: Color::Indexed(101),
                accent: Color::Indexed(114),
                secondary: Color::Indexed(151),
                warm: Color::Indexed(179),
                keyword: Color::Indexed(150),
                danger: Color::Indexed(216),
            }
        } else {
            Self {
                // A slightly deeper navy, with distinct panel/raised surfaces.
                // Green, honey amber and warm white keep details legible.
                canvas: Color::Rgb(9, 14, 23),
                panel: Color::Rgb(13, 20, 34),
                raised: Color::Rgb(22, 32, 49),
                line: Color::Rgb(112, 90, 67),
                text: Color::Rgb(248, 245, 236),
                muted: Color::Rgb(187, 182, 165),
                faint: Color::Rgb(145, 129, 107),
                accent: Color::Rgb(134, 211, 138),
                secondary: Color::Rgb(189, 217, 179),
                warm: Color::Rgb(219, 173, 91),
                keyword: Color::Rgb(186, 214, 138),
                danger: Color::Rgb(239, 170, 145),
            }
        }
    }
    pub fn base(self) -> Style {
        Style::default().bg(self.canvas).fg(self.text)
    }
}

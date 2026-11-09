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
                canvas: Color::Indexed(233),
                panel: Color::Indexed(234),
                raised: Color::Indexed(236),
                line: Color::Indexed(95),
                text: Color::Indexed(230),
                muted: Color::Indexed(144),
                faint: Color::Indexed(101),
                accent: Color::Indexed(114),
                secondary: Color::Indexed(151),
                warm: Color::Indexed(180),
                keyword: Color::Indexed(150),
                danger: Color::Indexed(216),
            }
        } else {
            Self {
                // Preserve the original navy surfaces. Green, walnut and
                // warm white belong to content and details; danger is clay.
                canvas: Color::Rgb(12, 17, 28),
                panel: Color::Rgb(17, 24, 39),
                raised: Color::Rgb(25, 36, 54),
                line: Color::Rgb(112, 90, 67),
                text: Color::Rgb(248, 245, 236),
                muted: Color::Rgb(187, 182, 165),
                faint: Color::Rgb(145, 129, 107),
                accent: Color::Rgb(134, 211, 138),
                secondary: Color::Rgb(189, 217, 179),
                warm: Color::Rgb(218, 181, 138),
                keyword: Color::Rgb(186, 214, 138),
                danger: Color::Rgb(239, 170, 145),
            }
        }
    }
    pub fn base(self) -> Style {
        Style::default().bg(self.canvas).fg(self.text)
    }
}

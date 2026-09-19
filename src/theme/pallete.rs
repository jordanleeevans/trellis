use ratatui::style::Color;

pub struct PastelPallette;

impl PastelPallette {
    pub const CYAN: Color = Color::Rgb(137, 220, 235);
    pub const MAGENTA: Color = Color::Rgb(221, 182, 242);
    pub const BLUE: Color = Color::Rgb(166, 191, 255);

    pub const SURFACE: Color = Color::Rgb(30, 30, 46);

    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const TEXT_MUTED: Color = Color::Rgb(147, 153, 178);
    pub const TEXT_INVERSE: Color = Color::Rgb(30, 30, 46);

    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const LINK: Color = Color::Rgb(137, 180, 250);
}

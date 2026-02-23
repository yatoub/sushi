use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border: Color,
    pub group_header: Color,
    pub env_header: Color,
    pub server_item: Color,
    pub search_box: Color,
    pub search_text: Color,
}

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    bg: Color::Rgb(30, 30, 46),          // Base
    fg: Color::Rgb(205, 214, 244),       // Text
    selection_bg: Color::Rgb(49, 50, 68), // Surface0
    selection_fg: Color::Rgb(203, 166, 247), // Mauve
    border: Color::Rgb(147, 153, 178),   // Overlay2
    group_header: Color::Rgb(137, 180, 250), // Blue
    env_header: Color::Rgb(166, 227, 161),   // Green
    server_item: Color::Rgb(205, 214, 244),  // Text
    search_box: Color::Rgb(69, 71, 90),  // Surface1
    search_text: Color::Rgb(249, 226, 175), // Yellow
};

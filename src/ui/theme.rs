use std::sync::LazyLock;
use ratatui::style::Color;

pub struct Theme {
    #[allow(dead_code)]
    pub bg: Color,
    pub fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border: Color,
    pub group_header: Color,
    pub env_header: Color,
    pub server_item: Color,
    #[allow(dead_code)]
    pub search_box: Color,
    pub search_text: Color,
    #[allow(dead_code)]
    pub yellow: Color,
    pub subtext0: Color,
}

pub static CATPPUCCIN_MOCHA: LazyLock<Theme> = LazyLock::new(|| {
    let mocha = catppuccin::PALETTE.mocha;
    let colors = mocha.colors;
    
    Theme {
        bg: to_ratatui(colors.base),
        fg: to_ratatui(colors.text),
        selection_bg: to_ratatui(colors.surface0),
        selection_fg: to_ratatui(colors.mauve),
        border: to_ratatui(colors.overlay2),
        group_header: to_ratatui(colors.blue),
        env_header: to_ratatui(colors.green),
        server_item: to_ratatui(colors.text),
        search_box: to_ratatui(colors.surface1),
        search_text: to_ratatui(colors.yellow),
        yellow: to_ratatui(colors.yellow),
        subtext0: to_ratatui(colors.subtext0),
    }
});

fn to_ratatui(c: catppuccin::Color) -> Color {
    Color::Rgb(c.rgb.r, c.rgb.g, c.rgb.b)
}

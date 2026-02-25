use ratatui::style::Color;
use std::sync::LazyLock;

use crate::config::ThemeVariant;

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
    pub green: Color,
    pub red: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub subtext0: Color,
}

fn to_ratatui(c: catppuccin::Color) -> Color {
    Color::Rgb(c.rgb.r, c.rgb.g, c.rgb.b)
}

fn make_theme(flavor: catppuccin::Flavor) -> Theme {
    let colors = flavor.colors;
    Theme {
        bg: to_ratatui(colors.base),
        fg: to_ratatui(colors.text),
        selection_bg: to_ratatui(colors.surface2),
        selection_fg: to_ratatui(colors.text),
        border: to_ratatui(colors.blue),
        group_header: to_ratatui(colors.mauve),
        env_header: to_ratatui(colors.green),
        server_item: to_ratatui(colors.text),
        search_box: to_ratatui(colors.surface1),
        search_text: to_ratatui(colors.text),
        yellow: to_ratatui(colors.yellow),
        green: to_ratatui(colors.green),
        red: to_ratatui(colors.red),
        sky: to_ratatui(colors.sky),
        sapphire: to_ratatui(colors.sapphire),
        subtext0: to_ratatui(colors.subtext0),
    }
}

pub static CATPPUCCIN_LATTE: LazyLock<Theme> =
    LazyLock::new(|| make_theme(catppuccin::PALETTE.latte));

pub static CATPPUCCIN_FRAPPE: LazyLock<Theme> =
    LazyLock::new(|| make_theme(catppuccin::PALETTE.frappe));

pub static CATPPUCCIN_MACCHIATO: LazyLock<Theme> =
    LazyLock::new(|| make_theme(catppuccin::PALETTE.macchiato));

pub static CATPPUCCIN_MOCHA: LazyLock<Theme> =
    LazyLock::new(|| make_theme(catppuccin::PALETTE.mocha));

/// Retourne le thème statique correspondant à la variante choisie.
pub fn get_theme(variant: ThemeVariant) -> &'static Theme {
    match variant {
        ThemeVariant::Latte => &*CATPPUCCIN_LATTE,
        ThemeVariant::Frappe => &*CATPPUCCIN_FRAPPE,
        ThemeVariant::Macchiato => &*CATPPUCCIN_MACCHIATO,
        ThemeVariant::Mocha => &*CATPPUCCIN_MOCHA,
    }
}

use std::io;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use crossterm::event::MouseEvent;
use crate::app::App;
use crate::config::ConnectionMode;

pub struct AppLayout {
    pub list_area: Rect,
    pub tabs_area: Rect,
}

pub fn get_layout(size: Rect) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main
            Constraint::Length(1), // Status
        ])
        .split(size);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(67), // List
            Constraint::Min(0),         // Details
        ])
        .split(chunks[2]);

    AppLayout {
        tabs_area: chunks[1],
        list_area: main_chunks[0],
    }
}

pub fn is_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

pub fn handle_mouse_event(mouse: MouseEvent, app: &mut App, size: Rect) -> io::Result<bool> {
    let layout = get_layout(size);

    if is_in_rect(mouse.column, mouse.row, layout.tabs_area) {
        let titles = vec!["Direct [1]", "Rebond [2]", "Bastion [3]"];
        let separator_width = 1; 
        
        // Tabs block has Borders::ALL, so content starts at x+1
        let start_x = layout.tabs_area.x + 1;
        
        if mouse.column < start_x {
             return Ok(false);
        }

        let rel_x = (mouse.column - start_x) as usize;
        let mut current_x = 0;
        
        for (i, title) in titles.iter().enumerate() {
            let width = title.chars().count();
            // Check if click is strictly within the title text
            if rel_x >= current_x && rel_x < current_x + width {
                app.connection_mode = ConnectionMode::from_index(i);
                return Ok(true);
            }
            // Advance cursor (title + separator)
            current_x += width + separator_width;
        }
        
        return Ok(true); 
    } else if is_in_rect(mouse.column, mouse.row, layout.list_area) {
        // Determine item index
        // List renders inside the block. Block has Borders::ALL -> 1px padding
        let inner_y = layout.list_area.y + 1;
        let inner_h = layout.list_area.height.saturating_sub(2);
        
        if mouse.row >= inner_y && mouse.row < inner_y + inner_h {
            let row_idx = (mouse.row - inner_y) as usize;
            
            let offset = app.list_state.offset(); 
            let target_index = offset + row_idx;
            
            // Check bounds
            if target_index < app.get_visible_items().len() {
                app.select(target_index);
                // Toggle expansion on single click if it's a group or env
                app.toggle_expansion();
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}

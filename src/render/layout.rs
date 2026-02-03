//! Dynamic layout engine for responsive UI
//!
//! Provides layout calculations that adapt to terminal width,
//! optimized for AI pair programming workflows with narrow terminals.

use ratatui::layout::Rect;

use crate::core::UiDensity;

/// Tree column configuration
#[derive(Debug, Clone)]
pub struct TreeColumns {
    /// Width for selection marker
    pub mark_width: u16,
    /// Width for git status indicator
    pub git_indicator_width: u16,
    /// Width for indentation per level
    pub indent_width: u16,
    /// Width for icon
    pub icon_width: u16,
    /// Maximum width for filename
    pub max_filename_width: u16,
    /// Whether to show icons
    pub show_icons: bool,
}

impl TreeColumns {
    /// Create tree columns for given density and area
    pub fn new(density: UiDensity, area_width: u16) -> Self {
        let inner_width = area_width.saturating_sub(2); // Account for borders

        match density {
            UiDensity::Ultra => Self {
                mark_width: 1,
                git_indicator_width: 1,
                indent_width: 1,
                icon_width: 0, // No icons in ultra mode
                max_filename_width: inner_width.saturating_sub(3),
                show_icons: false,
            },
            UiDensity::Narrow => Self {
                mark_width: 1,
                git_indicator_width: 1,
                indent_width: 1,
                icon_width: 0, // No icons in narrow mode
                max_filename_width: inner_width.saturating_sub(3),
                show_icons: false,
            },
            UiDensity::Compact => Self {
                mark_width: 1,
                git_indicator_width: 1,
                indent_width: 2,
                icon_width: 2,
                max_filename_width: inner_width.saturating_sub(6),
                show_icons: true,
            },
            UiDensity::Full => Self {
                mark_width: 1,
                git_indicator_width: 1,
                indent_width: 2,
                icon_width: 2,
                max_filename_width: inner_width.saturating_sub(6),
                show_icons: true,
            },
        }
    }

    /// Calculate available width for filename at a given depth
    pub fn filename_width_at_depth(&self, depth: usize) -> u16 {
        let used = self.mark_width
            + self.git_indicator_width
            + (depth as u16 * self.indent_width)
            + self.icon_width
            + 1; // Space after icon
        self.max_filename_width.saturating_sub(used)
    }
}

/// Status bar layout configuration
#[derive(Debug, Clone)]
pub struct StatusLayout {
    /// Number of panels (1 for ultra/narrow, 2 for compact/full)
    pub panel_count: u8,
    /// Whether to show git branch
    pub show_git_branch: bool,
    /// Whether to show file info
    pub show_file_info: bool,
    /// Whether to show selection count
    pub show_selection_count: bool,
    /// Whether to show sort mode
    pub show_sort_mode: bool,
    /// Whether to show filter indicator
    pub show_filter: bool,
    /// Whether to show watch indicator
    pub show_watch: bool,
    /// Maximum branch name length
    pub max_branch_len: usize,
    /// Maximum message length
    pub max_message_len: usize,
}

impl StatusLayout {
    /// Create status layout for given density and width
    pub fn new(density: UiDensity, width: u16) -> Self {
        match density {
            UiDensity::Ultra => Self {
                panel_count: 1,
                show_git_branch: true,
                show_file_info: false,
                show_selection_count: true,
                show_sort_mode: false,
                show_filter: true,
                show_watch: false,
                max_branch_len: 6,
                max_message_len: (width.saturating_sub(15)) as usize,
            },
            UiDensity::Narrow => Self {
                panel_count: 1,
                show_git_branch: true,
                show_file_info: true,
                show_selection_count: true,
                show_sort_mode: false,
                show_filter: true,
                show_watch: false,
                max_branch_len: 8,
                max_message_len: (width.saturating_sub(20)) as usize,
            },
            UiDensity::Compact => Self {
                panel_count: 2,
                show_git_branch: true,
                show_file_info: true,
                show_selection_count: true,
                show_sort_mode: true,
                show_filter: true,
                show_watch: true,
                max_branch_len: 12,
                max_message_len: (width / 2).saturating_sub(10) as usize,
            },
            UiDensity::Full => Self {
                panel_count: 2,
                show_git_branch: true,
                show_file_info: true,
                show_selection_count: true,
                show_sort_mode: true,
                show_filter: true,
                show_watch: true,
                max_branch_len: 20,
                max_message_len: (width / 2).saturating_sub(10) as usize,
            },
        }
    }
}

/// Main layout engine
#[derive(Debug, Clone)]
pub struct LayoutEngine {
    /// Terminal width
    pub width: u16,
    /// Terminal height
    pub height: u16,
    /// Current UI density
    pub density: UiDensity,
}

impl LayoutEngine {
    /// Create a new layout engine from terminal dimensions
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            density: UiDensity::from_width(width),
        }
    }

    /// Create from a Rect
    pub fn from_rect(area: Rect) -> Self {
        Self::new(area.width, area.height)
    }

    /// Get tree column configuration
    pub fn tree_columns(&self, area: Rect) -> TreeColumns {
        TreeColumns::new(self.density, area.width)
    }

    /// Get status layout configuration
    pub fn status_layout(&self) -> StatusLayout {
        StatusLayout::new(self.density, self.width)
    }

    /// Check if icons should be shown
    pub fn should_show_icons(&self) -> bool {
        self.density.show_icons()
    }

    /// Get maximum filename width for tree display
    pub fn max_filename_width(&self, area: Rect) -> usize {
        let cols = self.tree_columns(area);
        cols.max_filename_width as usize
    }

    /// Get peek preview line count
    pub fn peek_preview_lines(&self) -> usize {
        self.density.peek_preview_lines()
    }

    /// Calculate tree/preview split ratio
    ///
    /// Returns (tree_percentage, preview_percentage)
    pub fn split_ratio(&self, preview_visible: bool) -> (u16, u16) {
        if !preview_visible {
            return (100, 0);
        }

        match self.density {
            UiDensity::Ultra | UiDensity::Narrow => {
                // No side-by-side preview in narrow modes
                (100, 0)
            }
            UiDensity::Compact => (50, 50),
            UiDensity::Full => (40, 60),
        }
    }

    /// Check if preview should be visible given current settings
    pub fn should_show_preview(&self, preview_enabled: bool) -> bool {
        if !preview_enabled {
            return false;
        }

        // In ultra/narrow modes, use peek mode instead of side preview
        !matches!(self.density, UiDensity::Ultra | UiDensity::Narrow)
    }

    /// Get help popup dimensions
    pub fn help_popup_size(&self) -> (u16, u16) {
        let width = match self.density {
            UiDensity::Ultra => self.width.saturating_sub(2),
            UiDensity::Narrow => self.width.saturating_sub(2),
            UiDensity::Compact => (self.width * 90 / 100).max(40),
            UiDensity::Full => (self.width * 80 / 100).max(60),
        };

        let height = self.height.saturating_sub(2).min(40);
        (width, height)
    }

    /// Get title max length for tree panel
    pub fn title_max_len(&self, area: Rect) -> usize {
        (area.width.saturating_sub(4)) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density_from_width() {
        assert_eq!(UiDensity::from_width(20), UiDensity::Ultra);
        assert_eq!(UiDensity::from_width(24), UiDensity::Ultra);
        assert_eq!(UiDensity::from_width(25), UiDensity::Narrow);
        assert_eq!(UiDensity::from_width(39), UiDensity::Narrow);
        assert_eq!(UiDensity::from_width(40), UiDensity::Compact);
        assert_eq!(UiDensity::from_width(79), UiDensity::Compact);
        assert_eq!(UiDensity::from_width(80), UiDensity::Full);
        assert_eq!(UiDensity::from_width(120), UiDensity::Full);
    }

    #[test]
    fn test_layout_engine_creation() {
        let engine = LayoutEngine::new(80, 24);
        assert_eq!(engine.density, UiDensity::Full);
        assert!(engine.should_show_icons());

        let engine = LayoutEngine::new(20, 24);
        assert_eq!(engine.density, UiDensity::Ultra);
        assert!(!engine.should_show_icons());
    }

    #[test]
    fn test_tree_columns() {
        let area = Rect::new(0, 0, 80, 24);

        let cols = TreeColumns::new(UiDensity::Full, area.width);
        assert!(cols.show_icons);
        assert_eq!(cols.indent_width, 2);

        let cols = TreeColumns::new(UiDensity::Ultra, 20);
        assert!(!cols.show_icons);
        assert_eq!(cols.indent_width, 1);
    }

    #[test]
    fn test_status_layout() {
        let layout = StatusLayout::new(UiDensity::Full, 100);
        assert_eq!(layout.panel_count, 2);
        assert!(layout.show_file_info);

        let layout = StatusLayout::new(UiDensity::Ultra, 20);
        assert_eq!(layout.panel_count, 1);
        assert!(!layout.show_file_info);
    }

    #[test]
    fn test_split_ratio() {
        let engine = LayoutEngine::new(100, 24);
        assert_eq!(engine.split_ratio(true), (40, 60));
        assert_eq!(engine.split_ratio(false), (100, 0));

        let engine = LayoutEngine::new(20, 24);
        assert_eq!(engine.split_ratio(true), (100, 0)); // No preview in ultra
    }

    #[test]
    fn test_peek_preview_lines() {
        assert_eq!(UiDensity::Ultra.peek_preview_lines(), 2);
        assert_eq!(UiDensity::Narrow.peek_preview_lines(), 3);
        assert_eq!(UiDensity::Compact.peek_preview_lines(), 4);
        assert_eq!(UiDensity::Full.peek_preview_lines(), 5);
    }
}

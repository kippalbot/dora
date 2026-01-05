//! # Log Panel Widget
//!
//! A scrollable panel for displaying system log messages with Markdown rendering.
//! Ideal for real-time status updates, debug output, or chat history.
//!
//! ## Features
//!
//! - **Markdown Support**: Renders bold, italic, code, and other formatting
//! - **Auto-scroll**: Uses `ScrollYView` for overflow handling
//! - **Customizable Styling**: Font size and color via theme constants
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::log_panel::LogPanel;
//!
//!     MyScreen = <View> {
//!         log = <LogPanel> {
//!             width: Fill, height: 200
//!         }
//!     }
//! }
//! ```
//!
//! ## Updating Content
//!
//! Update the log content by setting the Markdown widget's text:
//!
//! ```rust,ignore
//! // Get the markdown widget and set its content
//! let markdown = self.view.markdown(ids!(log_scroll.log_content));
//! markdown.set_text("**Status**: Connected\n\n`12:34:56` Message received");
//!
//! // For appending, maintain a buffer and re-set
//! self.log_buffer.push_str(&format!("\n{}", new_message));
//! markdown.set_text(&self.log_buffer);
//! ```
//!
//! ## Dark Mode
//!
//! Apply dark mode colors to the Markdown draw components:
//!
//! ```rust,ignore
//! // Update text colors for dark mode
//! self.view.widget(ids!(log_scroll.log_content)).apply_over(cx, live!{
//!     draw_normal: { color: (vec4(0.95, 0.96, 0.98, 1.0)) }  // TEXT_PRIMARY_DARK
//!     draw_bold: { color: (vec4(0.95, 0.96, 0.98, 1.0)) }
//!     draw_fixed: { color: (vec4(0.58, 0.64, 0.72, 1.0)) }   // TEXT_SECONDARY_DARK
//! });
//! ```
//!
//! ## Layout
//!
//! The widget is structured as:
//! - `LogPanel` - Outer container with background
//!   - `log_scroll` - `ScrollYView` for overflow
//!     - `log_content` - `Markdown` widget for text

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import colors from theme
    use crate::theme::GRAY_50;
    use crate::theme::GRAY_600;

    pub LogPanel = {{LogPanel}} <View> {
        width: Fill, height: Fill
        show_bg: true
        draw_bg: { color: (GRAY_50) }

        log_scroll = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            padding: 8

            log_content = <Markdown> {
                width: Fill, height: Fit
                font_size: 10.0
                font_color: (GRAY_600)
                paragraph_spacing: 4

                draw_normal: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_bold: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_italic: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_fixed: {
                    text_style: {
                        font_size: 9.0
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct LogPanel {
    #[deref]
    view: View,
}

impl Widget for LogPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

//! # Buffer Gauge Widget
//!
//! A horizontal bar gauge showing fill level, with color change at threshold.
//! Useful for displaying buffer status, progress, or any 0-100% value.
//!
//! ## Features
//!
//! - **Dynamic Color**: Green below 80%, red above (configurable in shader)
//! - **Rounded Corners**: Smooth visual appearance
//! - **Border**: Subtle stroke for definition
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::led_gauge::BufferGauge;
//!
//!     MyScreen = <View> {
//!         buffer_indicator = <BufferGauge> {
//!             width: Fill, height: 40
//!         }
//!     }
//! }
//! ```
//!
//! ## Updating Fill Level
//!
//! Set the `fill_pct` instance variable (0.0 to 1.0):
//!
//! ```rust,ignore
//! // Update fill percentage (0.0 = empty, 1.0 = full)
//! self.view.view(ids!(buffer_indicator)).apply_over(cx, live!{
//!     draw_bg: { fill_pct: 0.65 }  // 65% full
//! });
//! self.view.redraw(cx);
//! ```
//!
//! ## Color Behavior
//!
//! The gauge changes color based on fill level:
//! - **0-80%**: Green (`GREEN_500`) - normal/safe
//! - **80-100%**: Red (`ACCENT_RED`) - warning/critical
//!
//! ## Customizing Colors
//!
//! Override the `get_fill_color` function in a derived widget:
//!
//! ```rust,ignore
//! live_design! {
//!     CustomGauge = <BufferGauge> {
//!         draw_bg: {
//!             fn get_fill_color(self, pct: float) -> vec4 {
//!                 // Custom: yellow above 50%, green below
//!                 if pct > 0.5 {
//!                     return vec4(0.95, 0.85, 0.2, 1.0);  // Yellow
//!                 } else {
//!                     return vec4(0.2, 0.8, 0.4, 1.0);   // Green
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Instance Variables
//!
//! | Variable | Range | Description |
//! |----------|-------|-------------|
//! | `fill_pct` | 0.0-1.0 | Fill percentage (0=empty, 1=full) |

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import colors from theme
    use crate::theme::ACCENT_RED;
    use crate::theme::GREEN_500;
    use crate::theme::GRAY_100;
    use crate::theme::GRAY_300;

    pub BufferGauge = {{BufferGauge}} <View> {
        width: Fill, height: 80
        show_bg: true

        draw_bg: {
            instance fill_pct: 0.0

            fn get_fill_color(self, pct: float) -> vec4 {
                // Red when above 80%, green otherwise
                if pct > 0.8 {
                    return (ACCENT_RED);
                } else {
                    return (GREEN_500);
                }
            }

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Light background
                sdf.box(4.0, 4.0, self.rect_size.x - 8.0, self.rect_size.y - 8.0, 3.0);
                sdf.fill((GRAY_100));

                // Fill bar
                let bar_width = (self.rect_size.x - 16.0) * self.fill_pct;
                if bar_width > 0.0 {
                    sdf.box(8.0, 8.0, bar_width, self.rect_size.y - 16.0, 2.0);
                    sdf.fill(self.get_fill_color(self.fill_pct));
                }

                // Border
                sdf.box(4.0, 4.0, self.rect_size.x - 8.0, self.rect_size.y - 8.0, 3.0);
                sdf.stroke((GRAY_300), 1.5);

                return sdf.result;
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct BufferGauge {
    #[deref]
    view: View,
}

impl Widget for BufferGauge {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

//! # Participant Panel Widget
//!
//! A composite widget showing participant status in voice chat/conference applications.
//! Combines a status indicator, name label, and 8-band audio waveform visualization.
//!
//! ## Features
//!
//! - **Status Indicator**: Colored dot showing participant state (waiting/speaking/error)
//! - **Name Label**: Participant name with dark mode support
//! - **Audio Waveform**: 8-band rainbow equalizer with level bar background
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::participant_panel::ParticipantPanel;
//!
//!     MyScreen = <View> {
//!         participant = <ParticipantPanel> {
//!             // Optional: set dark mode
//!             draw_bg: { dark_mode: 0.0 }
//!         }
//!     }
//! }
//! ```
//!
//! ## Updating at Runtime
//!
//! ### Status Indicator
//!
//! The status indicator supports 3 states:
//! - `0.0` = Blue (waiting/idle)
//! - `1.0` = Green (speaking)
//! - `2.0` = Red (error)
//!
//! ```rust,ignore
//! // Set status to "speaking"
//! self.view.view(ids!(participant.header.indicator)).apply_over(cx, live!{
//!     draw_bg: { status: 1.0 }
//! });
//! ```
//!
//! ### Waveform Levels
//!
//! Update the 8 frequency bands (band0-band7) with values from 0.0 to 1.0:
//!
//! ```rust,ignore
//! self.view.view(ids!(participant.waveform)).apply_over(cx, live!{
//!     draw_bg: {
//!         active: 1.0,  // Enable waveform display
//!         level: 0.5,   // Background level bar (0.0-1.0)
//!         band0: 0.3,
//!         band1: 0.5,
//!         band2: 0.8,
//!         // ... band3 through band7
//!     }
//! });
//! ```
//!
//! ### Dark Mode
//!
//! Use the `update_dark_mode` method on the widget ref:
//!
//! ```rust,ignore
//! self.ui.participant_panel(ids!(my_participant))
//!     .update_dark_mode(cx, 1.0);  // 0.0=light, 1.0=dark
//! ```
//!
//! ## Instance Variables
//!
//! | Variable | Widget | Range | Description |
//! |----------|--------|-------|-------------|
//! | `status` | StatusIndicator | 0/1/2 | Blue/Green/Red indicator |
//! | `dark_mode` | ParticipantPanel | 0.0-1.0 | Theme switching |
//! | `level` | ParticipantWaveform | 0.0-1.0 | Background level bar |
//! | `active` | ParticipantWaveform | 0/1 | Show/hide waveform bars |
//! | `band0`-`band7` | ParticipantWaveform | 0.0-1.0 | Frequency band levels |

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import colors from theme (single source of truth)
    use crate::theme::PANEL_BG;
    use crate::theme::PANEL_BG_DARK;
    use crate::theme::TEXT_PRIMARY;
    use crate::theme::TEXT_PRIMARY_DARK;
    use crate::theme::ACCENT_BLUE;
    use crate::theme::GREEN_500;
    use crate::theme::ACCENT_RED;
    use crate::theme::GRAY_200;
    use crate::theme::SLATE_600;

    // Status indicator with 3 states: 0=idle(blue), 1=speaking(green), 2=error(red)
    StatusIndicator = <View> {
        width: 12, height: 12
        show_bg: true
        draw_bg: {
            instance status: 0.0  // 0=waiting, 1=speaking, 2=error

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                let radius = min(center.x, center.y) - 0.5;

                sdf.circle(center.x, center.y, radius);

                // Blue=waiting, Green=speaking, Red=error
                let color = vec4(0.0, 0.0, 0.0, 1.0);
                if self.status < 0.5 {
                    color = (ACCENT_BLUE);  // Blue - waiting
                } else if self.status < 1.5 {
                    color = (GREEN_500);  // Green - speaking
                } else {
                    color = (ACCENT_RED);  // Red - error
                }
                sdf.fill(color);

                return sdf.result;
            }
        }
    }

    // Combined waveform with level bar background - 8 bars with 4 segments each
    ParticipantWaveform = <View> {
        width: Fill, height: 36
        show_bg: true
        draw_bg: {
            instance level: 0.0  // 0.0 - 1.0 for background level bar
            instance active: 0.0  // 0=inactive, 1=active
            instance dark_mode: 0.0
            instance band0: 0.0
            instance band1: 0.0
            instance band2: 0.0
            instance band3: 0.0
            instance band4: 0.0
            instance band5: 0.0
            instance band6: 0.0
            instance band7: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Background adapts to dark mode
                let bg = mix((GRAY_200), (SLATE_600), self.dark_mode);
                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                sdf.fill(bg);

                // Background level bar (behind waveform)
                let level_width = self.rect_size.x * self.level;
                if level_width > 0.0 {
                    sdf.box(0.0, 0.0, level_width, self.rect_size.y, 2.0);
                    // Gradient based on level: green -> yellow -> red
                    let r = mix(0.2, 0.95, smoothstep(0.4, 0.9, self.level));
                    let g = mix(0.75, 0.35, smoothstep(0.6, 1.0, self.level));
                    let b = mix(0.3, 0.2, smoothstep(0.0, 0.5, self.level));
                    sdf.fill(vec4(r, g, b, 0.35));
                }

                // Only show waveform bars when active
                if self.active > 0.5 {
                    let num_bars = 8.0;
                    let num_segs = 4.0;
                    let gap = 2.0;
                    let seg_gap = 1.5;
                    let bar_width = (self.rect_size.x - gap * (num_bars + 1.0)) / num_bars;
                    let seg_height = (self.rect_size.y - 4.0 - seg_gap * (num_segs - 1.0)) / num_segs;
                    let dim = vec4(0.82, 0.82, 0.85, 1.0);

                    // Rainbow neon colors
                    let c0 = vec4(0.9, 0.25, 0.3, 0.95);   // Red
                    let c1 = vec4(0.95, 0.5, 0.2, 0.95);   // Orange
                    let c2 = vec4(0.95, 0.85, 0.2, 0.95);  // Yellow
                    let c3 = vec4(0.3, 0.90, 0.4, 0.95);   // Green
                    let c4 = vec4(0.2, 0.90, 0.90, 0.95);  // Cyan
                    let c5 = vec4(0.3, 0.5, 0.98, 0.95);   // Blue
                    let c6 = vec4(0.6, 0.35, 0.95, 0.95);  // Purple
                    let c7 = vec4(0.95, 0.4, 0.75, 0.95);  // Pink

                    // Band 0
                    let l0 = self.band0 * num_segs;
                    let x0 = gap;
                    sdf.box(x0, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c0, step(0.5, l0)));
                    sdf.box(x0, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c0, step(1.5, l0)));
                    sdf.box(x0, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c0, step(2.5, l0)));
                    sdf.box(x0, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c0, step(3.5, l0)));

                    // Band 1
                    let l1 = self.band1 * num_segs;
                    let x1 = gap + (bar_width + gap);
                    sdf.box(x1, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c1, step(0.5, l1)));
                    sdf.box(x1, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c1, step(1.5, l1)));
                    sdf.box(x1, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c1, step(2.5, l1)));
                    sdf.box(x1, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c1, step(3.5, l1)));

                    // Band 2
                    let l2 = self.band2 * num_segs;
                    let x2 = gap + 2.0 * (bar_width + gap);
                    sdf.box(x2, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c2, step(0.5, l2)));
                    sdf.box(x2, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c2, step(1.5, l2)));
                    sdf.box(x2, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c2, step(2.5, l2)));
                    sdf.box(x2, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c2, step(3.5, l2)));

                    // Band 3
                    let l3 = self.band3 * num_segs;
                    let x3 = gap + 3.0 * (bar_width + gap);
                    sdf.box(x3, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c3, step(0.5, l3)));
                    sdf.box(x3, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c3, step(1.5, l3)));
                    sdf.box(x3, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c3, step(2.5, l3)));
                    sdf.box(x3, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c3, step(3.5, l3)));

                    // Band 4
                    let l4 = self.band4 * num_segs;
                    let x4 = gap + 4.0 * (bar_width + gap);
                    sdf.box(x4, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c4, step(0.5, l4)));
                    sdf.box(x4, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c4, step(1.5, l4)));
                    sdf.box(x4, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c4, step(2.5, l4)));
                    sdf.box(x4, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c4, step(3.5, l4)));

                    // Band 5
                    let l5 = self.band5 * num_segs;
                    let x5 = gap + 5.0 * (bar_width + gap);
                    sdf.box(x5, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c5, step(0.5, l5)));
                    sdf.box(x5, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c5, step(1.5, l5)));
                    sdf.box(x5, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c5, step(2.5, l5)));
                    sdf.box(x5, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c5, step(3.5, l5)));

                    // Band 6
                    let l6 = self.band6 * num_segs;
                    let x6 = gap + 6.0 * (bar_width + gap);
                    sdf.box(x6, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c6, step(0.5, l6)));
                    sdf.box(x6, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c6, step(1.5, l6)));
                    sdf.box(x6, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c6, step(2.5, l6)));
                    sdf.box(x6, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c6, step(3.5, l6)));

                    // Band 7
                    let l7 = self.band7 * num_segs;
                    let x7 = gap + 7.0 * (bar_width + gap);
                    sdf.box(x7, self.rect_size.y - 2.0 - seg_height, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c7, step(0.5, l7)));
                    sdf.box(x7, self.rect_size.y - 2.0 - 2.0 * seg_height - seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c7, step(1.5, l7)));
                    sdf.box(x7, self.rect_size.y - 2.0 - 3.0 * seg_height - 2.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c7, step(2.5, l7)));
                    sdf.box(x7, self.rect_size.y - 2.0 - 4.0 * seg_height - 3.0 * seg_gap, bar_width, seg_height, 1.0);
                    sdf.fill(mix(dim, c7, step(3.5, l7)));
                }

                return sdf.result;
            }
        }
    }

    pub ParticipantPanel = {{ParticipantPanel}} <RoundedView> {
        width: Fill, height: Fit
        padding: 6
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 2.0
            fn get_color(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }
        }
        flow: Down
        spacing: 4

        // Top row: indicator + name
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 6
            align: {y: 0.5}

            indicator = <StatusIndicator> {}

            name_label = <Label> {
                text: "Participant"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 11.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Waveform with level bar background
        waveform = <ParticipantWaveform> {}
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ParticipantPanel {
    #[deref]
    view: View,
}

impl Widget for ParticipantPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ParticipantPanelRef {
    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Panel background
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Name label
            inner.view.label(ids!(header.name_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Waveform background
            inner.view.view(ids!(waveform)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }
}

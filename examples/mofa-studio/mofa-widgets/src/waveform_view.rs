//! # Waveform View Widget
//!
//! A standalone FFT-style frequency bar visualization with smooth animation.
//! Displays 8 rainbow-colored bars representing frequency bands.
//!
//! ## Features
//!
//! - **8-Band Spectrum**: Rainbow colors (red→orange→yellow→green→cyan→blue→purple→pink)
//! - **Smooth Animation**: Uses `NextFrame` event for interpolated level changes
//! - **Dark Background**: Uses SLATE_950 for optimal contrast
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::waveform_view::WaveformView;
//!
//!     MyScreen = <View> {
//!         waveform = <WaveformView> {
//!             width: 200, height: 100
//!         }
//!     }
//! }
//! ```
//!
//! ## Animation Integration
//!
//! The widget automatically handles animation via `NextFrame`. Start animation with:
//!
//! ```rust,ignore
//! // In your widget's after_new_from_doc
//! cx.start_interval(0.05);  // 20 FPS refresh
//!
//! // In handle_event, trigger NextFrame
//! if let Event::Timer(_) = event {
//!     cx.request_next_frame();
//! }
//! ```
//!
//! ## Updating Band Levels
//!
//! The widget uses target levels for smooth interpolation. Update via `apply_over`:
//!
//! ```rust,ignore
//! // Direct shader update (immediate)
//! self.view.view(ids!(my_waveform)).apply_over(cx, live!{
//!     draw_bg: {
//!         amplitude: 0.5,
//!         band0: 0.3,
//!         band1: 0.5,
//!         band2: 0.8,
//!         band3: 0.6,
//!         band4: 0.5,
//!         band5: 0.4,
//!         band6: 0.3,
//!         band7: 0.2,
//!     }
//! });
//! ```
//!
//! ## Instance Variables
//!
//! | Variable | Range | Description |
//! |----------|-------|-------------|
//! | `anim_time` | auto | Animation time (set by NextFrame) |
//! | `amplitude` | 0.0-1.0 | Global amplitude multiplier |
//! | `band0`-`band7` | 0.0-1.0 | Individual frequency band levels |
//!
//! ## Internal State
//!
//! The widget maintains internal state for smooth animation:
//! - `band_levels[8]` - Current displayed levels
//! - `target_levels[8]` - Target levels (interpolated towards)
//!
//! Attack is faster (0.3) than decay (0.1) for natural audio response.

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import colors from theme
    use crate::theme::SLATE_950;

    pub WaveformView = {{WaveformView}} <View> {
        width: Fill, height: Fill
        show_bg: true

        draw_bg: {
            instance anim_time: 0.0
            instance amplitude: 0.5

            // Frequency band levels (8 bands)
            instance band0: 0.3
            instance band1: 0.4
            instance band2: 0.5
            instance band3: 0.6
            instance band4: 0.5
            instance band5: 0.4
            instance band6: 0.3
            instance band7: 0.2

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Dark background
                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                sdf.fill((SLATE_950));

                // Bar dimensions
                let num_bars = 8.0;
                let gap = 4.0;
                let bar_width = (self.rect_size.x - gap * (num_bars + 1.0)) / num_bars;
                let max_height = self.rect_size.y - 4.0;
                let amp = self.amplitude * 2.0;

                // Colors for each frequency (rainbow: red->orange->yellow->green->cyan->blue->purple->pink)
                let c0 = vec3(0.9, 0.25, 0.3);   // Red
                let c1 = vec3(0.95, 0.5, 0.2);   // Orange
                let c2 = vec3(0.95, 0.85, 0.2);  // Yellow
                let c3 = vec3(0.3, 0.85, 0.4);   // Green
                let c4 = vec3(0.2, 0.85, 0.85);  // Cyan
                let c5 = vec3(0.3, 0.5, 0.95);   // Blue
                let c6 = vec3(0.6, 0.35, 0.9);   // Purple
                let c7 = vec3(0.9, 0.4, 0.7);    // Pink

                // Band 0
                let x0 = gap;
                let l0 = clamp(self.band0 * amp + sin(self.anim_time * 2.0) * 0.02, 0.05, 1.0);
                let h0 = max_height * l0;
                sdf.box(x0, self.rect_size.y - h0 - 2.0, bar_width, h0, 2.0);
                sdf.fill(vec4(c0, 0.9));

                // Band 1
                let x1 = gap + (bar_width + gap);
                let l1 = clamp(self.band1 * amp + sin(self.anim_time * 2.0 + 0.4) * 0.02, 0.05, 1.0);
                let h1 = max_height * l1;
                sdf.box(x1, self.rect_size.y - h1 - 2.0, bar_width, h1, 2.0);
                sdf.fill(vec4(c1, 0.9));

                // Band 2
                let x2 = gap + 2.0 * (bar_width + gap);
                let l2 = clamp(self.band2 * amp + sin(self.anim_time * 2.0 + 0.8) * 0.02, 0.05, 1.0);
                let h2 = max_height * l2;
                sdf.box(x2, self.rect_size.y - h2 - 2.0, bar_width, h2, 2.0);
                sdf.fill(vec4(c2, 0.9));

                // Band 3
                let x3 = gap + 3.0 * (bar_width + gap);
                let l3 = clamp(self.band3 * amp + sin(self.anim_time * 2.0 + 1.2) * 0.02, 0.05, 1.0);
                let h3 = max_height * l3;
                sdf.box(x3, self.rect_size.y - h3 - 2.0, bar_width, h3, 2.0);
                sdf.fill(vec4(c3, 0.9));

                // Band 4
                let x4 = gap + 4.0 * (bar_width + gap);
                let l4 = clamp(self.band4 * amp + sin(self.anim_time * 2.0 + 1.6) * 0.02, 0.05, 1.0);
                let h4 = max_height * l4;
                sdf.box(x4, self.rect_size.y - h4 - 2.0, bar_width, h4, 2.0);
                sdf.fill(vec4(c4, 0.9));

                // Band 5
                let x5 = gap + 5.0 * (bar_width + gap);
                let l5 = clamp(self.band5 * amp + sin(self.anim_time * 2.0 + 2.0) * 0.02, 0.05, 1.0);
                let h5 = max_height * l5;
                sdf.box(x5, self.rect_size.y - h5 - 2.0, bar_width, h5, 2.0);
                sdf.fill(vec4(c5, 0.9));

                // Band 6
                let x6 = gap + 6.0 * (bar_width + gap);
                let l6 = clamp(self.band6 * amp + sin(self.anim_time * 2.0 + 2.4) * 0.02, 0.05, 1.0);
                let h6 = max_height * l6;
                sdf.box(x6, self.rect_size.y - h6 - 2.0, bar_width, h6, 2.0);
                sdf.fill(vec4(c6, 0.9));

                // Band 7
                let x7 = gap + 7.0 * (bar_width + gap);
                let l7 = clamp(self.band7 * amp + sin(self.anim_time * 2.0 + 2.8) * 0.02, 0.05, 1.0);
                let h7 = max_height * l7;
                sdf.box(x7, self.rect_size.y - h7 - 2.0, bar_width, h7, 2.0);
                sdf.fill(vec4(c7, 0.9));

                return sdf.result;
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct WaveformView {
    #[deref]
    view: View,

    #[rust]
    animator_time: f64,

    #[rust]
    band_levels: [f32; 8],

    #[rust]
    target_levels: [f32; 8],
}

impl Widget for WaveformView {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if let Event::NextFrame(nf) = event {
            self.animator_time = nf.time;

            for i in 0..8 {
                let diff = self.target_levels[i] - self.band_levels[i];
                if diff.abs() > 0.01 {
                    let speed = if diff > 0.0 { 0.3 } else { 0.1 };
                    self.band_levels[i] += diff * speed;
                }
            }

            self.view.apply_over(cx, live! {
                draw_bg: {
                    anim_time: (self.animator_time),
                    band0: (self.band_levels[0] as f64),
                    band1: (self.band_levels[1] as f64),
                    band2: (self.band_levels[2] as f64),
                    band3: (self.band_levels[3] as f64),
                    band4: (self.band_levels[4] as f64),
                    band5: (self.band_levels[5] as f64),
                    band6: (self.band_levels[6] as f64),
                    band7: (self.band_levels[7] as f64),
                }
            });
            self.view.redraw(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

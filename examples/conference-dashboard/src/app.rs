//! Makepad Application setup for Conference Dashboard

use makepad_widgets::*;
use crate::{SharedStateRef, widgets};
use std::sync::OnceLock;

/// Extension trait for Event to easily get actions
trait EventExt {
    fn actions(&self) -> &[Action];
}

impl EventExt for Event {
    fn actions(&self) -> &[Action] {
        match self {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        }
    }
}

// Global static for shared state - accessible across threads
static SHARED_STATE: OnceLock<SharedStateRef> = OnceLock::new();

/// Set the shared state before starting the app
pub fn set_shared_state(state: SharedStateRef) {
    SHARED_STATE.set(state).ok();
}

/// Get a clone of the shared state
fn get_shared_state() -> Option<SharedStateRef> {
    SHARED_STATE.get().cloned()
}

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::widgets::participant_panel::ParticipantPanel;
    use crate::widgets::log_panel::LogPanel;

    // Font definitions with Chinese and Emoji support
    FONT_REGULAR = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Regular.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    FONT_MEDIUM = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Medium.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    FONT_SEMIBOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-SemiBold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    FONT_BOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Bold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }

    // Logo image
    MOFA_LOGO = dep("crate://self/mofa-logo.png")

    // Light color palette
    DARK_BG = #f5f7fa
    PANEL_BG = #ffffff
    ACCENT_BLUE = #3b82f6
    ACCENT_GREEN = #10b981
    TEXT_PRIMARY = #1f2937
    TEXT_SECONDARY = #6b7280

    // Main Dashboard Layout - Horizontal split with foldable log panel
    Dashboard = {{Dashboard}} <View> {
        width: Fill, height: Fill
        flow: Right
        show_bg: true
        draw_bg: { color: (DARK_BG) }

        // Left side - Main content
        main_content = <View> {
            width: Fill, height: Fill
            flow: Down
            padding: 20

            // Header
            header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12
                align: {y: 0.5}
                padding: {bottom: 20}

                // Logo
                logo = <Image> {
                    width: 40, height: 40
                    source: (MOFA_LOGO)
                }

                title = <Label> {
                    text: "MoFA FM"
                    draw_text: {
                        color: (TEXT_PRIMARY)
                        text_style: <FONT_BOLD>{ font_size: 24.0 }
                    }
                }

                <View> { width: Fill, height: 1 }

                // Blinking dot for connected status
                connection_dot = <RoundedView> {
                    width: 12, height: 12
                    margin: {right: 8}
                    draw_bg: {
                        color: #9ca3af
                        border_radius: 6.0
                    }
                }

                status_indicator = <RoundedView> {
                    width: Fit, height: Fit
                    padding: {left: 12, right: 12, top: 6, bottom: 6}
                    draw_bg: {
                        color: #9ca3af
                        border_radius: 6.0
                    }
                    status_label = <Label> {
                        text: "DEMO"
                        draw_text: {
                            color: #fff
                            text_style: <FONT_SEMIBOLD>{ font_size: 12.0 }
                        }
                    }
                }
            }

            // Main content area
            content = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 12

            // Top row - Waveform/Buffer stacked + Control buttons (horizontal)
            control_bar = <View> {
                width: Fill, height: 70
                flow: Right
                spacing: 12

                // Buffer status LED bar (neon blue to red)
                buffer_section = <RoundedView> {
                    width: 180, height: Fill
                    padding: 6
                    draw_bg: {
                        color: #f0f0f5
                        border_radius: 2.0
                    }
                    flow: Down
                    spacing: 4
                    align: {x: 0.5, y: 0.5}

                    // Header with status dot and label
                    buffer_header = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 6
                        align: {x: 0.5, y: 0.5}

                        // Status dot: green < 80%, red >= 80%
                        buffer_status_dot = <View> {
                            width: 10, height: 10
                            show_bg: true
                            draw_bg: {
                                instance critical: 0.0  // 0=good(green), 1=critical(red)

                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let center = self.rect_size * 0.5;
                                    let radius = min(center.x, center.y) - 0.5;
                                    sdf.circle(center.x, center.y, radius);
                                    let green = vec4(0.13, 0.77, 0.37, 1.0);
                                    let red = vec4(0.95, 0.25, 0.25, 1.0);
                                    sdf.fill(mix(green, red, self.critical));
                                    return sdf.result;
                                }
                            }
                        }

                        buffer_label = <Label> {
                            text: "Buffer"
                            draw_text: {
                                color: #374151
                                text_style: { font_size: 10.0 }
                            }
                        }
                    }

                    // LED bar buffer gauge (10 segments, blue to red)
                    buffer_gauge = <View> {
                        width: Fill, height: 20
                        show_bg: true
                        draw_bg: {
                            instance fill_pct: 0.0

                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                                // Light background
                                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                                sdf.fill(#e5e7eb);

                                let num_segs = 10.0;
                                let gap = 2.0;
                                let seg_width = (self.rect_size.x - gap * (num_segs + 1.0)) / num_segs;
                                let seg_height = self.rect_size.y - 4.0;
                                let active_segs = self.fill_pct * num_segs;
                                let dim = vec4(0.85, 0.85, 0.88, 1.0);

                                // Neon colors: blue -> cyan -> green -> yellow -> orange -> red
                                let c0 = vec4(0.2, 0.4, 0.95, 1.0);   // Blue
                                let c1 = vec4(0.2, 0.55, 0.95, 1.0);  // Blue-cyan
                                let c2 = vec4(0.2, 0.75, 0.90, 1.0);  // Cyan
                                let c3 = vec4(0.2, 0.85, 0.70, 1.0);  // Cyan-green
                                let c4 = vec4(0.3, 0.85, 0.45, 1.0);  // Green
                                let c5 = vec4(0.6, 0.85, 0.3, 1.0);   // Green-yellow
                                let c6 = vec4(0.90, 0.80, 0.2, 1.0);  // Yellow
                                let c7 = vec4(0.95, 0.60, 0.2, 1.0);  // Orange
                                let c8 = vec4(0.95, 0.4, 0.2, 1.0);   // Orange-red
                                let c9 = vec4(0.95, 0.2, 0.2, 1.0);   // Red

                                // Draw 10 segments
                                let x0 = gap;
                                sdf.box(x0, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c0, step(0.5, active_segs)));

                                let x1 = gap + (seg_width + gap);
                                sdf.box(x1, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c1, step(1.5, active_segs)));

                                let x2 = gap + 2.0 * (seg_width + gap);
                                sdf.box(x2, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c2, step(2.5, active_segs)));

                                let x3 = gap + 3.0 * (seg_width + gap);
                                sdf.box(x3, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c3, step(3.5, active_segs)));

                                let x4 = gap + 4.0 * (seg_width + gap);
                                sdf.box(x4, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c4, step(4.5, active_segs)));

                                let x5 = gap + 5.0 * (seg_width + gap);
                                sdf.box(x5, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c5, step(5.5, active_segs)));

                                let x6 = gap + 6.0 * (seg_width + gap);
                                sdf.box(x6, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c6, step(6.5, active_segs)));

                                let x7 = gap + 7.0 * (seg_width + gap);
                                sdf.box(x7, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c7, step(7.5, active_segs)));

                                let x8 = gap + 8.0 * (seg_width + gap);
                                sdf.box(x8, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c8, step(8.5, active_segs)));

                                let x9 = gap + 9.0 * (seg_width + gap);
                                sdf.box(x9, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c9, step(9.5, active_segs)));

                                return sdf.result;
                            }
                        }
                    }

                    buffer_pct_label = <Label> {
                        text: "0%"
                        draw_text: {
                            color: #374151
                            text_style: { font_size: 10.0 }
                        }
                    }
                }

                // CPU usage LED bar
                cpu_section = <RoundedView> {
                    width: 180, height: Fill
                    padding: 6
                    draw_bg: {
                        color: #f0f0f5
                        border_radius: 2.0
                    }
                    flow: Down
                    spacing: 4
                    align: {x: 0.5, y: 0.5}

                    cpu_header = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 6
                        align: {x: 0.5, y: 0.5}

                        cpu_status_dot = <View> {
                            width: 10, height: 10
                            show_bg: true
                            draw_bg: {
                                instance critical: 0.0

                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let center = self.rect_size * 0.5;
                                    let radius = min(center.x, center.y) - 0.5;
                                    sdf.circle(center.x, center.y, radius);
                                    let green = vec4(0.13, 0.77, 0.37, 1.0);
                                    let red = vec4(0.95, 0.25, 0.25, 1.0);
                                    sdf.fill(mix(green, red, self.critical));
                                    return sdf.result;
                                }
                            }
                        }

                        cpu_label = <Label> {
                            text: "CPU"
                            draw_text: {
                                color: #374151
                                text_style: { font_size: 10.0 }
                            }
                        }
                    }

                    cpu_gauge = <View> {
                        width: Fill, height: 20
                        show_bg: true
                        draw_bg: {
                            instance fill_pct: 0.0

                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                                sdf.fill(#e5e7eb);

                                let num_segs = 10.0;
                                let gap = 2.0;
                                let seg_width = (self.rect_size.x - gap * (num_segs + 1.0)) / num_segs;
                                let seg_height = self.rect_size.y - 4.0;
                                let active_segs = self.fill_pct * num_segs;
                                let dim = vec4(0.85, 0.85, 0.88, 1.0);

                                let c0 = vec4(0.2, 0.4, 0.95, 1.0);
                                let c1 = vec4(0.2, 0.55, 0.95, 1.0);
                                let c2 = vec4(0.2, 0.75, 0.90, 1.0);
                                let c3 = vec4(0.2, 0.85, 0.70, 1.0);
                                let c4 = vec4(0.3, 0.85, 0.45, 1.0);
                                let c5 = vec4(0.6, 0.85, 0.3, 1.0);
                                let c6 = vec4(0.90, 0.80, 0.2, 1.0);
                                let c7 = vec4(0.95, 0.60, 0.2, 1.0);
                                let c8 = vec4(0.95, 0.4, 0.2, 1.0);
                                let c9 = vec4(0.95, 0.2, 0.2, 1.0);

                                let x0 = gap;
                                sdf.box(x0, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c0, step(0.5, active_segs)));
                                let x1 = gap + (seg_width + gap);
                                sdf.box(x1, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c1, step(1.5, active_segs)));
                                let x2 = gap + 2.0 * (seg_width + gap);
                                sdf.box(x2, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c2, step(2.5, active_segs)));
                                let x3 = gap + 3.0 * (seg_width + gap);
                                sdf.box(x3, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c3, step(3.5, active_segs)));
                                let x4 = gap + 4.0 * (seg_width + gap);
                                sdf.box(x4, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c4, step(4.5, active_segs)));
                                let x5 = gap + 5.0 * (seg_width + gap);
                                sdf.box(x5, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c5, step(5.5, active_segs)));
                                let x6 = gap + 6.0 * (seg_width + gap);
                                sdf.box(x6, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c6, step(6.5, active_segs)));
                                let x7 = gap + 7.0 * (seg_width + gap);
                                sdf.box(x7, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c7, step(7.5, active_segs)));
                                let x8 = gap + 8.0 * (seg_width + gap);
                                sdf.box(x8, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c8, step(8.5, active_segs)));
                                let x9 = gap + 9.0 * (seg_width + gap);
                                sdf.box(x9, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c9, step(9.5, active_segs)));

                                return sdf.result;
                            }
                        }
                    }

                    cpu_pct_label = <Label> {
                        text: "0%"
                        draw_text: {
                            color: #374151
                            text_style: { font_size: 10.0 }
                        }
                    }
                }

                // Memory usage LED bar
                memory_section = <RoundedView> {
                    width: 180, height: Fill
                    padding: 6
                    draw_bg: {
                        color: #f0f0f5
                        border_radius: 2.0
                    }
                    flow: Down
                    spacing: 4
                    align: {x: 0.5, y: 0.5}

                    memory_header = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 6
                        align: {x: 0.5, y: 0.5}

                        memory_status_dot = <View> {
                            width: 10, height: 10
                            show_bg: true
                            draw_bg: {
                                instance critical: 0.0

                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let center = self.rect_size * 0.5;
                                    let radius = min(center.x, center.y) - 0.5;
                                    sdf.circle(center.x, center.y, radius);
                                    let green = vec4(0.13, 0.77, 0.37, 1.0);
                                    let red = vec4(0.95, 0.25, 0.25, 1.0);
                                    sdf.fill(mix(green, red, self.critical));
                                    return sdf.result;
                                }
                            }
                        }

                        memory_label = <Label> {
                            text: "Memory"
                            draw_text: {
                                color: #374151
                                text_style: { font_size: 10.0 }
                            }
                        }
                    }

                    memory_gauge = <View> {
                        width: Fill, height: 20
                        show_bg: true
                        draw_bg: {
                            instance fill_pct: 0.0

                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                                sdf.fill(#e5e7eb);

                                let num_segs = 10.0;
                                let gap = 2.0;
                                let seg_width = (self.rect_size.x - gap * (num_segs + 1.0)) / num_segs;
                                let seg_height = self.rect_size.y - 4.0;
                                let active_segs = self.fill_pct * num_segs;
                                let dim = vec4(0.85, 0.85, 0.88, 1.0);

                                let c0 = vec4(0.2, 0.4, 0.95, 1.0);
                                let c1 = vec4(0.2, 0.55, 0.95, 1.0);
                                let c2 = vec4(0.2, 0.75, 0.90, 1.0);
                                let c3 = vec4(0.2, 0.85, 0.70, 1.0);
                                let c4 = vec4(0.3, 0.85, 0.45, 1.0);
                                let c5 = vec4(0.6, 0.85, 0.3, 1.0);
                                let c6 = vec4(0.90, 0.80, 0.2, 1.0);
                                let c7 = vec4(0.95, 0.60, 0.2, 1.0);
                                let c8 = vec4(0.95, 0.4, 0.2, 1.0);
                                let c9 = vec4(0.95, 0.2, 0.2, 1.0);

                                let x0 = gap;
                                sdf.box(x0, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c0, step(0.5, active_segs)));
                                let x1 = gap + (seg_width + gap);
                                sdf.box(x1, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c1, step(1.5, active_segs)));
                                let x2 = gap + 2.0 * (seg_width + gap);
                                sdf.box(x2, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c2, step(2.5, active_segs)));
                                let x3 = gap + 3.0 * (seg_width + gap);
                                sdf.box(x3, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c3, step(3.5, active_segs)));
                                let x4 = gap + 4.0 * (seg_width + gap);
                                sdf.box(x4, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c4, step(4.5, active_segs)));
                                let x5 = gap + 5.0 * (seg_width + gap);
                                sdf.box(x5, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c5, step(5.5, active_segs)));
                                let x6 = gap + 6.0 * (seg_width + gap);
                                sdf.box(x6, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c6, step(6.5, active_segs)));
                                let x7 = gap + 7.0 * (seg_width + gap);
                                sdf.box(x7, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c7, step(7.5, active_segs)));
                                let x8 = gap + 8.0 * (seg_width + gap);
                                sdf.box(x8, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c8, step(8.5, active_segs)));
                                let x9 = gap + 9.0 * (seg_width + gap);
                                sdf.box(x9, 2.0, seg_width, seg_height, 2.0);
                                sdf.fill(mix(dim, c9, step(9.5, active_segs)));

                                return sdf.result;
                            }
                        }
                    }

                    memory_pct_label = <Label> {
                        text: "0%"
                        draw_text: {
                            color: #374151
                            text_style: { font_size: 10.0 }
                        }
                    }
                }
            }

            // Second row - Participant status cards (horizontal, compact)
            participant_bar = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12

                // Participant 1 status card
                student1_panel = <ParticipantPanel> {
                    width: Fill, height: Fit
                }

                // Participant 2 status card
                student2_panel = <ParticipantPanel> {
                    width: Fill, height: Fit
                }

                // Tutor status card
                tutor_panel = <ParticipantPanel> {
                    width: Fill, height: Fit
                }
            }

            // Chat window - conversation history (moly-style)
            chat_section = <RoundedView> {
                width: Fill, height: Fill
                draw_bg: {
                    color: (PANEL_BG)
                    border_radius: 4.0
                }
                flow: Down

                // Chat header
                <View> {
                    width: Fill, height: 40
                    padding: {left: 12, right: 12}
                    align: {y: 0.5}
                    show_bg: true
                    draw_bg: { color: #f0f2f5 }

                    <Label> {
                        text: "Chat History"
                        draw_text: {
                            color: (TEXT_PRIMARY)
                            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        }
                    }
                }

                // Chat messages area (scrollable)
                chat_scroll = <ScrollYView> {
                    width: Fill, height: Fill
                    flow: Down
                    scroll_bars: <ScrollBars> {
                        show_scroll_x: false
                        show_scroll_y: true
                    }

                    <View> {
                        width: Fill, height: Fit
                        padding: 12
                        flow: Down

                        chat_content = <Markdown> {
                            width: Fill, height: Fit
                            font_size: 13.0
                            font_color: #1f2937
                            paragraph_spacing: 8

                            draw_normal: {
                                text_style: <FONT_REGULAR>{ font_size: 13.0 }
                            }
                            draw_bold: {
                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                            }
                        }
                    }
                }
            }

            // Prompt input area (bottom)
            prompt_section = <RoundedView> {
                width: Fill, height: Fit
                padding: 12
                draw_bg: {
                    color: (PANEL_BG)
                    border_radius: 4.0
                }
                flow: Down
                spacing: 8

                <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 10

                    prompt_input = <TextInput> {
                        width: Fill, height: 40
                        empty_text: "Enter prompt to send to tutor..."
                        draw_bg: {
                            color: #ffffff
                        }
                        draw_text: {
                            color: #1f2937
                            uniform color_hover: #1f2937
                            uniform color_focus: #1f2937
                            uniform color_down: #1f2937
                            uniform color_disabled: #9ca3af
                            uniform color_empty: #9ca3af
                            uniform color_empty_hover: #6b7280
                            uniform color_empty_focus: #6b7280
                            text_style: <FONT_REGULAR>{ font_size: 14.0 }
                        }
                        draw_cursor: {
                            color: #1f2937
                        }
                        draw_selection: {
                            color: #bfdbfe
                        }
                    }

                    send_prompt_btn = <Button> {
                        width: 80, height: 40
                        text: "Send"
                        draw_text: {
                            color: #ffffff
                            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        }
                        draw_bg: {
                            instance pressed: 0.0
                            instance hover: 0.0
                            uniform color_normal: (ACCENT_BLUE)
                            uniform color_pressed: #1d4ed8
                            uniform color_hover: #2563ff
                            border_radius: 6.0

                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let color = mix(self.color_normal, self.color_hover, self.hover);
                                let color = mix(color, self.color_pressed, self.pressed);
                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                sdf.fill(color);
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward {duration: 0.15}} apply: {draw_bg: {hover: 0.0}} }
                                on = { from: {all: Forward {duration: 0.15}} apply: {draw_bg: {hover: 1.0}} }
                            }
                            pressed = {
                                default: off
                                off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {pressed: 0.0}} }
                                on = { from: {all: Forward {duration: 0.05}} apply: {draw_bg: {pressed: 1.0}} }
                            }
                        }
                    }

                    reset_conf_btn = <Button> {
                        width: 60, height: 40
                        text: "Reset"
                        draw_text: {
                            color: #4b5563
                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                        }
                        draw_bg: {
                            instance pressed: 0.0
                            instance hover: 0.0
                            uniform color_normal: #e5e7eb
                            uniform color_pressed: #9ca3af
                            uniform color_hover: #d1d5db
                            border_radius: 6.0

                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let color = mix(self.color_normal, self.color_hover, self.hover);
                                let color = mix(color, self.color_pressed, self.pressed);
                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                sdf.fill(color);
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward {duration: 0.15}} apply: {draw_bg: {hover: 0.0}} }
                                on = { from: {all: Forward {duration: 0.15}} apply: {draw_bg: {hover: 1.0}} }
                            }
                            pressed = {
                                default: off
                                off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {pressed: 0.0}} }
                                on = { from: {all: Forward {duration: 0.05}} apply: {draw_bg: {pressed: 1.0}} }
                            }
                        }
                    }
                }
            }
            } // end content
        } // end main_content

        // Draggable splitter between main content and log panel
        splitter = <View> {
            width: 6, height: Fill
            show_bg: true
            draw_bg: { color: #d1d5db }
            cursor: ColResize
        }

        // Right side - Foldable System Logs Panel
        log_panel = <View> {
            width: 350, height: Fill
            flow: Right
            show_bg: true
            draw_bg: { color: #f0f2f5 }

            // Toggle button column (always visible)
            toggle_column = <View> {
                width: 32, height: Fill
                show_bg: true
                draw_bg: { color: #e5e7eb }
                align: {x: 0.5, y: 0.0}
                padding: {top: 8}

                toggle_log_btn = <Button> {
                    width: 24, height: 24
                    text: ">"
                    draw_text: {
                        color: #4b5563
                        text_style: <FONT_BOLD>{ font_size: 12.0 }
                    }
                    draw_bg: {
                        color: #d1d5db
                        border_radius: 4.0
                    }
                }
            }

            // Log content column (can be hidden)
            log_content_column = <View> {
                width: Fill, height: Fill
                flow: Down

                // Header with filters
                log_header = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    show_bg: true
                    draw_bg: { color: #e5e7eb }

                    // Title row
                    log_title_row = <View> {
                        width: Fill, height: 32
                        flow: Right
                        align: {y: 0.5}
                        padding: {left: 12, right: 8}

                        log_title = <Label> {
                            text: "System Logs"
                            draw_text: {
                                color: (TEXT_PRIMARY)
                                text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                            }
                        }
                    }

                    // Filter row
                    log_filter_row = <View> {
                        width: Fill, height: 32
                        flow: Right
                        align: {y: 0.5}
                        padding: {left: 8, right: 8, bottom: 4}
                        spacing: 6

                        // Level filter dropdown
                        level_filter = <DropDown> {
                            width: 70, height: 24
                            popup_menu_position: BelowInput
                            draw_text: {
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                color: #374151
                            }
                            labels: ["ALL", "DEBUG", "INFO", "WARN", "ERROR"]
                            values: [ALL, DEBUG, INFO, WARN, ERROR]
                        }

                        // Node filter dropdown
                        node_filter = <DropDown> {
                            width: 90, height: 24
                            popup_menu_position: BelowInput
                            draw_text: {
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                color: #374151
                            }
                            labels: ["All Nodes", "Daniu", "Yifei", "Laoshi", "Bridge", "Controller", "Segmenter", "TTS", "Dashboard"]
                            values: [ALL, DANIU, YIFEI, LAOSHI, BRIDGE, CONTROLLER, SEGMENTER, TTS, DASHBOARD]
                        }

                        // Search icon
                        search_icon = <View> {
                            width: 20, height: 24
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: {
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;
                                    // Magnifying glass circle
                                    sdf.circle(c.x - 2.0, c.y - 2.0, 5.0);
                                    sdf.stroke(#6b7280, 1.5);
                                    // Handle
                                    sdf.move_to(c.x + 1.5, c.y + 1.5);
                                    sdf.line_to(c.x + 6.0, c.y + 6.0);
                                    sdf.stroke(#6b7280, 1.5);
                                    return sdf.result;
                                }
                            }
                        }

                        // Search field
                        log_search = <TextInput> {
                            width: Fill, height: 24
                            empty_text: "Search..."
                            draw_bg: {
                                color: #ffffff
                                border_radius: 4.0
                            }
                            draw_text: {
                                color: #1f2937
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                            }
                        }

                        // Copy to clipboard button
                        copy_log_btn = <Button> {
                            width: 28, height: 24
                            text: ""
                            draw_bg: {
                                instance hover: 0.0
                                instance pressed: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;

                                    // Background
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                    let bg_color = mix(#e5e7eb, #d1d5db, self.hover);
                                    let bg_color = mix(bg_color, #9ca3af, self.pressed);
                                    sdf.fill(bg_color);

                                    // Clipboard icon - back rectangle
                                    let icon_color = #4b5563;
                                    sdf.box(c.x - 4.0, c.y - 2.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    // Clipboard icon - front rectangle (overlapping)
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.fill(bg_color);
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    return sdf.result;
                                }
                            }
                            animator: {
                                hover = {
                                    default: off
                                    off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 0.0}} }
                                    on = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 1.0}} }
                                }
                                pressed = {
                                    default: off
                                    off = { from: {all: Forward {duration: 0.05}} apply: {draw_bg: {pressed: 0.0}} }
                                    on = { from: {all: Forward {duration: 0.02}} apply: {draw_bg: {pressed: 1.0}} }
                                }
                            }
                        }
                    }
                }

                // Log content area
                log_body = <View> {
                    width: Fill, height: Fill
                    padding: 8

                    log_scroll = <ScrollYView> {
                        width: Fill, height: Fill

                        log_content = <Markdown> {
                            width: Fill, height: Fit
                            padding: 8
                            font_size: 10.0
                            font_color: #4b5563
                            paragraph_spacing: 2

                            draw_normal: {
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                            }
                            draw_bold: {
                                text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
                            }
                        }
                    }
                }
            }
        }
    }

    // Main App Window
    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(1400, 900) }
            pass: { clear_color: (DARK_BG) }

            body = <Dashboard> {}
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct Dashboard {
    #[deref]
    view: View,

    #[rust]
    update_timer: Timer,

    #[rust]
    prompt_text: String,

    #[rust]
    blink_state: bool,

    #[rust]
    last_log_count: usize,

    #[rust]
    log_panel_collapsed: bool,

    #[rust]
    log_panel_width: f64,

    #[rust]
    is_dragging_splitter: bool,

    #[rust]
    drag_start_x: f64,

    #[rust]
    drag_start_width: f64,

    #[rust]
    participant_levels: [f64; 3],

    #[rust]
    last_chat_count: usize,

    #[rust]
    log_level_filter: usize,  // 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR

    #[rust]
    log_node_filter: usize,   // 0=ALL, 1=Daniu, 2=Yifei, 3=Laoshi, 4=Bridge, 5=Controller, 6=Segmenter, 7=TTS, 8=Dashboard
}

impl Widget for Dashboard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Start update timer on startup and initialize log panel width
        if let Event::Startup = event {
            self.update_timer = cx.start_interval(1.0 / 30.0); // 30 FPS updates
            if self.log_panel_width == 0.0 {
                self.log_panel_width = 350.0; // Default width
            }
        }

        // Handle splitter drag events
        let splitter_area = self.view.view(ids!(splitter)).area();
        match event.hits(cx, splitter_area) {
            Hit::FingerDown(fe) => {
                self.is_dragging_splitter = true;
                self.drag_start_x = fe.abs.x;
                self.drag_start_width = self.log_panel_width;
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerMove(fe) => {
                if self.is_dragging_splitter {
                    // Dragging left increases log panel width, right decreases
                    let delta = self.drag_start_x - fe.abs.x;
                    let new_width = (self.drag_start_width + delta).clamp(200.0, 800.0);
                    self.log_panel_width = new_width;

                    // Apply width to log panel
                    self.view.view(ids!(log_panel)).apply_over(cx, live!{
                        width: (new_width)
                    });
                    self.view.redraw(cx);
                }
            }
            Hit::FingerUp(_) => {
                self.is_dragging_splitter = false;
            }
            Hit::FingerHoverIn(_) => {
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerHoverOut(_) => {
                if !self.is_dragging_splitter {
                    cx.set_cursor(MouseCursor::Default);
                }
            }
            _ => {}
        }

        // Handle timer for UI updates
        if self.update_timer.is_event(event).is_some() {
            self.update_from_shared_state(cx);
        }

        // Handle button click actions using event.actions() pattern
        let actions = event.actions();

        // Handle log panel toggle button click
        if self.view.button(ids!(toggle_log_btn)).clicked(actions) {
            self.on_toggle_log_panel(cx);
        }

        // Handle prompt section buttons
        if self.view.button(ids!(send_prompt_btn)).clicked(actions) {
            self.on_send_prompt_clicked(cx);
        }
        if self.view.button(ids!(reset_conf_btn)).clicked(actions) {
            self.on_reset_clicked();
        }

        // Handle log filter dropdown changes
        if let Some(selected) = self.view.drop_down(ids!(level_filter)).selected(actions) {
            self.log_level_filter = selected;
            self.last_log_count = 0; // Force refresh
        }
        if let Some(selected) = self.view.drop_down(ids!(node_filter)).selected(actions) {
            self.log_node_filter = selected;
            self.last_log_count = 0; // Force refresh
        }

        // Handle copy log button click
        if self.view.button(ids!(copy_log_btn)).clicked(actions) {
            self.on_copy_logs_clicked(cx);
        }

        // Handle text input changes
        if let Event::TextInput(te) = event {
            self.prompt_text.push_str(&te.input);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl Dashboard {
    fn update_from_shared_state(&mut self, cx: &mut Cx) {
        let Some(state_ref) = get_shared_state() else { return };
        let state = state_ref.lock();

        // Toggle blink state for connected indicator
        self.blink_state = !self.blink_state;

        // Update connection status indicator
        if state.is_connected {
            // Connected: show "Connected" with blinking red dot
            self.view.label(ids!(status_label))
                .set_text_with(|t| *t = "Connected".to_string());
            self.view.view(ids!(status_indicator)).apply_over(cx, live! {
                draw_bg: { color: (vec4(0.863, 0.149, 0.149, 1.0)) }
            });
            // Blinking dot - alternate between bright red and dark red
            let (r, g, b) = if self.blink_state {
                (0.863, 0.149, 0.149)
            } else {
                (0.498, 0.114, 0.114)
            };
            self.view.view(ids!(connection_dot)).apply_over(cx, live! {
                draw_bg: { color: (vec4(r, g, b, 1.0)) }
            });
        } else {
            // Demo mode: gray indicator
            self.view.label(ids!(status_label))
                .set_text_with(|t| *t = "Demo".to_string());
            self.view.view(ids!(status_indicator)).apply_over(cx, live! {
                draw_bg: { color: (vec4(0.612, 0.639, 0.686, 1.0)) }
            });
            self.view.view(ids!(connection_dot)).apply_over(cx, live! {
                draw_bg: { color: (vec4(0.612, 0.639, 0.686, 1.0)) }
            });
        }

        // Update buffer gauge, percentage label, and status dot
        let buffer_pct = state.buffer_fill / 100.0;
        let buffer_critical = if state.buffer_fill >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(buffer_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (buffer_pct) }
        });
        self.view.view(ids!(buffer_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (buffer_critical) }
        });
        self.view.label(ids!(buffer_pct_label)).set_text_with(|t| {
            *t = format!("{:.0}%", state.buffer_fill)
        });

        // Update CPU gauge, percentage label, and status dot
        let cpu_pct = (state.cpu_usage / 100.0) as f64;
        let cpu_critical = if state.cpu_usage >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(cpu_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (cpu_pct) }
        });
        self.view.view(ids!(cpu_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (cpu_critical) }
        });
        self.view.label(ids!(cpu_pct_label)).set_text_with(|t| {
            *t = format!("{:.0}%", state.cpu_usage)
        });

        // Update Memory gauge, percentage label, and status dot
        let memory_pct = (state.memory_usage / 100.0) as f64;
        let memory_critical = if state.memory_usage >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(memory_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (memory_pct) }
        });
        self.view.view(ids!(memory_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (memory_critical) }
        });
        self.view.label(ids!(memory_pct_label)).set_text_with(|t| {
            *t = format!("{:.1}/{:.0}G", state.used_memory_gb, state.total_memory_gb)
        });

        // Calculate band levels from waveform data
        let band_levels = if state.waveform_data.is_empty() {
            [0.0f32; 8]
        } else {
            let samples = &state.waveform_data;
            let band_size = samples.len() / 8;
            let mut levels = [0.0f32; 8];
            let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, |a, b| a.max(b));
            let norm_factor = if peak > 0.01 { 1.0 / peak } else { 1.0 };

            for i in 0..8 {
                let start = i * band_size;
                let end = ((i + 1) * band_size).min(samples.len());
                if end > start {
                    let sum_sq: f32 = samples[start..end].iter().map(|s| s * s).sum();
                    let rms = (sum_sq / (end - start) as f32).sqrt();
                    levels[i] = (rms * norm_factor * 1.5).clamp(0.0, 1.0);
                }
            }
            levels
        };

        // Update participant panels
        let is_audio_playing = state.playback_status.is_playing;
        let active_idx = state.playback_status.active_participant_idx;

        let panel_ids: [&[LiveId]; 3] = [ids!(student1_panel), ids!(student2_panel), ids!(tutor_panel)];

        for (i, panel_id) in panel_ids.into_iter().enumerate() {
            if let Some(participant) = state.participants.get(i) {
                let panel = self.view.view(panel_id);

                // Update name in header
                panel.label(ids!(header.name_label)).set_text_with(|t| *t = participant.name.clone());

                // Check if this participant is the current audio speaker
                let is_current_audio_speaker = is_audio_playing && active_idx == Some(i);

                // Determine status: 0=idle(blue), 1=speaking(green), 2=error(red)
                let status_val = if participant.status.to_lowercase().contains("error") {
                    2.0  // Red
                } else if is_current_audio_speaker {
                    1.0  // Green - currently playing audio
                } else if participant.is_speaking {
                    1.0  // Green - generating text
                } else {
                    0.0  // Blue
                };

                // Update status indicator
                panel.view(ids!(header.indicator)).apply_over(cx, live! {
                    draw_bg: { status: (status_val) }
                });

                // Update waveform - show bands for active speaker, level bar for all
                let new_level = if is_current_audio_speaker && !state.waveform_data.is_empty() {
                    let samples = &state.waveform_data;
                    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
                    let rms = (sum_sq / samples.len() as f32).sqrt();
                    (rms * 2.0).clamp(0.0, 1.0) as f64
                } else {
                    self.participant_levels[i] * 0.85
                };
                self.participant_levels[i] = new_level;

                let active_val = if is_current_audio_speaker { 1.0 } else { 0.0 };
                panel.view(ids!(waveform)).apply_over(cx, live! {
                    draw_bg: {
                        level: (new_level),
                        active: (active_val),
                        band0: (if is_current_audio_speaker { band_levels[0] as f64 } else { 0.0 }),
                        band1: (if is_current_audio_speaker { band_levels[1] as f64 } else { 0.0 }),
                        band2: (if is_current_audio_speaker { band_levels[2] as f64 } else { 0.0 }),
                        band3: (if is_current_audio_speaker { band_levels[3] as f64 } else { 0.0 }),
                        band4: (if is_current_audio_speaker { band_levels[4] as f64 } else { 0.0 }),
                        band5: (if is_current_audio_speaker { band_levels[5] as f64 } else { 0.0 }),
                        band6: (if is_current_audio_speaker { band_levels[6] as f64 } else { 0.0 }),
                        band7: (if is_current_audio_speaker { band_levels[7] as f64 } else { 0.0 }),
                    }
                });
            }
        }

        // Update log panel with formatted messages (newest at bottom for auto-scroll)
        let log_count = state.log_messages.len();
        // Update chat content with conversation history (filter out Context messages)
        let filtered_messages: Vec<_> = state.chat_messages.iter()
            .filter(|msg| msg.sender != "Context")
            .collect();
        let chat_text = if filtered_messages.is_empty() {
            "Waiting for conversation...".to_string()
        } else {
            filtered_messages.iter()
                .map(|msg| format!("**{}** ({}):  \n{}", msg.sender, msg.timestamp, msg.text))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n")
        };
        self.view.markdown(ids!(chat_content)).set_text(cx, &chat_text);

        // Auto-scroll chat to bottom only when NEW messages arrive
        let chat_count = filtered_messages.len();
        if chat_count > self.last_chat_count {
            self.view.view(ids!(chat_scroll)).set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
            self.last_chat_count = chat_count;
        }

        // Apply log filters (level, node, and search text)
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Get search text from input field
        let search_text = self.view.text_input(ids!(log_search)).text().to_lowercase();
        let search_filter = search_text.trim();

        let filtered_logs: Vec<_> = state.log_messages.iter()
            .filter(|msg| {
                // Level filter: 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
                let level_pass = match level_filter {
                    0 => true, // ALL
                    1 => msg.level.to_uppercase() == "DEBUG",
                    2 => msg.level.to_uppercase() == "INFO",
                    3 => msg.level.to_uppercase().contains("WARN"),
                    4 => msg.level.to_uppercase() == "ERROR",
                    _ => true,
                };

                // Node filter: 0=ALL, 1=Daniu, 2=Yifei, 3=Laoshi, 4=Bridge, 5=Controller, 6=Segmenter, 7=TTS, 8=Dashboard
                let source_lower = msg.source.to_lowercase();
                let node_pass = match node_filter {
                    0 => true, // ALL
                    1 => source_lower.contains("student1") || source_lower.contains("llm1") || source_lower.contains("daniu"),
                    2 => source_lower.contains("student2") || source_lower.contains("llm2") || source_lower.contains("yifei"),
                    3 => source_lower.contains("tutor") || source_lower.contains("judge") || source_lower.contains("laoshi"),
                    4 => source_lower.contains("bridge"),
                    5 => source_lower.contains("controller"),
                    6 => source_lower.contains("segmenter"),
                    7 => source_lower.contains("tts") || source_lower.contains("primespeech"),
                    8 => source_lower.contains("dashboard"),
                    _ => true,
                };

                // Search filter: check if message or source contains the search text
                let search_pass = if search_filter.is_empty() {
                    true
                } else {
                    msg.message.to_lowercase().contains(search_filter) ||
                    msg.source.to_lowercase().contains(search_filter)
                };

                level_pass && node_pass && search_pass
            })
            .collect();

        let log_text = if filtered_logs.is_empty() {
            if state.log_messages.is_empty() {
                format!("Waiting for log messages... (Buffer: {:.1}%)", state.buffer_fill)
            } else {
                "No matching log messages".to_string()
            }
        } else {
            // Show last 1000 filtered messages, oldest first (so newest is at bottom)
            filtered_logs.iter()
                .rev()
                .take(1000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|msg| format!("[{}] {}: {}", msg.timestamp, msg.source, msg.message))
                .collect::<Vec<_>>()
                .join("  \n")
        };
        // Update log content (using Markdown widget for emoji support)
        self.view.markdown(ids!(log_content)).set_text(cx, &log_text);

        // Only auto-scroll when NEW logs arrive (not on filter changes)
        // This allows users to scroll up and read old logs
        if log_count > self.last_log_count {
            self.last_log_count = log_count;
            self.view.view(ids!(log_scroll)).set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
        }

        self.view.redraw(cx);
    }

    /// Handle send prompt button click
    fn on_send_prompt_clicked(&mut self, cx: &mut Cx) {
        // Get text from input widget, use default if empty
        let input_text = self.view.text_input(ids!(prompt_input)).text();
        let text = if input_text.trim().is_empty() {
            "Let's start".to_string()
        } else {
            input_text
        };

        ::log::info!("Sending prompt: {}", text);

        // Send to Dora bridge via control command
        if let Some(state_ref) = get_shared_state() {
            let mut state = state_ref.lock();
            state.control_commands.push(crate::ControlCommand::SendPrompt(text.clone()));
        }

        // Clear input field
        self.view.text_input(ids!(prompt_input)).set_text(cx, "");
        self.prompt_text.clear();
    }

    /// Handle reset conference button click
    fn on_reset_clicked(&mut self) {
        ::log::info!("Resetting conference");

        if let Some(state_ref) = get_shared_state() {
            let mut state = state_ref.lock();
            state.control_commands.push(crate::ControlCommand::Reset);
        }
    }

    /// Handle copy logs to clipboard button click
    fn on_copy_logs_clicked(&mut self, cx: &mut Cx) {
        let Some(state_ref) = get_shared_state() else {
            ::log::warn!("Cannot copy logs: shared state not available");
            return;
        };

        let state = state_ref.lock();

        // Apply current filters to get the logs to copy
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;
        let search_text = self.view.text_input(ids!(log_search)).text().to_lowercase();
        let search_filter = search_text.trim();

        let filtered_logs: Vec<_> = state.log_messages.iter()
            .filter(|msg| {
                // Level filter
                let level_pass = match level_filter {
                    0 => true,
                    1 => msg.level.to_uppercase() == "DEBUG",
                    2 => msg.level.to_uppercase() == "INFO",
                    3 => msg.level.to_uppercase().contains("WARN"),
                    4 => msg.level.to_uppercase() == "ERROR",
                    _ => true,
                };

                // Node filter
                let source_lower = msg.source.to_lowercase();
                let node_pass = match node_filter {
                    0 => true,
                    1 => source_lower.contains("student1") || source_lower.contains("llm1") || source_lower.contains("daniu"),
                    2 => source_lower.contains("student2") || source_lower.contains("llm2") || source_lower.contains("yifei"),
                    3 => source_lower.contains("tutor") || source_lower.contains("judge") || source_lower.contains("laoshi"),
                    4 => source_lower.contains("bridge"),
                    5 => source_lower.contains("controller"),
                    6 => source_lower.contains("segmenter"),
                    7 => source_lower.contains("tts") || source_lower.contains("primespeech"),
                    8 => source_lower.contains("dashboard"),
                    _ => true,
                };

                // Search filter
                let search_pass = if search_filter.is_empty() {
                    true
                } else {
                    msg.message.to_lowercase().contains(search_filter) ||
                    msg.source.to_lowercase().contains(search_filter)
                };

                level_pass && node_pass && search_pass
            })
            .collect();

        // Format logs for clipboard
        let log_text = if filtered_logs.is_empty() {
            "No log messages to copy".to_string()
        } else {
            filtered_logs.iter()
                .map(|msg| format!("[{}] [{}] {}: {}", msg.timestamp, msg.level, msg.source, msg.message))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Copy to clipboard using Makepad's clipboard API
        cx.copy_to_clipboard(&log_text);
        ::log::info!("Copied {} log entries to clipboard", filtered_logs.len());
    }

    /// Handle log panel toggle button click
    fn on_toggle_log_panel(&mut self, cx: &mut Cx) {
        self.log_panel_collapsed = !self.log_panel_collapsed;

        // Update panel width and button text
        if self.log_panel_collapsed {
            // Collapsed: narrow panel (just toggle button), show "<" to expand
            self.view.view(ids!(log_panel)).apply_over(cx, live!{
                width: 32
            });
            self.view.view(ids!(splitter)).apply_over(cx, live!{
                width: 0
            });
            self.view.view(ids!(log_content_column)).apply_over(cx, live!{
                visible: false
            });
            self.view.button(ids!(toggle_log_btn)).set_text(cx, "<");
        } else {
            // Expanded: restore to saved width, show ">" to collapse
            let width = self.log_panel_width;
            self.view.view(ids!(log_panel)).apply_over(cx, live!{
                width: (width)
            });
            self.view.view(ids!(splitter)).apply_over(cx, live!{
                width: 6
            });
            self.view.view(ids!(log_content_column)).apply_over(cx, live!{
                visible: true
            });
            self.view.button(ids!(toggle_log_btn)).set_text(cx, ">");
        }

        self.view.redraw(cx);
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);  // Already includes emoji font registration
        widgets::live_design(cx);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

app_main!(App);

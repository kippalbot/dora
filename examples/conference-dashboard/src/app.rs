//! Makepad Application setup for Conference Dashboard

use makepad_widgets::*;
use makepad_widgets::makepad_platform::{AudioDeviceType, AudioDeviceDesc};
use crate::{SharedStateRef, widgets};
use std::sync::OnceLock;

// Re-export log macros with explicit crate reference to avoid glob import ambiguity
use ::log as logger;

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
    use crate::widgets::sidebar::Sidebar;
    use crate::widgets::mofa_hero::MofaHero;

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

    // Action icons
    ICO_START = dep("crate://self/resources/icons/start.svg")
    ICO_STOP = dep("crate://self/resources/icons/stop.svg")

    // Light color palette
    DARK_BG = #f5f7fa
    PANEL_BG = #ffffff
    ACCENT_BLUE = #3b82f6
    ACCENT_GREEN = #10b981
    TEXT_PRIMARY = #1f2937
    TEXT_SECONDARY = #6b7280

    // Main Dashboard Layout
    Dashboard = {{Dashboard}} <View> {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: { color: (DARK_BG) }

        // Header at top (full width)
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 12
            align: {y: 0.5}
            padding: {left: 20, right: 20, top: 15, bottom: 15}
            show_bg: true
            draw_bg: { color: (PANEL_BG) }

            // Logo
            logo = <Image> {
                width: 40, height: 40
                source: (MOFA_LOGO)
            }

            title = <Label> {
                text: "MoFA Desktop"
                draw_text: {
                    color: (TEXT_PRIMARY)
                    text_style: <FONT_BOLD>{ font_size: 24.0 }
                }
            }

            <View> { width: Fill, height: 1 }

            // User profile dropdown button container
            user_profile_container = <View> {
                width: Fit, height: Fill
                flow: Right
                align: {x: 0.5, y: 0.5}
                spacing: 4
                cursor: Hand

                // User icon button - circular with centered SVG icon
                user_profile_btn = <View> {
                    width: 32, height: 32
                    padding: {left: 6, top: 8, right: 10, bottom: 8}
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cx = self.rect_size.x * 0.5;
                            let cy = self.rect_size.y * 0.5;
                            sdf.circle(cx, cy, 15.0);
                            sdf.fill(#f1f5f9);
                            return sdf.result;
                        }
                    }

                    <Icon> {
                        draw_icon: {
                            svg_file: dep("crate://self/resources/icons/user.svg")
                            fn get_color(self) -> vec4 { return #4b5563; }
                        }
                        icon_walk: {width: 16, height: 16}
                    }
                }

                // Dropdown arrow indicator
                dropdown_arrow = <View> {
                    width: 12, height: Fill
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cx = self.rect_size.x * 0.5;
                            let cy = self.rect_size.y * 0.5;
                            // Chevron down arrow
                            sdf.move_to(cx - 4.0, cy - 2.0);
                            sdf.line_to(cx, cy + 2.0);
                            sdf.line_to(cx + 4.0, cy - 2.0);
                            sdf.stroke(#9ca3af, 1.5);
                            return sdf.result;
                        }
                    }
                }
            }
        }

        // Content area below header (sidebar + main content + log panel)
        content_area = <View> {
            width: Fill, height: Fill
            flow: Right

            // Left sidebar zone (fold button at top, sidebar below)
            sidebar_zone = <View> {
                width: Fit, height: Fill
                flow: Down
                show_bg: true
                draw_bg: { color: #f0f2f5 }

                // Fold button at top (40x40, positioned at left)
                fold_button_panel = <View> {
                    width: 40, height: 40
                    align: {x: 0.5, y: 0.5}
                    show_bg: true
                    draw_bg: { color: #f0f2f5 }

                    sidebar_trigger = <View> {
                        width: 32, height: 32
                        cursor: Hand
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let cy = self.rect_size.y * 0.5;
                                let cx = self.rect_size.x * 0.5;
                                // Hamburger lines - gray, blue on hover
                                let line_color = mix(#64748b, #3b82f6, self.hover);
                                sdf.move_to(cx - 8.0, cy - 6.0);
                                sdf.line_to(cx + 8.0, cy - 6.0);
                                sdf.stroke(line_color, 2.0);
                                sdf.move_to(cx - 8.0, cy);
                                sdf.line_to(cx + 8.0, cy);
                                sdf.stroke(line_color, 2.0);
                                sdf.move_to(cx - 8.0, cy + 6.0);
                                sdf.line_to(cx + 8.0, cy + 6.0);
                                sdf.stroke(line_color, 2.0);
                                return sdf.result;
                            }
                        }
                    }
                }

                // Sidebar content (starts hidden, expands below the button)
                sidebar_content = <View> {
                    width: 0, height: Fill
                    sidebar = <Sidebar> {}
                }
            }

            // Main content
            main_content = <View> {
                width: Fill, height: Fill
                flow: Down
                padding: 20

            // Content area with switchable pages
            content = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 12

            // FM Page (default visible)
            fm_page = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 12
                visible: true

            // Top row - System status bar (MofaHero widget)
            mofa_hero = <MofaHero> {}

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

            // Audio control panel (above prompt input)
            audio_panel = <RoundedView> {
                width: Fill, height: Fit
                padding: 12
                margin: {bottom: 8}
                draw_bg: {
                    color: (PANEL_BG)
                    border_radius: 4.0
                }
                flow: Right
                spacing: 16
                align: {y: 0.5}

                // Mic level meter with clickable mic icon
                <View> {
                    width: Fit, height: Fit
                    flow: Right
                    spacing: 8
                    align: {y: 0.5}

                    mic_mute_btn = <View> {
                        width: 18, height: 18
                        flow: Overlay
                        cursor: Hand

                        mic_icon_on = <View> {
                            width: 18, height: 18
                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/mic.svg")
                                    fn get_color(self) -> vec4 { return #64748b; }
                                }
                                icon_walk: {width: 18, height: 18}
                            }
                        }

                        mic_icon_off = <View> {
                            width: 18, height: 18
                            visible: false
                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/mic-off.svg")
                                    fn get_color(self) -> vec4 { return #ef4444; }
                                }
                                icon_walk: {width: 18, height: 18}
                            }
                        }
                    }

                    // LED level meter (5 bars)
                    mic_level_meter = <View> {
                        width: 60, height: 16
                        flow: Right
                        spacing: 2
                        align: {y: 0.5}

                        mic_led_1 = <RoundedView> {
                            width: 10, height: 12
                            draw_bg: { color: #d9d9e0, border_radius: 2.0 }
                        }
                        mic_led_2 = <RoundedView> {
                            width: 10, height: 12
                            draw_bg: { color: #d9d9e0, border_radius: 2.0 }
                        }
                        mic_led_3 = <RoundedView> {
                            width: 10, height: 12
                            draw_bg: { color: #d9d9e0, border_radius: 2.0 }
                        }
                        mic_led_4 = <RoundedView> {
                            width: 10, height: 12
                            draw_bg: { color: #d9d9e0, border_radius: 2.0 }
                        }
                        mic_led_5 = <RoundedView> {
                            width: 10, height: 12
                            draw_bg: { color: #d9d9e0, border_radius: 2.0 }
                        }
                    }
                }

                // Divider
                <View> { width: 1, height: 24, show_bg: true, draw_bg: { color: #d1d5db } }

                // AEC toggle - white icon on neon green background when ON, gray when OFF
                aec_toggle_btn = <View> {
                    width: 32, height: 32
                    flow: Overlay
                    cursor: Hand
                    show_bg: true
                    draw_bg: {
                        instance blink: 0.0
                        instance enabled: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            // Neon green when enabled, gray when disabled
                            let green = vec4(0.13, 0.77, 0.37, 1.0);
                            let bright_green = vec4(0.2, 0.9, 0.5, 1.0);
                            let gray = vec4(0.9, 0.91, 0.92, 1.0);
                            let base = mix(gray, green, self.enabled);
                            let col = mix(base, bright_green, self.blink * 0.5 * self.enabled);
                            sdf.fill(col);
                            return sdf.result;
                        }
                    }
                    align: {x: 0.5, y: 0.5}

                    aec_icon_on = <View> {
                        width: Fit, height: Fit
                        <Icon> {
                            draw_icon: {
                                svg_file: dep("crate://self/resources/icons/aec.svg")
                                fn get_color(self) -> vec4 { return #ffffff; }
                            }
                            icon_walk: {width: 20, height: 20}
                        }
                    }

                    aec_icon_off = <View> {
                        width: Fit, height: Fit
                        visible: false
                        <Icon> {
                            draw_icon: {
                                svg_file: dep("crate://self/resources/icons/aec.svg")
                                fn get_color(self) -> vec4 { return #6b7280; }
                            }
                            icon_walk: {width: 20, height: 20}
                        }
                    }
                }

                // Divider
                <View> { width: 1, height: 24, show_bg: true, draw_bg: { color: #d1d5db } }

                // Input device dropdown
                <View> {
                    width: Fit, height: Fit
                    flow: Right
                    spacing: 6
                    align: {y: 0.5}

                    <Label> {
                        text: "Microphone"
                        draw_text: {
                            color: (TEXT_SECONDARY)
                            text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                        }
                    }

                    input_device_dropdown = <DropDown> {
                        width: 240, height: 28
                        popup_menu_position: BelowInput
                        draw_text: {
                            text_style: <FONT_REGULAR>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix(#374151, #1f2937, self.focus);
                            }
                        }
                        popup_menu: {
                            draw_bg: {
                                color: #ffffff
                                border_color: #e5e7eb
                                border_size: 1.0
                            }
                            menu_item: {
                                draw_bg: {
                                    color: #ffffff
                                    color_hover: #f3f4f6
                                }
                                draw_text: {
                                    fn get_color(self) -> vec4 {
                                        return mix(
                                            mix(#374151, #1f2937, self.active),
                                            #1f2937,
                                            self.hover
                                        );
                                    }
                                }
                            }
                        }
                        labels: ["Loading..."]
                        values: [loading]
                    }
                }

                // Output device dropdown
                <View> {
                    width: Fit, height: Fit
                    flow: Right
                    spacing: 6
                    align: {y: 0.5}

                    <Label> {
                        text: "Speaker"
                        draw_text: {
                            color: (TEXT_SECONDARY)
                            text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                        }
                    }

                    output_device_dropdown = <DropDown> {
                        width: 240, height: 28
                        popup_menu_position: BelowInput
                        draw_text: {
                            text_style: <FONT_REGULAR>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix(#374151, #1f2937, self.focus);
                            }
                        }
                        popup_menu: {
                            draw_bg: {
                                color: #ffffff
                                border_color: #e5e7eb
                                border_size: 1.0
                            }
                            menu_item: {
                                draw_bg: {
                                    color: #ffffff
                                    color_hover: #f3f4f6
                                }
                                draw_text: {
                                    fn get_color(self) -> vec4 {
                                        return mix(
                                            mix(#374151, #1f2937, self.active),
                                            #1f2937,
                                            self.hover
                                        );
                                    }
                                }
                            }
                        }
                        labels: ["Loading..."]
                        values: [loading]
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
                        width: Fill, height: 32
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
                            text_style: <FONT_REGULAR>{ font_size: 11.0 }
                        }
                        draw_cursor: {
                            color: #1f2937
                        }
                        draw_selection: {
                            color: #bfdbfe
                        }
                    }

                    send_prompt_btn = <Button> {
                        width: 60, height: 32
                        text: "Send"
                        draw_text: {
                            color: #ffffff
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return self.color;
                            }
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
                        width: 60, height: 32
                        text: "Reset"
                        draw_text: {
                            color: #4b5563
                            text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return self.color;
                            }
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
            } // end fm_page

            // App Page (hidden by default, shown when clicking app buttons)
            app_page = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 12
                visible: false
                align: {x: 0.5, y: 0.5}

                <Label> {
                    text: "App Page"
                    draw_text: {
                        color: #9ca3af
                        text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                    }
                }
                <Label> {
                    text: "Select an app from the sidebar"
                    draw_text: {
                        color: #d1d5db
                        text_style: <FONT_REGULAR>{ font_size: 13.0 }
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
                        text_style: <FONT_BOLD>{ font_size: 12.0 }
                        color: #4b5563
                        fn get_color(self) -> vec4 {
                            return self.color;
                        }
                    }
                    draw_bg: {
                        instance color: #d1d5db
                        instance color_hover: #c4c9d0
                        border_radius: 4.0
                        fn get_color(self) -> vec4 {
                            return mix(self.color, self.color_hover, self.hover);
                        }
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            sdf.fill(self.get_color());
                            return sdf.result;
                        }
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
                                fn get_color(self) -> vec4 {
                                    return mix(#374151, #1f2937, self.focus);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    color: #ffffff
                                    border_color: #e5e7eb
                                    border_size: 1.0
                                }
                                menu_item: {
                                    draw_bg: {
                                        color: #ffffff
                                        color_hover: #f3f4f6
                                    }
                                    draw_text: {
                                        fn get_color(self) -> vec4 {
                                            return mix(
                                                mix(#374151, #1f2937, self.active),
                                                #1f2937,
                                                self.hover
                                            );
                                        }
                                    }
                                }
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
                                fn get_color(self) -> vec4 {
                                    return mix(#374151, #1f2937, self.focus);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    color: #ffffff
                                    border_color: #e5e7eb
                                    border_size: 1.0
                                }
                                menu_item: {
                                    draw_bg: {
                                        color: #ffffff
                                        color_hover: #f3f4f6
                                    }
                                    draw_text: {
                                        fn get_color(self) -> vec4 {
                                            return mix(
                                                mix(#374151, #1f2937, self.active),
                                                #1f2937,
                                                self.hover
                                            );
                                        }
                                    }
                                }
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
        } // end log_panel
        } // end content_area

    // Main App Window
    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(1400, 900) }
            pass: { clear_color: (DARK_BG) }
            flow: Overlay

            body = <Dashboard> {}

            // Invisible click zone for user button (covers entire user_profile_container area)
            user_btn_overlay = <View> {
                width: 60, height: 44
                abs_pos: vec2(1320.0, 10.0)
                cursor: Hand
            }

            // User dropdown menu - at Window level for proper overlay
            user_menu = <View> {
                width: 140, height: Fit
                abs_pos: vec2(1250.0, 55.0)
                visible: false
                padding: 6
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        sdf.fill(#f8fafc);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        sdf.stroke(#e2e8f0, 1.0);
                        return sdf.result;
                    }
                }
                flow: Down
                spacing: 2

                menu_profile_btn = <Button> {
                    width: Fill, height: Fit
                    padding: {top: 10, bottom: 10, left: 10, right: 10}
                    align: {x: 0.0, y: 0.5}
                    text: "Profile"
                    icon_walk: {width: 14, height: 14, margin: {right: 8}}
                    draw_icon: {
                        svg_file: dep("crate://self/resources/icons/user.svg")
                        fn get_color(self) -> vec4 { return #64748b; }
                    }
                    draw_text: {
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 { return #374151; }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let color = mix(#f8fafc, #e2e8f0, self.hover);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                    animator: {
                        hover = {
                            default: off
                            off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 0.0}} }
                            on = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 1.0}} }
                        }
                    }
                }
                menu_settings_btn = <Button> {
                    width: Fill, height: Fit
                    padding: {top: 10, bottom: 10, left: 10, right: 10}
                    align: {x: 0.0, y: 0.5}
                    text: "Settings"
                    icon_walk: {width: 14, height: 14, margin: {right: 8}}
                    draw_icon: {
                        svg_file: dep("crate://self/resources/icons/settings.svg")
                        fn get_color(self) -> vec4 { return #64748b; }
                    }
                    draw_text: {
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 { return #374151; }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let color = mix(#f8fafc, #e2e8f0, self.hover);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                    animator: {
                        hover = {
                            default: off
                            off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 0.0}} }
                            on = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 1.0}} }
                        }
                    }
                }
                menu_logout_btn = <Button> {
                    width: Fill, height: Fit
                    padding: {top: 10, bottom: 10, left: 10, right: 10}
                    align: {x: 0.0, y: 0.5}
                    text: "Logout"
                    icon_walk: {width: 14, height: 14, margin: {right: 8}}
                    draw_icon: {
                        svg_file: dep("crate://self/resources/icons/logout.svg")
                        fn get_color(self) -> vec4 { return #64748b; }
                    }
                    draw_text: {
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 { return #374151; }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let color = mix(#f8fafc, #e2e8f0, self.hover);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                    animator: {
                        hover = {
                            default: off
                            off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 0.0}} }
                            on = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 1.0}} }
                        }
                    }
                }
            }

            // Invisible hover zone for sidebar trigger (hamburger button)
            sidebar_trigger_overlay = <View> {
                width: 40, height: 40
                abs_pos: vec2(4.0, 74.0)
                cursor: Hand
            }

            // Sidebar menu overlay - positioned below hamburger button
            sidebar_menu_overlay = <View> {
                width: 180, height: 700
                abs_pos: vec2(0.0, 110.0)
                visible: false
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        sdf.fill(#ffffff);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        sdf.stroke(#e2e8f0, 1.0);
                        return sdf.result;
                    }
                }

                sidebar_content = <Sidebar> {}
            }
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

    #[rust]
    sidebar_collapsed: bool,  // true = sidebar hidden (default)

    #[rust]
    sidebar_pinned: bool,     // true = sidebar locked open by click

    #[rust]
    sidebar_hover_active: bool,  // Track if mouse is in sidebar zone

    #[rust]
    sidebar_content_hover: bool,  // Track if mouse is over sidebar content

    #[rust]
    hover_zone_active: bool,  // Track if mouse is over hamburger button

    #[rust]
    sidebar_show_grace_period: u8,  // Counter to prevent immediate hide after show

    #[rust]
    sidebar_hide_pending: bool,  // True when waiting to hide sidebar
    
    #[rust]
    sidebar_hide_counter: u8,  // Counter for delayed hide

    #[rust]
    mic_muted: bool,  // Microphone mute state

    #[rust]
    aec_enabled: bool,  // AEC (Acoustic Echo Cancellation) enabled state

    #[rust]
    mic_level: f32,  // Current microphone input level (0.0 - 1.0)

    #[rust]
    aec_blink_state: bool,  // For AEC button blinking animation

    #[rust]
    aec_blink_counter: u8,  // Counter to slow down blink rate

    #[rust]
    audio_devices: Vec<AudioDeviceDesc>,  // All available audio devices from Makepad

    #[rust]
    action_running: bool,  // Action button state (start/stop)
}

impl Widget for Dashboard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);

        // Start update timer on startup and initialize defaults
        if let Event::Startup = event {
            self.update_timer = cx.start_interval(1.0 / 30.0); // 30 FPS updates
            if self.log_panel_width == 0.0 {
                self.log_panel_width = 350.0; // Default width
            }
            // Default audio settings
            self.aec_enabled = true;
            self.mic_muted = false;

            // Request default audio devices to trigger Makepad's audio device enumeration
            // This causes handle_audio_devices callback to be called with full device list
            cx.use_audio_inputs(&[]);
            cx.use_audio_outputs(&[]);
        }

        // Handle splitter drag events
        let splitter_area = self.view.view(ids!(content_area.splitter)).area();
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
                    self.view.view(ids!(content_area.log_panel)).apply_over(cx, live!{
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

            // Update AEC blink animation (only when enabled, at stable rate)
            if self.aec_enabled {
                self.aec_blink_counter += 1;
                if self.aec_blink_counter >= 30 {  // Toggle every 30 ticks (1s at 30fps)
                    self.aec_blink_counter = 0;
                    self.aec_blink_state = !self.aec_blink_state;
                    let blink_val = if self.aec_blink_state { 1.0 } else { 0.6 };
                    self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.aec_toggle_btn)).apply_over(cx, live!{
                        draw_bg: { blink: (blink_val) }
                    });
                }
            }

        }

        // Handle button click actions
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

        // Handle log panel toggle button click
        if self.view.button(ids!(content_area.log_panel.toggle_column.toggle_log_btn)).clicked(actions) {
            self.on_toggle_log_panel(cx);
        }

        // Note: Sidebar hover and click handling is done at App level (sidebar is overlay)

        // Handle mic mute button click
        let mic_btn = self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.mic_mute_btn));
        match event.hits(cx, mic_btn.area()) {
            Hit::FingerUp(_) => {
                self.mic_muted = !self.mic_muted;
                self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.mic_mute_btn.mic_icon_on))
                    .set_visible(cx, !self.mic_muted);
                self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.mic_mute_btn.mic_icon_off))
                    .set_visible(cx, self.mic_muted);
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle AEC toggle click
        let aec_btn = self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.aec_toggle_btn));
        match event.hits(cx, aec_btn.area()) {
            Hit::FingerUp(_) => {
                self.aec_enabled = !self.aec_enabled;
                let enabled_val = if self.aec_enabled { 1.0 } else { 0.0 };
                self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.aec_toggle_btn))
                    .apply_over(cx, live!{ draw_bg: { enabled: (enabled_val) } });
                self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.aec_toggle_btn.aec_icon_on))
                    .set_visible(cx, self.aec_enabled);
                self.view.view(ids!(content_area.main_content.content.fm_page.audio_panel.aec_toggle_btn.aec_icon_off))
                    .set_visible(cx, !self.aec_enabled);
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle prompt section buttons
        if self.view.button(ids!(content_area.main_content.content.fm_page.prompt_section.send_prompt_btn)).clicked(actions) {
            self.on_send_prompt_clicked(cx);
        }
        if self.view.button(ids!(content_area.main_content.content.fm_page.prompt_section.reset_conf_btn)).clicked(actions) {
            self.on_reset_clicked();
        }

        // Handle log filter dropdown changes
        if let Some(selected) = self.view.drop_down(ids!(content_area.log_panel.log_content_column.log_header.log_filter_row.level_filter)).selected(actions) {
            self.log_level_filter = selected;
            self.last_log_count = 0; // Force refresh
        }
        if let Some(selected) = self.view.drop_down(ids!(content_area.log_panel.log_content_column.log_header.log_filter_row.node_filter)).selected(actions) {
            self.log_node_filter = selected;
            self.last_log_count = 0; // Force refresh
        }

        // Handle copy log button click
        if self.view.button(ids!(content_area.log_panel.log_content_column.log_header.log_filter_row.copy_log_btn)).clicked(actions) {
            self.on_copy_logs_clicked(cx);
        }

        // Handle action button click (start/stop toggle)
        if self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.start_btn)).clicked(actions) {
            self.action_running = true;
            self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.start_btn)).apply_over(cx, live! { visible: false });
            self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.stop_btn)).apply_over(cx, live! { visible: true });
            self.view.redraw(cx);
        }
        if self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.stop_btn)).clicked(actions) {
            self.action_running = false;
            self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.start_btn)).apply_over(cx, live! { visible: true });
            self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.action_btn_container.stop_btn)).apply_over(cx, live! { visible: false });
            self.view.redraw(cx);
        }

        // Note: User menu click handling is done at App level since user_menu is at Window level

        // Handle text input changes
        if let Event::TextInput(te) = event {
            self.prompt_text.push_str(&te.input);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for Dashboard {
    /// Handle audio device enumeration from Makepad's audio system
    fn handle_audio_devices(&mut self, cx: &mut Cx, devices: &AudioDevicesEvent, _scope: &mut Scope) {
        let mut input_names = Vec::new();
        let mut output_names = Vec::new();
        let mut default_input_name = String::new();
        let mut default_output_name = String::new();

        // Collect device names by type
        for desc in &devices.descs {
            match desc.device_type {
                AudioDeviceType::Input => {
                    input_names.push(desc.name.clone());
                    if desc.is_default {
                        default_input_name = desc.name.clone();
                    }
                }
                AudioDeviceType::Output => {
                    output_names.push(desc.name.clone());
                    if desc.is_default {
                        default_output_name = desc.name.clone();
                    }
                }
            }
        }

        logger::info!("Makepad audio devices: {} inputs, {} outputs", input_names.len(), output_names.len());
        for name in &input_names {
            logger::info!("  Input: {}", name);
        }
        for name in &output_names {
            logger::info!("  Output: {}", name);
        }

        // Update input device dropdown
        if !input_names.is_empty() {
            let input_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.input_device_dropdown));
            input_dropdown.set_labels(cx, input_names.clone());
            input_dropdown.set_selected_by_label(&default_input_name, cx);
        }

        // Update output device dropdown
        if !output_names.is_empty() {
            let output_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.output_device_dropdown));
            output_dropdown.set_labels(cx, output_names.clone());
            output_dropdown.set_selected_by_label(&default_output_name, cx);
        }

        // Store device list for later use
        self.audio_devices = devices.descs.clone();

        self.view.redraw(cx);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Handle device selection changes
        let input_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.input_device_dropdown));
        if input_dropdown.changed(actions).is_some() {
            let selected_label = input_dropdown.selected_label();
            if let Some(device) = self.audio_devices.iter().find(|d| d.name == selected_label && d.device_type == AudioDeviceType::Input) {
                cx.use_audio_inputs(&[device.device_id]);
                logger::info!("Selected input device: {}", selected_label);
            }
        }

        let output_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.output_device_dropdown));
        if output_dropdown.changed(actions).is_some() {
            let selected_label = output_dropdown.selected_label();
            if let Some(device) = self.audio_devices.iter().find(|d| d.name == selected_label && d.device_type == AudioDeviceType::Output) {
                cx.use_audio_outputs(&[device.device_id]);
                logger::info!("Selected output device: {}", selected_label);
            }
        }
    }
}

impl Dashboard {
    fn update_from_shared_state(&mut self, cx: &mut Cx) {
        let Some(state_ref) = get_shared_state() else { return };
        let state = state_ref.lock();

        // Toggle blink state for connected indicator
        self.blink_state = !self.blink_state;

        // Update dataflow status button
        // Status: 0=Ready(gray), 1=Connected(green), 2=Error(red)
        let (status_val, status_text, dot_color) = if state.is_connected {
            // Connected: green with blinking dot
            let (r, g, b) = if self.blink_state {
                (0.13, 0.77, 0.37)  // Bright green
            } else {
                (0.08, 0.55, 0.25)  // Dim green
            };
            (1.0, "Connected", (r, g, b))
        } else {
            // Ready: gray (default state)
            (0.0, "Ready", (0.612, 0.639, 0.686))
        };

        self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.dataflow_btn_container.dataflow_btn)).set_text(cx, status_text);
        self.view.button(ids!(content_area.main_content.content.fm_page.mofa_hero.dataflow_btn_container.dataflow_btn)).apply_over(cx, live! {
            draw_bg: { status: (status_val) }
        });
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.connection_dot)).apply_over(cx, live! {
            draw_bg: { color: (vec4(dot_color.0, dot_color.1, dot_color.2, 1.0)) }
        });

        // Update buffer gauge, percentage label, and status dot
        let buffer_pct = state.buffer_fill / 100.0;
        let buffer_critical = if state.buffer_fill >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.buffer_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (buffer_pct) }
        });
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.buffer_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (buffer_critical) }
        });
        self.view.label(ids!(content_area.main_content.content.fm_page.mofa_hero.buffer_pct_label)).set_text_with(|t| {
            *t = format!("{:.0}%", state.buffer_fill)
        });

        // Update CPU gauge, percentage label, and status dot
        let cpu_pct = (state.cpu_usage / 100.0) as f64;
        let cpu_critical = if state.cpu_usage >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.cpu_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (cpu_pct) }
        });
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.cpu_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (cpu_critical) }
        });
        self.view.label(ids!(content_area.main_content.content.fm_page.mofa_hero.cpu_pct_label)).set_text_with(|t| {
            *t = format!("{:.0}%", state.cpu_usage)
        });

        // Update Memory gauge, percentage label, and status dot
        let memory_pct = (state.memory_usage / 100.0) as f64;
        let memory_critical = if state.memory_usage >= 80.0 { 1.0 } else { 0.0 };
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.memory_gauge)).apply_over(cx, live! {
            draw_bg: { fill_pct: (memory_pct) }
        });
        self.view.view(ids!(content_area.main_content.content.fm_page.mofa_hero.memory_status_dot)).apply_over(cx, live! {
            draw_bg: { critical: (memory_critical) }
        });
        self.view.label(ids!(content_area.main_content.content.fm_page.mofa_hero.memory_pct_label)).set_text_with(|t| {
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

        let panel_ids: [&[LiveId]; 3] = [ids!(content_area.main_content.content.fm_page.participant_bar.student1_panel), ids!(content_area.main_content.content.fm_page.participant_bar.student2_panel), ids!(content_area.main_content.content.fm_page.participant_bar.tutor_panel)];

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
        self.view.markdown(ids!(content_area.main_content.content.fm_page.chat_section.chat_scroll.chat_content)).set_text(cx, &chat_text);

        // Auto-scroll chat to bottom only when NEW messages arrive
        let chat_count = filtered_messages.len();
        if chat_count > self.last_chat_count {
            self.view.view(ids!(content_area.main_content.content.fm_page.chat_section.chat_scroll)).set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
            self.last_chat_count = chat_count;
        }

        // Apply log filters (level, node, and search text)
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Get search text from input field
        let search_text = self.view.text_input(ids!(content_area.log_panel.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
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
        self.view.markdown(ids!(content_area.log_panel.log_content_column.log_body.log_scroll.log_content)).set_text(cx, &log_text);

        // Only auto-scroll when NEW logs arrive (not on filter changes)
        // This allows users to scroll up and read old logs
        if log_count > self.last_log_count {
            self.last_log_count = log_count;
            self.view.view(ids!(content_area.log_panel.log_content_column.log_body.log_scroll)).set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
        }

        // Update mic level LED meter from shared state
        let mic_level = state.mic_input_level;
        drop(state); // Release lock before updating UI
        self.update_mic_level_meter(cx, mic_level);

        self.view.redraw(cx);
    }

    /// Handle send prompt button click
    fn on_send_prompt_clicked(&mut self, cx: &mut Cx) {
        // Get text from input widget, use default if empty
        let input_text = self.view.text_input(ids!(content_area.main_content.content.fm_page.prompt_section.prompt_input)).text();
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
        self.view.text_input(ids!(content_area.main_content.content.fm_page.prompt_section.prompt_input)).set_text(cx, "");
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
        let search_text = self.view.text_input(ids!(content_area.log_panel.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
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
            self.view.view(ids!(content_area.log_panel)).apply_over(cx, live!{
                width: 32
            });
            self.view.view(ids!(content_area.splitter)).apply_over(cx, live!{
                width: 0
            });
            self.view.view(ids!(content_area.log_panel.log_content_column)).apply_over(cx, live!{
                visible: false
            });
            self.view.button(ids!(content_area.log_panel.toggle_column.toggle_log_btn)).set_text(cx, "<");
        } else {
            // Expanded: restore to saved width, show ">" to collapse
            let width = self.log_panel_width;
            self.view.view(ids!(content_area.log_panel)).apply_over(cx, live!{
                width: (width)
            });
            self.view.view(ids!(content_area.splitter)).apply_over(cx, live!{
                width: 6
            });
            self.view.view(ids!(content_area.log_panel.log_content_column)).apply_over(cx, live!{
                visible: true
            });
            self.view.button(ids!(content_area.log_panel.toggle_column.toggle_log_btn)).set_text(cx, ">");
        }

        self.view.redraw(cx);
    }

    /// Toggle sidebar visibility (click to show/hide)
    fn toggle_sidebar(&mut self, cx: &mut Cx) {
        if self.sidebar_collapsed {
            // Show sidebar
            self.sidebar_collapsed = false;
            self.view.view(ids!(sidebar_zone.sidebar_content)).apply_over(cx, live!{
                width: 234
            });
        } else {
            // Hide sidebar
            self.sidebar_collapsed = true;
            self.view.view(ids!(sidebar_zone.sidebar_content)).apply_over(cx, live!{
                width: 0
            });
        }
        self.view.redraw(cx);
    }

    /// Show FM page
    fn show_fm_page(&mut self, cx: &mut Cx) {
        self.view.view(ids!(content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: true });
        self.view.view(ids!(content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: false });
        self.view.redraw(cx);
    }

    /// Show App page (blank)
    fn show_app_page(&mut self, cx: &mut Cx) {
        self.view.view(ids!(content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: false });
        self.view.view(ids!(content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: true });
        self.view.redraw(cx);
    }

    /// Populate audio device dropdowns from cpal enumeration in shared state
    fn populate_device_dropdowns_from_cpal(&mut self, cx: &mut Cx) {
        let Some(state_ref) = SHARED_STATE.get() else { return };
        let state = state_ref.lock();

        // Update input device dropdown
        if !state.input_devices.is_empty() {
            let input_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.input_device_dropdown));
            input_dropdown.set_labels(cx, state.input_devices.clone());
            input_dropdown.set_selected_item(cx, state.selected_input_device);
        }

        // Update output device dropdown
        if !state.output_devices.is_empty() {
            let output_dropdown = self.view.drop_down(ids!(content_area.main_content.content.fm_page.audio_panel.output_device_dropdown));
            output_dropdown.set_labels(cx, state.output_devices.clone());
            output_dropdown.set_selected_item(cx, state.selected_output_device);
        }

        self.view.redraw(cx);
    }

    /// Update mic level LED meter based on input level (0.0 - 1.0)
    fn update_mic_level_meter(&mut self, cx: &mut Cx, level: f32) {
        // Calculate which LEDs should be lit based on level
        // LED 1: 0-20%, LED 2: 20-40%, LED 3: 40-60%, LED 4: 60-80%, LED 5: 80-100%
        let level = level.clamp(0.0, 1.0);

        // Colors for lit and dim states (matches buffer gauge: blue -> cyan -> green -> yellow -> red)
        let lit_colors = [0x3366f2u32, 0x33bfe6u32, 0x4dd973u32, 0xe6cc33u32, 0xf23333u32];
        let dim_color = 0xd9d9e0u32; // Light gray when not lit (matches CPU gauge)

        // Update each LED
        for i in 0..5 {
            let is_lit = level >= (i as f32 * 0.2);
            let color = if is_lit { lit_colors[i] } else { dim_color };

            let led_id = match i {
                0 => ids!(content_area.main_content.content.fm_page.audio_panel.mic_level_meter.mic_led_1),
                1 => ids!(content_area.main_content.content.fm_page.audio_panel.mic_level_meter.mic_led_2),
                2 => ids!(content_area.main_content.content.fm_page.audio_panel.mic_level_meter.mic_led_3),
                3 => ids!(content_area.main_content.content.fm_page.audio_panel.mic_level_meter.mic_led_4),
                4 => ids!(content_area.main_content.content.fm_page.audio_panel.mic_level_meter.mic_led_5),
                _ => continue,
            };

            // Convert u32 color to vec4
            let r = ((color >> 16) & 0xFF) as f32 / 255.0;
            let g = ((color >> 8) & 0xFF) as f32 / 255.0;
            let b = (color & 0xFF) as f32 / 255.0;

            self.view.view(led_id).apply_over(cx, live!{
                draw_bg: { color: (vec4(r, g, b, 1.0)) }
            });
        }
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,

    #[rust]
    user_menu_open: bool,

    #[rust]
    sidebar_menu_open: bool,
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

        // Handle user menu hover at App level
        let user_btn = self.ui.view(ids!(user_btn_overlay));
        let user_menu = self.ui.view(ids!(user_menu));

        // Show menu when hovering over button
        match event.hits(cx, user_btn.area()) {
            Hit::FingerHoverIn(_) => {
                if !self.user_menu_open {
                    self.user_menu_open = true;
                    user_menu.set_visible(cx, true);
                    self.ui.redraw(cx);
                }
            }
            _ => {}
        }

        // Hide menu when mouse moves away from both button and menu
        if self.user_menu_open {
            // Check if we need to close - only on MouseMove events
            if let Event::MouseMove(mm) = event {
                let btn_rect = user_btn.area().rect(cx);
                let menu_rect = user_menu.area().rect(cx);

                // Expand rects slightly for tolerance
                let in_btn = mm.abs.x >= btn_rect.pos.x - 5.0
                    && mm.abs.x <= btn_rect.pos.x + btn_rect.size.x + 5.0
                    && mm.abs.y >= btn_rect.pos.y - 5.0
                    && mm.abs.y <= btn_rect.pos.y + btn_rect.size.y + 10.0;

                let in_menu = mm.abs.x >= menu_rect.pos.x - 5.0
                    && mm.abs.x <= menu_rect.pos.x + menu_rect.size.x + 5.0
                    && mm.abs.y >= menu_rect.pos.y - 5.0
                    && mm.abs.y <= menu_rect.pos.y + menu_rect.size.y + 5.0;

                if !in_btn && !in_menu {
                    self.user_menu_open = false;
                    user_menu.set_visible(cx, false);
                    self.ui.redraw(cx);
                }
            }
        }

        // Handle sidebar hover at App level (using overlay)
        let sidebar_trigger = self.ui.view(ids!(sidebar_trigger_overlay));
        let sidebar_menu = self.ui.view(ids!(sidebar_menu_overlay));

        // Show sidebar when hovering over trigger (hamburger button)
        match event.hits(cx, sidebar_trigger.area()) {
            Hit::FingerHoverIn(_) => {
                if !self.sidebar_menu_open {
                    self.sidebar_menu_open = true;
                    sidebar_menu.set_visible(cx, true);
                    self.ui.redraw(cx);
                }
            }
            _ => {}
        }

        // Hide sidebar when mouse moves away from both trigger and sidebar
        if self.sidebar_menu_open {
            if let Event::MouseMove(mm) = event {
                let trigger_rect = sidebar_trigger.area().rect(cx);
                let sidebar_rect = sidebar_menu.area().rect(cx);

                // Check if mouse is in trigger area (with tolerance)
                let in_trigger = mm.abs.x >= trigger_rect.pos.x - 5.0
                    && mm.abs.x <= trigger_rect.pos.x + trigger_rect.size.x + 5.0
                    && mm.abs.y >= trigger_rect.pos.y - 5.0
                    && mm.abs.y <= trigger_rect.pos.y + trigger_rect.size.y + 5.0;

                // Check if mouse is in sidebar area (with tolerance)
                let in_sidebar = mm.abs.x >= sidebar_rect.pos.x - 5.0
                    && mm.abs.x <= sidebar_rect.pos.x + sidebar_rect.size.x + 10.0
                    && mm.abs.y >= sidebar_rect.pos.y - 5.0
                    && mm.abs.y <= sidebar_rect.pos.y + sidebar_rect.size.y + 5.0;

                if !in_trigger && !in_sidebar {
                    self.sidebar_menu_open = false;
                    sidebar_menu.set_visible(cx, false);
                    self.ui.redraw(cx);
                }
            }
        }

        // Handle user menu item clicks
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

        if self.ui.button(ids!(user_menu.menu_profile_btn)).clicked(actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.ui.redraw(cx);
        }
        if self.ui.button(ids!(user_menu.menu_settings_btn)).clicked(actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.ui.redraw(cx);
        }
        if self.ui.button(ids!(user_menu.menu_logout_btn)).clicked(actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.ui.redraw(cx);
        }

        // Handle sidebar menu item clicks (sidebar is overlay at Window level)
        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.mofa_fm_tab)).clicked(actions) {
            self.sidebar_menu_open = false;
            self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, false);
            // Show FM page
            self.ui.view(ids!(body.content_area.main_content.content.fm_page)).set_visible(cx, true);
            self.ui.view(ids!(body.content_area.main_content.content.app_page)).set_visible(cx, false);
            self.ui.redraw(cx);
        }
        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.settings_tab)).clicked(actions) {
            self.sidebar_menu_open = false;
            self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, false);
            self.ui.redraw(cx);
        }

        // Handle sidebar app button clicks
        let app_buttons = [
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app1_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app2_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app3_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app4_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app5_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app6_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app7_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app8_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app9_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app10_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app11_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app12_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app13_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app14_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app15_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app16_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app17_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app18_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app19_btn),
            ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app20_btn),
        ];
        for app_id in app_buttons {
            if self.ui.button(app_id).clicked(actions) {
                self.sidebar_menu_open = false;
                self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, false);
                // Show app page
                self.ui.view(ids!(body.content_area.main_content.content.fm_page)).set_visible(cx, false);
                self.ui.view(ids!(body.content_area.main_content.content.app_page)).set_visible(cx, true);
                self.ui.redraw(cx);
                break;
            }
        }
    }
}

app_main!(App);

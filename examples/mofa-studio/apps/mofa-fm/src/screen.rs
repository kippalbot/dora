//! MoFA FM Screen - Main screen for AI-powered audio streaming

use makepad_widgets::*;
use crate::mofa_hero::MofaHeroWidgetExt;
use mofa_widgets::participant_panel::ParticipantPanelWidgetExt;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;
    use mofa_widgets::participant_panel::ParticipantPanel;
    use mofa_widgets::log_panel::LogPanel;
    use crate::mofa_hero::MofaHero;

    // Local layout constants (colors imported from theme)
    SECTION_SPACING = 12.0
    PANEL_RADIUS = 4.0
    PANEL_PADDING = 12.0

    // Reusable panel header style with dark mode support
    PanelHeader = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        align: {y: 0.5}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((SLATE_50), (SLATE_800), self.dark_mode);
            }
        }
    }

    // Reusable vertical divider
    VerticalDivider = <View> {
        width: 1, height: Fill
        margin: {top: 4, bottom: 4}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
            }
        }
    }

    // MoFA FM Screen - adaptive horizontal layout with left content and right log panel
    pub MoFaFMScreen = {{MoFaFMScreen}} {
        width: Fill, height: Fill
        flow: Right
        spacing: 0
        padding: { left: 16, right: 16, top: 16, bottom: 16 }
        align: {y: 0.0}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Left column - main content area (adaptive width)
        left_column = <View> {
            width: Fill, height: Fill
            flow: Down
            spacing: (SECTION_SPACING)
            align: {y: 0.0}

            // System status bar (self-contained widget)
            mofa_hero = <MofaHero> {
                width: Fill
            }

            // Participant status cards container
            participant_container = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8

                participant_bar = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: (SECTION_SPACING)

                    student1_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                    }
                    student2_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                    }
                    tutor_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                    }
                }
            }

            // Chat window container (fills remaining space)
            chat_container = <View> {
                width: Fill, height: Fill
                flow: Down

                chat_section = <RoundedView> {
                    width: Fill, height: Fill
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    flow: Down

                    // Chat header
                    chat_header = <PanelHeader> {
                        chat_title = <Label> {
                            text: "Chat History"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }

                    // Chat messages area (scrollable, fills space)
                    chat_scroll = <ScrollYView> {
                        width: Fill, height: Fill
                        flow: Down
                        scroll_bars: <ScrollBars> {
                            show_scroll_x: false
                            show_scroll_y: true
                        }

                        chat_content_wrapper = <View> {
                            width: Fill, height: Fit
                            padding: (PANEL_PADDING)
                            flow: Down

                            chat_content = <Markdown> {
                                width: Fill, height: Fit
                                font_size: 13.0
                                font_color: (TEXT_PRIMARY)
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
            }

            // Audio control panel container
            audio_container = <View> {
                width: Fill, height: Fit
                flow: Down

                audio_panel = <RoundedView> {
                    width: Fill, height: Fit
                    padding: (PANEL_PADDING)
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    flow: Right
                    spacing: 16
                    align: {y: 0.5}

                    // Mic level meter group
                    mic_group = <View> {
                        width: Fit, height: Fit
                        flow: Right
                        spacing: 10
                        align: {y: 0.5}
                        padding: {right: 8}

                        mic_mute_btn = <View> {
                            width: Fit, height: Fit
                            flow: Overlay
                            cursor: Hand
                            padding: 4

                            mic_icon_on = <View> {
                                width: Fit, height: Fit
                                <Icon> {
                                    draw_icon: {
                                        svg_file: dep("crate://self/resources/icons/mic.svg")
                                        fn get_color(self) -> vec4 { return (SLATE_500); }
                                    }
                                    icon_walk: {width: 20, height: 20}
                                }
                            }
                        }

                        mic_level_meter = <View> {
                            width: Fit, height: Fit
                            flow: Right
                            spacing: 3
                            align: {y: 0.5}
                            padding: {top: 2, bottom: 2}

                            mic_led_1 = <RoundedView> { width: 8, height: 14, draw_bg: { color: (GREEN_500), border_radius: 2.0 } }
                            mic_led_2 = <RoundedView> { width: 8, height: 14, draw_bg: { color: (GREEN_500), border_radius: 2.0 } }
                            mic_led_3 = <RoundedView> { width: 8, height: 14, draw_bg: { color: (SLATE_200), border_radius: 2.0 } }
                            mic_led_4 = <RoundedView> { width: 8, height: 14, draw_bg: { color: (SLATE_200), border_radius: 2.0 } }
                            mic_led_5 = <RoundedView> { width: 8, height: 14, draw_bg: { color: (SLATE_200), border_radius: 2.0 } }
                        }
                    }

                    <VerticalDivider> {}

                    // AEC toggle group
                    aec_group = <View> {
                        width: Fit, height: Fit
                        flow: Right
                        spacing: 8
                        align: {y: 0.5}
                        padding: {left: 8, right: 8}

                        aec_toggle_btn = <View> {
                            width: Fit, height: Fit
                            padding: 6
                            flow: Overlay
                            cursor: Hand
                            show_bg: true
                            draw_bg: {
                                instance enabled: 1.0  // 1.0=on, 0.0=off
                                // Blink animation now driven by shader time - no timer needed!
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                    let green = vec4(0.133, 0.773, 0.373, 1.0);
                                    let bright = vec4(0.2, 0.9, 0.5, 1.0);
                                    let gray = vec4(0.667, 0.686, 0.725, 1.0);
                                    // When enabled, pulse between green and bright green using shader time
                                    // sin(time * speed) creates smooth oscillation, step makes it blink
                                    let blink = step(0.0, sin(self.time * 2.0)) * self.enabled;
                                    let base = mix(gray, green, self.enabled);
                                    let col = mix(base, bright, blink * 0.5);
                                    sdf.fill(col);
                                    return sdf.result;
                                }
                            }
                            align: {x: 0.5, y: 0.5}

                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/aec.svg")
                                    fn get_color(self) -> vec4 { return (WHITE); }
                                }
                                icon_walk: {width: 20, height: 20}
                            }
                        }
                    }

                    <VerticalDivider> {}

                    // Device selectors container - fills remaining space
                    device_selectors = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 16
                        align: {y: 0.5}

                        // Input device group (fills available space)
                        input_device_group = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: 8
                            align: {y: 0.5}

                            input_device_label = <Label> {
                                width: 70  // Fixed width for alignment with output label
                                text: "Mic:"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }

                            input_device_dropdown = <DropDown> {
                                width: Fill, height: Fit
                                padding: {left: 10, right: 10, top: 6, bottom: 6}
                                popup_menu_position: BelowInput
                                // Labels will be set at runtime by init_audio()
                                labels: []
                                values: []
                                selected_item: 0
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 3.0);
                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                        sdf.fill(bg);
                                        return sdf.result;
                                    }
                                }
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        let light = mix((GRAY_700), (TEXT_PRIMARY), self.focus);
                                        let dark = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.focus);
                                        return mix(light, dark, self.dark_mode);
                                    }
                                }
                                popup_menu: {
                                    width: 250  // Initial width - will be synced at runtime
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        border_size: 1.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                            let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                            sdf.fill(bg);
                                            sdf.stroke(border, self.border_size);
                                            return sdf.result;
                                        }
                                    }
                                    menu_item: {
                                        width: Fill
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                                let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                                sdf.fill(mix(base, hover_color, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            fn get_color(self) -> vec4 {
                                                let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                                let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                                let base = mix(light_base, dark_base, self.dark_mode);
                                                let light_hover = (TEXT_PRIMARY);
                                                let dark_hover = (TEXT_PRIMARY_DARK);
                                                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                                return mix(base, hover_color, self.hover);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        <VerticalDivider> {}

                        // Output device group (fills available space)
                        output_device_group = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: 8
                            align: {y: 0.5}

                            output_device_label = <Label> {
                                width: 70  // Fixed width for alignment with input label
                                text: "Speaker:"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }

                            output_device_dropdown = <DropDown> {
                                width: Fill, height: Fit
                                padding: {left: 10, right: 10, top: 6, bottom: 6}
                                popup_menu_position: BelowInput
                                // Labels will be set at runtime by init_audio()
                                labels: []
                                values: []
                                selected_item: 0
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 3.0);
                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                        sdf.fill(bg);
                                        return sdf.result;
                                    }
                                }
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        let light = mix((GRAY_700), (TEXT_PRIMARY), self.focus);
                                        let dark = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.focus);
                                        return mix(light, dark, self.dark_mode);
                                    }
                                }
                                popup_menu: {
                                    width: 250  // Initial width - will be synced at runtime
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        border_size: 1.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                            let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                            sdf.fill(bg);
                                            sdf.stroke(border, self.border_size);
                                            return sdf.result;
                                        }
                                    }
                                    menu_item: {
                                        width: Fill
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                                let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                                sdf.fill(mix(base, hover_color, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            fn get_color(self) -> vec4 {
                                                let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                                let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                                let base = mix(light_base, dark_base, self.dark_mode);
                                                let light_hover = (TEXT_PRIMARY);
                                                let dark_hover = (TEXT_PRIMARY_DARK);
                                                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                                return mix(base, hover_color, self.hover);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Prompt input area container
            prompt_container = <View> {
                width: Fill, height: Fit
                flow: Down

                prompt_section = <RoundedView> {
                    width: Fill, height: Fit
                    padding: (PANEL_PADDING)
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    flow: Down
                    spacing: 8

                    prompt_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 12
                        align: {y: 0.5}

                        prompt_input = <TextInput> {
                            width: Fill, height: Fit
                            padding: {left: 12, right: 12, top: 10, bottom: 10}
                            empty_text: "Enter prompt to send..."
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 4.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                    let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            draw_selection: {
                                color: (INDIGO_200)
                            }
                        }

                        button_group = <View> {
                            width: Fit, height: Fit
                            flow: Right
                            spacing: 8

                            send_prompt_btn = <Button> {
                                width: Fit, height: 35
                                padding: {left: 16, right: 16}
                                text: "Send"
                                draw_text: {
                                    color: (WHITE)
                                    text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                                }
                                draw_bg: {
                                    instance color: (ACCENT_BLUE)
                                    instance color_hover: (BLUE_700)
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

                            reset_btn = <Button> {
                                width: Fit, height: 35
                                padding: {left: 16, right: 16}
                                text: "Reset"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((GRAY_700), (SLATE_300), self.dark_mode);
                                    }
                                }
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    border_radius: 4.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                        let base = mix((HOVER_BG), (SLATE_600), self.dark_mode);
                                        let hover_color = mix((SLATE_200), (SLATE_500), self.dark_mode);
                                        sdf.fill(mix(base, hover_color, self.hover));
                                        return sdf.result;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Splitter - draggable handle with padding
        splitter = <View> {
            width: 16, height: Fill
            margin: { left: 8, right: 8 }
            align: {y: 0.0}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    // Draw thin line in center
                    sdf.rect(7.0, 16.0, 2.0, self.rect_size.y - 32.0);
                    let color = mix((SLATE_300), (SLATE_600), self.dark_mode);
                    sdf.fill(color);
                    return sdf.result;
                }
            }
            cursor: ColResize
        }

        // System Log panel - adaptive width, top-aligned
        log_section = <View> {
            width: 320, height: Fill
            flow: Right
            align: {y: 0.0}

            // Toggle button column
            toggle_column = <View> {
                width: Fit, height: Fill
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((SLATE_50), (SLATE_800), self.dark_mode);
                    }
                }
                align: {x: 0.5, y: 0.0}
                padding: {left: 4, right: 4, top: 8}

                toggle_log_btn = <Button> {
                    width: Fit, height: Fit
                    padding: {left: 8, right: 8, top: 6, bottom: 6}
                    text: ">"
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_BOLD>{ font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((SLATE_500), (SLATE_400), self.dark_mode);
                        }
                    }
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 4.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let base = mix((SLATE_200), (SLATE_600), self.dark_mode);
                            let hover_color = mix((SLATE_300), (SLATE_500), self.dark_mode);
                            sdf.fill(mix(base, hover_color, self.hover));
                            return sdf.result;
                        }
                    }
                }
            }

            // Log content panel
            log_content_column = <RoundedView> {
                width: Fill, height: Fill
                draw_bg: {
                    instance dark_mode: 0.0
                    border_radius: (PANEL_RADIUS)
                    fn get_color(self) -> vec4 {
                        return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                    }
                }
                flow: Down

                log_header = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            return mix((SLATE_50), (SLATE_800), self.dark_mode);
                        }
                    }

                    // Title row
                    log_title_row = <View> {
                        width: Fill, height: Fit
                        padding: {left: 12, right: 12, top: 10, bottom: 6}
                        log_title_label = <Label> {
                            text: "System Log"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }

                    // Filter row
                    log_filter_row = <View> {
                        width: Fill, height: 32
                        flow: Right
                        align: {y: 0.5}
                        padding: {left: 8, right: 8, bottom: 6}
                        spacing: 6

                        // Level filter dropdown
                        level_filter = <DropDown> {
                            width: 70, height: 24
                            popup_menu_position: BelowInput
                            draw_bg: {
                                color: (HOVER_BG)
                                border_color: (SLATE_200)
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    // Background
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                    sdf.fill((HOVER_BG));
                                    // Down arrow on right side
                                    let ax = self.rect_size.x - 12.0;
                                    let ay = self.rect_size.y * 0.5 - 2.0;
                                    sdf.move_to(ax - 3.0, ay);
                                    sdf.line_to(ax, ay + 4.0);
                                    sdf.line_to(ax + 3.0, ay);
                                    sdf.stroke((TEXT_PRIMARY), 1.5);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return (TEXT_PRIMARY);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    color: (WHITE)
                                    border_color: (BORDER)
                                    border_size: 1.0
                                    border_radius: 2.0
                                }
                                menu_item: {
                                    draw_bg: {
                                        color: (WHITE)
                                        color_hover: (GRAY_100)
                                    }
                                    draw_text: {
                                        fn get_color(self) -> vec4 {
                                            return mix(
                                                mix((GRAY_700), (TEXT_PRIMARY), self.active),
                                                (TEXT_PRIMARY),
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
                            width: 85, height: 24
                            popup_menu_position: BelowInput
                            draw_bg: {
                                color: (HOVER_BG)
                                border_color: (SLATE_200)
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    // Background
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                    sdf.fill((HOVER_BG));
                                    // Down arrow on right side
                                    let ax = self.rect_size.x - 12.0;
                                    let ay = self.rect_size.y * 0.5 - 2.0;
                                    sdf.move_to(ax - 3.0, ay);
                                    sdf.line_to(ax, ay + 4.0);
                                    sdf.line_to(ax + 3.0, ay);
                                    sdf.stroke((TEXT_PRIMARY), 1.5);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return (TEXT_PRIMARY);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    color: (WHITE)
                                    border_color: (BORDER)
                                    border_size: 1.0
                                    border_radius: 2.0
                                }
                                menu_item: {
                                    draw_bg: {
                                        color: (WHITE)
                                        color_hover: (GRAY_100)
                                    }
                                    draw_text: {
                                        fn get_color(self) -> vec4 {
                                            return mix(
                                                mix((GRAY_700), (TEXT_PRIMARY), self.active),
                                                (TEXT_PRIMARY),
                                                self.hover
                                            );
                                        }
                                    }
                                }
                            }
                            labels: ["All Nodes", "ASR", "TTS", "LLM", "Bridge", "Monitor", "App"]
                            values: [ALL, ASR, TTS, LLM, BRIDGE, MONITOR, APP]
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
                                    sdf.stroke((GRAY_500), 1.5);
                                    // Handle
                                    sdf.move_to(c.x + 1.5, c.y + 1.5);
                                    sdf.line_to(c.x + 6.0, c.y + 6.0);
                                    sdf.stroke((GRAY_500), 1.5);
                                    return sdf.result;
                                }
                            }
                        }

                        // Search field
                        log_search = <TextInput> {
                            width: Fill, height: 24
                            empty_text: "Search..."
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                    let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            draw_selection: {
                                color: (INDIGO_200)
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
                                    let bg_color = mix((BORDER), (GRAY_300), self.hover);
                                    let bg_color = mix(bg_color, (TEXT_MUTED), self.pressed);
                                    sdf.fill(bg_color);

                                    // Clipboard icon - back rectangle
                                    let icon_color = (GRAY_600);
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

                log_scroll = <ScrollYView> {
                    width: Fill, height: Fill
                    flow: Down
                    scroll_bars: <ScrollBars> {
                        show_scroll_x: false
                        show_scroll_y: true
                    }

                    log_content_wrapper = <View> {
                        width: Fill, height: Fit
                        padding: { left: 12, right: 12, top: 8, bottom: 8 }
                        flow: Down

                        log_content = <Markdown> {
                            width: Fill, height: Fit
                            font_size: 10.0
                            font_color: (GRAY_600)
                            paragraph_spacing: 4

                            draw_normal: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_600), (SLATE_300), self.dark_mode);
                                }
                            }
                            draw_bold: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_600), (SLATE_300), self.dark_mode);
                                }
                            }
                            draw_fixed: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 9.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_600), (SLATE_300), self.dark_mode);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MoFaFMScreen {
    #[deref]
    view: View,
    #[rust]
    log_panel_collapsed: bool,
    #[rust]
    log_panel_width: f64,
    #[rust]
    splitter_dragging: bool,
    #[rust]
    audio_manager: Option<crate::audio::AudioManager>,
    #[rust]
    audio_timer: Timer,
    #[rust]
    audio_initialized: bool,
    #[rust]
    input_devices: Vec<String>,
    #[rust]
    output_devices: Vec<String>,
    #[rust]
    log_level_filter: usize,  // 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
    #[rust]
    log_node_filter: usize,   // 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
    #[rust]
    log_entries: Vec<String>,  // Raw log entries for filtering

    // Dropdown width caching for popup menu sync
    #[rust]
    dropdown_widths_initialized: bool,
    #[rust]
    cached_input_dropdown_width: f64,
    #[rust]
    cached_output_dropdown_width: f64,

    // AEC toggle state
    #[rust]
    aec_enabled: bool,
    // Note: AEC blink animation is now shader-driven (self.time), no timer needed
}

impl Widget for MoFaFMScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Initialize audio on first event
        if !self.audio_initialized {
            self.init_audio(cx);
            self.audio_initialized = true;
        }

        // Handle audio timer for mic level updates
        if self.audio_timer.is_event(event).is_some() {
            self.update_mic_level(cx);
        }

        // Handle AEC toggle button click
        // Note: AEC blink animation is now shader-driven, no timer needed
        let aec_btn = self.view.view(ids!(audio_container.audio_panel.aec_group.aec_toggle_btn));
        match event.hits(cx, aec_btn.area()) {
            Hit::FingerUp(_) => {
                self.aec_enabled = !self.aec_enabled;
                let enabled_val = if self.aec_enabled { 1.0 } else { 0.0 };
                self.view.view(ids!(audio_container.audio_panel.aec_group.aec_toggle_btn))
                    .apply_over(cx, live!{ draw_bg: { enabled: (enabled_val) } });
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle splitter drag
        let splitter = self.view.view(ids!(splitter));
        match event.hits(cx, splitter.area()) {
            Hit::FingerDown(_) => {
                self.splitter_dragging = true;
            }
            Hit::FingerMove(fm) => {
                if self.splitter_dragging {
                    self.resize_log_panel(cx, fm.abs.x);
                }
            }
            Hit::FingerUp(_) => {
                self.splitter_dragging = false;
            }
            _ => {}
        }

        // Handle actions
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

        // Handle toggle log panel button
        if self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).clicked(actions) {
            self.toggle_log_panel(cx);
        }

        // Handle input device selection
        if let Some(item) = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.input_device_group.input_device_dropdown)).selected(actions) {
            if item < self.input_devices.len() {
                let device_name = self.input_devices[item].clone();
                self.select_input_device(cx, &device_name);
            }
        }

        // Handle output device selection
        if let Some(item) = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.output_device_group.output_device_dropdown)).selected(actions) {
            if item < self.output_devices.len() {
                let device_name = self.output_devices[item].clone();
                self.select_output_device(&device_name);
            }
        }

        // Handle log level filter dropdown
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.level_filter)).selected(actions) {
            self.log_level_filter = selected;
            self.update_log_display(cx);
        }

        // Handle log node filter dropdown
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.node_filter)).selected(actions) {
            self.log_node_filter = selected;
            self.update_log_display(cx);
        }

        // Handle copy log button
        if self.view.button(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn)).clicked(actions) {
            self.copy_logs_to_clipboard(cx);
        }

        // Handle log search text change
        if self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).changed(actions).is_some() {
            self.update_log_display(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Update popup menu widths to match dropdown widths
        // This handles first-frame zero width and caches values for performance
        let input_dropdown = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.input_device_group.input_device_dropdown));
        let input_width = input_dropdown.area().rect(cx).size.x;

        // Only update if width changed significantly (> 1px) to avoid unnecessary apply_over calls
        if input_width > 0.0 && (input_width - self.cached_input_dropdown_width).abs() > 1.0 {
            self.cached_input_dropdown_width = input_width;
            input_dropdown.apply_over(cx, live! {
                popup_menu: { width: (input_width) }
            });
        }

        let output_dropdown = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.output_device_group.output_device_dropdown));
        let output_width = output_dropdown.area().rect(cx).size.x;

        // Only update if width changed significantly (> 1px)
        if output_width > 0.0 && (output_width - self.cached_output_dropdown_width).abs() > 1.0 {
            self.cached_output_dropdown_width = output_width;
            output_dropdown.apply_over(cx, live! {
                popup_menu: { width: (output_width) }
            });
        }

        // Force an extra redraw on first frame to ensure widths are properly captured
        // This fixes the issue where first click shows narrow popup (width=0 on first frame)
        if !self.dropdown_widths_initialized {
            self.dropdown_widths_initialized = true;
            self.view.redraw(cx);
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl MoFaFMScreen {
    /// Initialize audio manager and populate device dropdowns
    fn init_audio(&mut self, cx: &mut Cx) {
        let mut audio_manager = crate::audio::AudioManager::new();

        // Get input devices
        let input_devices = audio_manager.get_input_devices();
        let input_labels: Vec<String> = input_devices.iter().map(|d| {
            if d.is_default {
                format!("{} (Default)", d.name)
            } else {
                d.name.clone()
            }
        }).collect();
        self.input_devices = input_devices.iter().map(|d| d.name.clone()).collect();

        // Get output devices
        let output_devices = audio_manager.get_output_devices();
        let output_labels: Vec<String> = output_devices.iter().map(|d| {
            if d.is_default {
                format!("{} (Default)", d.name)
            } else {
                d.name.clone()
            }
        }).collect();
        self.output_devices = output_devices.iter().map(|d| d.name.clone()).collect();

        // Populate input dropdown
        if !input_labels.is_empty() {
            let dropdown = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.input_device_group.input_device_dropdown));
            dropdown.set_labels(cx, input_labels);
            dropdown.set_selected_item(cx, 0);
        }

        // Populate output dropdown
        if !output_labels.is_empty() {
            let dropdown = self.view.drop_down(ids!(audio_container.audio_panel.device_selectors.output_device_group.output_device_dropdown));
            dropdown.set_labels(cx, output_labels);
            dropdown.set_selected_item(cx, 0);
        }

        // Start mic monitoring with default device
        if let Err(e) = audio_manager.start_mic_monitoring(None) {
            eprintln!("Failed to start mic monitoring: {}", e);
        }

        self.audio_manager = Some(audio_manager);

        // Start timer for mic level updates (50ms for smooth visualization)
        self.audio_timer = cx.start_interval(0.05);

        // AEC enabled by default (blink animation is shader-driven, no timer needed)
        self.aec_enabled = true;

        // Initialize demo log entries
        self.init_demo_logs(cx);

        self.view.redraw(cx);
    }

    /// Initialize demo log entries to demonstrate the log panel functionality
    fn init_demo_logs(&mut self, cx: &mut Cx) {
        self.log_entries = vec![
            // System startup logs
            "[INFO] [App] MoFA Studio v0.1.0 started".to_string(),
            "[INFO] [App] Loading configuration from ~/.dora/dashboard/preferences.json".to_string(),
            "[DEBUG] [App] Initializing audio subsystem".to_string(),
            "[INFO] [App] Audio devices enumerated successfully".to_string(),

            // ASR (Speech Recognition) logs
            "[INFO] [ASR] FunASR engine initialized with GPU acceleration".to_string(),
            "[DEBUG] [ASR] Model loaded: speech_seaco_paraformer_large_asr_nat-zh-cn-16k".to_string(),
            "[INFO] [ASR] Punctuation model loaded: punc_ct-transformer_cn-en-common".to_string(),
            "[DEBUG] [ASR] GPU memory allocated: 2.1 GB".to_string(),
            "[INFO] [ASR] Ready to process audio stream".to_string(),
            "[DEBUG] [ASR] Processing 17.35s audio chunk...".to_string(),
            "[INFO] [ASR] Transcription complete: RTF=0.016, 61.6x realtime".to_string(),
            "[DEBUG] [ASR] Result: \"你好，欢迎使用MoFA语音助手\"".to_string(),

            // TTS (Text-to-Speech) logs
            "[INFO] [TTS] PrimeSpeech TTS engine initialized".to_string(),
            "[DEBUG] [TTS] Loading G2PW model for grapheme-to-phoneme conversion".to_string(),
            "[INFO] [TTS] Voice model loaded: Doubao".to_string(),
            "[DEBUG] [TTS] HiFiGAN vocoder ready".to_string(),
            "[INFO] [TTS] Synthesizing 160 characters...".to_string(),
            "[DEBUG] [TTS] Audio generated: 31.97s, processing time: 41.84s".to_string(),
            "[WARN] [TTS] CPU mode active - consider enabling GPU for faster synthesis".to_string(),

            // LLM (Language Model) logs
            "[INFO] [LLM] Connecting to OpenAI API...".to_string(),
            "[DEBUG] [LLM] API endpoint: https://api.openai.com/v1/chat/completions".to_string(),
            "[INFO] [LLM] Model: gpt-4o-mini".to_string(),
            "[DEBUG] [LLM] Sending prompt with 256 tokens".to_string(),
            "[INFO] [LLM] Response received: 128 tokens, latency 1.2s".to_string(),
            "[DEBUG] [LLM] Token usage: prompt=256, completion=128, total=384".to_string(),

            // Bridge/WebSocket logs
            "[INFO] [Bridge] WebSocket server started on ws://localhost:8123".to_string(),
            "[DEBUG] [Bridge] Waiting for client connections...".to_string(),
            "[INFO] [Bridge] Client connected from 127.0.0.1:52341".to_string(),
            "[DEBUG] [Bridge] Received audio frame: 1024 samples @ 16kHz".to_string(),
            "[INFO] [Bridge] Streaming response to client".to_string(),
            "[WARN] [Bridge] Client heartbeat delayed by 500ms".to_string(),

            // Monitor logs
            "[INFO] [Monitor] System monitoring started".to_string(),
            "[DEBUG] [Monitor] CPU usage: 23.5%".to_string(),
            "[DEBUG] [Monitor] Memory usage: 4.2 GB / 16.0 GB".to_string(),
            "[DEBUG] [Monitor] GPU utilization: 45%".to_string(),
            "[INFO] [Monitor] Audio buffer healthy: 85% filled".to_string(),
            "[WARN] [Monitor] High CPU spike detected: 78%".to_string(),
            "[DEBUG] [Monitor] Spike resolved, CPU back to normal".to_string(),

            // Dataflow logs
            "[INFO] [App] Dataflow 'voice-chat-with-aec' loaded".to_string(),
            "[DEBUG] [App] Nodes: websocket → asr → llm → tts → websocket".to_string(),
            "[INFO] [App] Dataflow started successfully".to_string(),
            "[DEBUG] [App] All nodes reporting healthy status".to_string(),

            // Error examples (for testing ERROR filter)
            "[ERROR] [ASR] Failed to process corrupted audio frame - skipping".to_string(),
            "[ERROR] [TTS] Synthesis timeout after 60s - retrying".to_string(),
            "[WARN] [LLM] Rate limit approaching: 45/50 requests per minute".to_string(),
            "[ERROR] [Bridge] Connection reset by peer - reconnecting".to_string(),

            // More activity logs
            "[INFO] [ASR] New audio segment detected (VAD triggered)".to_string(),
            "[DEBUG] [ASR] Segment duration: 3.2s".to_string(),
            "[INFO] [ASR] Transcription: \"请帮我查一下今天的天气\"".to_string(),
            "[INFO] [LLM] Processing user query...".to_string(),
            "[DEBUG] [LLM] Context window: 2048 tokens".to_string(),
            "[INFO] [LLM] Response generated".to_string(),
            "[INFO] [TTS] Generating audio response...".to_string(),
            "[DEBUG] [TTS] Text length: 89 characters".to_string(),
            "[INFO] [TTS] Audio ready: 8.5s duration".to_string(),
            "[INFO] [Bridge] Streaming audio to client".to_string(),
            "[DEBUG] [Bridge] Stream complete, 136 frames sent".to_string(),
            "[INFO] [Monitor] Round-trip latency: 2.8s".to_string(),
        ];

        // Update the log display
        self.update_log_display(cx);
    }
    /// Update mic level LEDs based on current audio input
    fn update_mic_level(&mut self, cx: &mut Cx) {
        let level = if let Some(ref audio_manager) = self.audio_manager {
            audio_manager.get_mic_level()
        } else {
            return;
        };

        // Map level (0.0-1.0) to 5 LEDs
        // Use non-linear scaling for better visualization (human hearing is logarithmic)
        let scaled_level = (level * 3.0).min(1.0); // Amplify for visibility
        let active_leds = (scaled_level * 5.0).ceil() as u32;

        // Colors as vec4: green=#22c55f, yellow=#eab308, orange=#f97316, red=#ef4444, off=#e2e8f0
        let green = vec4(0.133, 0.773, 0.373, 1.0);
        let yellow = vec4(0.918, 0.702, 0.031, 1.0);
        let orange = vec4(0.976, 0.451, 0.086, 1.0);
        let red = vec4(0.937, 0.267, 0.267, 1.0);
        let off = vec4(0.886, 0.910, 0.941, 1.0);

        // LED colors by index: 0,1=green, 2=yellow, 3=orange, 4=red
        let led_colors = [green, green, yellow, orange, red];
        let led_ids = [
            ids!(audio_container.audio_panel.mic_group.mic_level_meter.mic_led_1),
            ids!(audio_container.audio_panel.mic_group.mic_level_meter.mic_led_2),
            ids!(audio_container.audio_panel.mic_group.mic_level_meter.mic_led_3),
            ids!(audio_container.audio_panel.mic_group.mic_level_meter.mic_led_4),
            ids!(audio_container.audio_panel.mic_group.mic_level_meter.mic_led_5),
        ];

        for (i, led_id) in led_ids.iter().enumerate() {
            let is_active = (i + 1) as u32 <= active_leds;
            let color = if is_active { led_colors[i] } else { off };
            self.view.view(led_id.clone()).apply_over(cx, live! {
                draw_bg: { color: (color) }
            });
        }

        self.view.redraw(cx);
    }

    /// Select input device for mic monitoring
    fn select_input_device(&mut self, cx: &mut Cx, device_name: &str) {
        if let Some(ref mut audio_manager) = self.audio_manager {
            if let Err(e) = audio_manager.set_input_device(device_name) {
                eprintln!("Failed to set input device '{}': {}", device_name, e);
            }
        }
        self.view.redraw(cx);
    }

    /// Select output device
    fn select_output_device(&mut self, device_name: &str) {
        if let Some(ref mut audio_manager) = self.audio_manager {
            audio_manager.set_output_device(device_name);
        }
    }

    fn toggle_log_panel(&mut self, cx: &mut Cx) {
        self.log_panel_collapsed = !self.log_panel_collapsed;

        if self.log_panel_width == 0.0 {
            self.log_panel_width = 320.0;
        }

        if self.log_panel_collapsed {
            // Collapse: hide log content, show only toggle button
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: Fit });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, false);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, "<");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 0 });
        } else {
            // Expand: show log content at saved width
            let width = self.log_panel_width;
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: (width) });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, true);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, ">");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 16 });
        }

        self.view.redraw(cx);
    }

    fn resize_log_panel(&mut self, cx: &mut Cx, abs_x: f64) {
        let container_rect = self.view.area().rect(cx);
        let padding = 16.0; // Match screen padding
        let new_log_width = (container_rect.pos.x + container_rect.size.x - abs_x - padding)
            .max(150.0)  // Minimum log panel width
            .min(container_rect.size.x - 400.0);  // Leave space for main content

        self.log_panel_width = new_log_width;

        self.view.view(ids!(log_section)).apply_over(cx, live!{
            width: (new_log_width)
        });

        self.view.redraw(cx);
    }

    /// Update log display based on current filter and search
    fn update_log_display(&mut self, cx: &mut Cx) {
        let search_text = self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Filter log entries
        let filtered_logs: Vec<&String> = self.log_entries.iter().filter(|entry| {
            // Level filter: 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
            let level_match = match level_filter {
                0 => true, // ALL
                1 => entry.contains("[DEBUG]"),
                2 => entry.contains("[INFO]"),
                3 => entry.contains("[WARN]"),
                4 => entry.contains("[ERROR]"),
                _ => true,
            };

            // Node filter: 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
            let node_match = match node_filter {
                0 => true, // All Nodes
                1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                _ => true,
            };

            // Search filter
            let search_match = search_text.is_empty() || entry.to_lowercase().contains(&search_text);

            level_match && node_match && search_match
        }).collect();

        // Build display text
        let log_text = if filtered_logs.is_empty() {
            "*No log entries*".to_string()
        } else {
            filtered_logs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        };

        // Update markdown display
        self.view.markdown(ids!(log_section.log_content_column.log_scroll.log_content_wrapper.log_content)).set_text(cx, &log_text);
        self.view.redraw(cx);
    }

    /// Copy filtered logs to clipboard
    fn copy_logs_to_clipboard(&mut self, cx: &mut Cx) {
        let search_text = self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Filter log entries (same as update_log_display)
        let filtered_logs: Vec<&String> = self.log_entries.iter().filter(|entry| {
            let level_match = match level_filter {
                0 => true,
                1 => entry.contains("[DEBUG]"),
                2 => entry.contains("[INFO]"),
                3 => entry.contains("[WARN]"),
                4 => entry.contains("[ERROR]"),
                _ => true,
            };
            let node_match = match node_filter {
                0 => true,
                1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                _ => true,
            };
            let search_match = search_text.is_empty() || entry.to_lowercase().contains(&search_text);
            level_match && node_match && search_match
        }).collect();

        let log_text = if filtered_logs.is_empty() {
            "No log entries".to_string()
        } else {
            filtered_logs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        };

        cx.copy_to_clipboard(&log_text);
    }

    /// Add a log entry
    pub fn add_log(&mut self, cx: &mut Cx, entry: &str) {
        self.log_entries.push(entry.to_string());
        self.update_log_display(cx);
    }

    /// Clear all logs
    pub fn clear_logs(&mut self, cx: &mut Cx) {
        self.log_entries.clear();
        self.update_log_display(cx);
    }
}

impl MoFaFMScreenRef {
    /// Stop audio timer - call this before hiding/removing the widget
    /// to prevent timer callbacks on inactive state
    /// Note: AEC blink animation is shader-driven and doesn't need stopping
    pub fn stop_timers(&self, cx: &mut Cx) {
        if let Some(inner) = self.borrow_mut() {
            cx.stop_timer(inner.audio_timer);
            ::log::debug!("MoFaFMScreen audio timer stopped");
        }
    }

    /// Restart audio timer - call this when the widget becomes visible again
    /// Note: AEC blink animation is shader-driven and auto-resumes
    pub fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.audio_timer = cx.start_interval(0.05);  // 50ms for mic level
            ::log::debug!("MoFaFMScreen audio timer started");
        }
    }

    /// Update dark mode for this screen
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Apply dark mode to screen background
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to chat section
            inner.view.view(ids!(left_column.chat_container.chat_section)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to chat header and title
            inner.view.view(ids!(left_column.chat_container.chat_section.chat_header)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.chat_container.chat_section.chat_header.chat_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to audio panel
            inner.view.view(ids!(left_column.audio_container.audio_panel)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to device labels
            inner.view.label(ids!(left_column.audio_container.audio_panel.device_selectors.input_device_group.input_device_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.audio_container.audio_panel.device_selectors.output_device_group.output_device_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // NOTE: DropDown apply_over causes "target class not found" errors
            // TODO: Find alternative way to theme dropdowns

            // Apply dark mode to MofaHero
            inner.view.mofa_hero(ids!(left_column.mofa_hero)).update_dark_mode(cx, dark_mode);

            // Apply dark mode to participant panels
            inner.view.participant_panel(ids!(left_column.participant_container.participant_bar.student1_panel)).update_dark_mode(cx, dark_mode);
            inner.view.participant_panel(ids!(left_column.participant_container.participant_bar.student2_panel)).update_dark_mode(cx, dark_mode);
            inner.view.participant_panel(ids!(left_column.participant_container.participant_bar.tutor_panel)).update_dark_mode(cx, dark_mode);

            // Apply dark mode to prompt section
            inner.view.view(ids!(left_column.prompt_container.prompt_section)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // NOTE: TextInput apply_over causes "target class not found" errors
            inner.view.button(ids!(left_column.prompt_container.prompt_section.prompt_row.button_group.reset_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to splitter
            inner.view.view(ids!(splitter)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log section - toggle column
            inner.view.view(ids!(log_section.toggle_column)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.button(ids!(log_section.toggle_column.toggle_log_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log section - log content column
            inner.view.view(ids!(log_section.log_content_column)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(log_section.log_content_column.log_header)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(log_section.log_content_column.log_header.log_title_row.log_title_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log content Markdown
            // Using widget() to get raw WidgetRef and apply_over
            inner.view.widget(ids!(log_section.log_content_column.log_scroll.log_content_wrapper.log_content)).apply_over(cx, live!{
                draw_normal: { dark_mode: (dark_mode) }
                draw_bold: { dark_mode: (dark_mode) }
                draw_fixed: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }
}

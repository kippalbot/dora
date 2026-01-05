//! MoFA Studio App - Main application shell
//!
//! This file contains the main App struct and all UI definitions.
//! Organized into sections:
//! - UI Definitions (live_design! macro)
//! - Widget Structs (Dashboard, App)
//! - Event Handling (AppMain impl)
//! - Helper Methods (organized by responsibility)

use makepad_widgets::*;
use mofa_studio_shell::widgets::sidebar::SidebarWidgetRefExt;

// App plugin system imports
use mofa_widgets::{MofaApp, AppRegistry};
use mofa_fm::{MoFaFMApp, MoFaFMScreenWidgetRefExt};
use mofa_settings::MoFaSettingsApp;
use mofa_settings::data::Preferences;
use mofa_settings::screen::SettingsScreenWidgetRefExt;

// ============================================================================
// TAB IDENTIFIER
// ============================================================================

/// Type-safe tab identifiers (replaces magic strings)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabId {
    Profile,
    Settings,
}

// ============================================================================
// UI DEFINITIONS
// ============================================================================

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import fonts and colors from shared theme (single source of truth)
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_MEDIUM;
    use mofa_widgets::theme::FONT_SEMIBOLD;
    use mofa_widgets::theme::FONT_BOLD;
    // Semantic colors
    use mofa_widgets::theme::DARK_BG;
    use mofa_widgets::theme::PANEL_BG;
    use mofa_widgets::theme::ACCENT_BLUE;
    use mofa_widgets::theme::ACCENT_GREEN;
    use mofa_widgets::theme::ACCENT_INDIGO;
    use mofa_widgets::theme::TEXT_PRIMARY;
    use mofa_widgets::theme::TEXT_SECONDARY;
    use mofa_widgets::theme::TEXT_MUTED;
    use mofa_widgets::theme::DIVIDER;
    use mofa_widgets::theme::BORDER;
    use mofa_widgets::theme::HOVER_BG;
    use mofa_widgets::theme::WHITE;
    use mofa_widgets::theme::TRANSPARENT;
    // Palette colors
    use mofa_widgets::theme::SLATE_50;
    use mofa_widgets::theme::SLATE_200;
    use mofa_widgets::theme::SLATE_400;
    use mofa_widgets::theme::SLATE_500;
    use mofa_widgets::theme::SLATE_600;
    use mofa_widgets::theme::SLATE_700;
    use mofa_widgets::theme::SLATE_800;
    use mofa_widgets::theme::GRAY_300;
    use mofa_widgets::theme::GRAY_600;
    use mofa_widgets::theme::GRAY_700;
    use mofa_widgets::theme::INDIGO_100;

    use mofa_studio_shell::widgets::sidebar::Sidebar;
    use mofa_fm::screen::MoFaFMScreen;
    use mofa_settings::screen::SettingsScreen;

    // Logo image
    MOFA_LOGO = dep("crate://self/resources/mofa-logo.png")

    // ------------------------------------------------------------------------
    // Tab Widgets
    // ------------------------------------------------------------------------

    // Tab widget - individual tab in tab bar
    TabWidget = <View> {
        width: Fit, height: 36
        flow: Right
        align: {y: 0.5}
        padding: {left: 12, right: 4, top: 0, bottom: 0}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance active: 0.0
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y + 4.0, 6.0);
                let light_inactive = (SLATE_200);
                let light_active = (WHITE);
                let dark_inactive = (SLATE_700);
                let dark_active = (SLATE_800);
                let inactive = mix(light_inactive, dark_inactive, self.dark_mode);
                let active_color = mix(light_active, dark_active, self.dark_mode);
                let color = mix(inactive, active_color, self.active);
                sdf.fill(color);
                return sdf.result;
            }
        }

        tab_label = <Label> {
            text: "Tab"
            margin: {right: 8}
            draw_text: {
                instance active: 0.0
                instance dark_mode: 0.0
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                fn get_color(self) -> vec4 {
                    let light_inactive = (SLATE_500);
                    let light_active = (SLATE_800);
                    let dark_inactive = (SLATE_400);
                    let dark_active = (SLATE_200);
                    let inactive = mix(light_inactive, dark_inactive, self.dark_mode);
                    let active_color = mix(light_active, dark_active, self.dark_mode);
                    return mix(inactive, active_color, self.active);
                }
            }
        }

        close_btn = <View> {
            width: 18, height: 18
            cursor: Hand
            show_bg: true
            draw_bg: {
                instance hover: 0.0
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let c = self.rect_size * 0.5;
                    sdf.circle(c.x, c.y, 8.0);
                    let hover_bg = mix((SLATE_200), (SLATE_600), self.dark_mode);
                    sdf.fill(mix((TRANSPARENT), hover_bg, self.hover));
                    let x_color = mix((SLATE_400), (SLATE_400), self.dark_mode);
                    sdf.move_to(c.x - 3.0, c.y - 3.0);
                    sdf.line_to(c.x + 3.0, c.y + 3.0);
                    sdf.stroke(x_color, 1.5);
                    sdf.move_to(c.x + 3.0, c.y - 3.0);
                    sdf.line_to(c.x - 3.0, c.y + 3.0);
                    sdf.stroke(x_color, 1.5);
                    return sdf.result;
                }
            }
        }
    }

    // Home tab widget - no close button
    HomeTabWidget = <View> {
        width: Fit, height: 36
        flow: Right
        align: {y: 0.5}
        padding: {left: 12, right: 12, top: 0, bottom: 0}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance active: 0.0
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y + 4.0, 6.0);
                let light_inactive = (SLATE_200);
                let light_active = (WHITE);
                let dark_inactive = (SLATE_700);
                let dark_active = (SLATE_800);
                let inactive = mix(light_inactive, dark_inactive, self.dark_mode);
                let active_color = mix(light_active, dark_active, self.dark_mode);
                let color = mix(inactive, active_color, self.active);
                sdf.fill(color);
                return sdf.result;
            }
        }

        <Icon> {
            margin: {right: 6}
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/app.svg")
                fn get_color(self) -> vec4 { return (ACCENT_BLUE); }
            }
            icon_walk: {width: 14, height: 14}
        }

        tab_label = <Label> {
            text: "MoFA FM"
            draw_text: {
                instance active: 0.0
                instance dark_mode: 0.0
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                fn get_color(self) -> vec4 {
                    let light_inactive = (SLATE_500);
                    let light_active = (SLATE_800);
                    let dark_inactive = (SLATE_400);
                    let dark_active = (SLATE_200);
                    let inactive = mix(light_inactive, dark_inactive, self.dark_mode);
                    let active_color = mix(light_active, dark_active, self.dark_mode);
                    return mix(inactive, active_color, self.active);
                }
            }
        }
    }

    // Tab bar container
    TabBar = <View> {
        width: Fill, height: 36
        flow: Right
        spacing: 2
        padding: {left: 12, top: 0}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((SLATE_200), (SLATE_700), self.dark_mode);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Dashboard Layout
    // ------------------------------------------------------------------------

    // Dark theme colors (imported for shader use)
    use mofa_widgets::theme::DARK_BG_DARK;
    use mofa_widgets::theme::PANEL_BG_DARK;
    use mofa_widgets::theme::TEXT_PRIMARY_DARK;
    use mofa_widgets::theme::TEXT_SECONDARY_DARK;
    use mofa_widgets::theme::BORDER_DARK;
    use mofa_widgets::theme::HOVER_BG_DARK;
    use mofa_widgets::theme::DIVIDER_DARK;

    Dashboard = {{Dashboard}} <View> {
        width: Fill, height: Fill
        flow: Overlay
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Base layer - header + content area
        dashboard_base = <View> {
            width: Fill, height: Fill
            flow: Down

            // Header
            header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12
                align: {y: 0.5}
                padding: {left: 20, right: 20, top: 15, bottom: 15}
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                    }
                }

                hamburger_placeholder = <View> {
                    width: 21, height: 21
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cy = self.rect_size.y * 0.5;
                            let cx = self.rect_size.x * 0.5;
                            sdf.move_to(cx - 5.0, cy - 4.0);
                            sdf.line_to(cx + 5.0, cy - 4.0);
                            sdf.stroke((SLATE_500), 1.5);
                            sdf.move_to(cx - 5.0, cy);
                            sdf.line_to(cx + 5.0, cy);
                            sdf.stroke((SLATE_500), 1.5);
                            sdf.move_to(cx - 5.0, cy + 4.0);
                            sdf.line_to(cx + 5.0, cy + 4.0);
                            sdf.stroke((SLATE_500), 1.5);
                            return sdf.result;
                        }
                    }
                }

                logo = <Image> {
                    width: 40, height: 40
                    source: (MOFA_LOGO)
                }

                title = <Label> {
                    text: "MoFA Studio"
                    draw_text: {
                        color: (TEXT_PRIMARY)
                        text_style: <FONT_BOLD>{ font_size: 24.0 }
                    }
                }

                <View> { width: Fill, height: 1 }

                // Theme toggle button
                theme_toggle = <View> {
                    width: 36, height: 36
                    align: {x: 0.5, y: 0.5}
                    cursor: Hand
                    show_bg: true
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cx = self.rect_size.x * 0.5;
                            let cy = self.rect_size.y * 0.5;
                            sdf.circle(cx, cy, 16.0);
                            sdf.fill(mix((TRANSPARENT), (HOVER_BG), self.hover));
                            return sdf.result;
                        }
                    }

                    // Sun icon (light mode) - amber color
                    sun_icon = <View> {
                        width: 20, height: 20
                        show_bg: true
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let c = self.rect_size * 0.5;
                                let amber = vec4(0.961, 0.624, 0.043, 1.0);  // AMBER_500 #f59f0b
                                // Sun circle
                                sdf.circle(c.x, c.y, 4.0);
                                sdf.fill(amber);
                                // Sun rays
                                let ray_len = 2.5;
                                let ray_dist = 6.5;
                                sdf.move_to(c.x, c.y - ray_dist);
                                sdf.line_to(c.x, c.y - ray_dist - ray_len);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x, c.y + ray_dist);
                                sdf.line_to(c.x, c.y + ray_dist + ray_len);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x - ray_dist, c.y);
                                sdf.line_to(c.x - ray_dist - ray_len, c.y);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x + ray_dist, c.y);
                                sdf.line_to(c.x + ray_dist + ray_len, c.y);
                                sdf.stroke(amber, 1.5);
                                return sdf.result;
                            }
                        }
                    }

                    // Moon icon (dark mode - hidden by default) - indigo color
                    moon_icon = <View> {
                        width: 20, height: 20
                        visible: false
                        show_bg: true
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let c = self.rect_size * 0.5;
                                let indigo = vec4(0.388, 0.400, 0.945, 1.0);  // INDIGO_500 #6366f1
                                let white = vec4(1.0, 1.0, 1.0, 1.0);
                                sdf.circle(c.x, c.y, 6.0);
                                sdf.fill(indigo);
                                sdf.circle(c.x + 3.5, c.y - 2.5, 4.5);
                                sdf.fill(white);
                                return sdf.result;
                            }
                        }
                    }
                }

                user_profile_container = <View> {
                    width: Fit, height: Fill
                    flow: Right
                    align: {x: 0.5, y: 0.5}
                    spacing: 4
                    cursor: Hand

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
                                sdf.fill((HOVER_BG));
                                return sdf.result;
                            }
                        }

                        <Icon> {
                            draw_icon: {
                                svg_file: dep("crate://self/resources/icons/user.svg")
                                fn get_color(self) -> vec4 { return (GRAY_600); }
                            }
                            icon_walk: {width: 16, height: 16}
                        }
                    }

                    dropdown_arrow = <View> {
                        width: 12, height: Fill
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let cx = self.rect_size.x * 0.5;
                                let cy = self.rect_size.y * 0.5;
                                sdf.move_to(cx - 4.0, cy - 2.0);
                                sdf.line_to(cx, cy + 2.0);
                                sdf.line_to(cx + 4.0, cy - 2.0);
                                sdf.stroke((TEXT_MUTED), 1.5);
                                return sdf.result;
                            }
                        }
                    }
                }
            }

            // Content area
            content_area = <View> {
                width: Fill, height: Fill
                flow: Right
                padding: 20

                main_content = <View> {
                    width: Fill, height: Fill
                    flow: Down

                    content = <View> {
                        width: Fill, height: Fill
                        flow: Overlay

                        fm_page = <MoFaFMScreen> {
                            width: Fill, height: Fill
                            visible: true
                        }

                        app_page = <View> {
                            width: Fill, height: Fill
                            flow: Down
                            spacing: 12
                            visible: false
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: { color: (DARK_BG) }

                            <Label> {
                                text: "Demo App"
                                draw_text: {
                                    color: (TEXT_MUTED)
                                    text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                                }
                            }
                            <Label> {
                                text: "Select an app from the sidebar"
                                draw_text: {
                                    color: (GRAY_300)
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                }
                            }
                        }

                        settings_page = <SettingsScreen> {
                            width: Fill, height: Fill
                            visible: false
                        }
                    }
                }
            }
        }

        // Tab overlay - modal layer for Profile/Settings
        tab_overlay = <View> {
            width: Fill, height: Fill
            flow: Down
            visible: false
            margin: {top: 70}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
                }
            }

            tab_bar = <TabBar> {
                profile_tab = <TabWidget> {
                    visible: false
                    tab_label = { text: "Profile" }
                }
                settings_tab = <TabWidget> {
                    visible: false
                    tab_label = { text: "Settings" }
                }
            }

            tab_content = <View> {
                width: Fill, height: Fill
                flow: Overlay
                padding: 20

                profile_page = <RoundedView> {
                    padding: 20
                    width: Fill, height: Fill
                    visible: false
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 8.0
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    padding: 24
                    flow: Down
                    spacing: 16

                    profile_title = <Label> {
                        text: "User Profile"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_BOLD>{ font_size: 20.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                            }
                        }
                    }

                    profile_divider = <View> {
                        width: Fill, height: 1
                        show_bg: true
                        draw_bg: {
                            instance dark_mode: 0.0
                            fn pixel(self) -> vec4 {
                                return mix((SLATE_200), (SLATE_700), self.dark_mode);
                            }
                        }
                    }

                    profile_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 16
                        align: {y: 0.5}

                        profile_avatar = <View> {
                            width: 64, height: 64
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;
                                    sdf.circle(c.x, c.y, 30.0);
                                    let bg = mix((INDIGO_100), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            align: {x: 0.5, y: 0.5}
                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/user.svg")
                                    fn get_color(self) -> vec4 { return (ACCENT_INDIGO); }
                                }
                                icon_walk: {width: 32, height: 32}
                            }
                        }

                        profile_info = <View> {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: 4

                            profile_name = <Label> {
                                text: "Demo User"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_SEMIBOLD>{ font_size: 16.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }
                            }
                            profile_email = <Label> {
                                text: "demo@mofa.studio"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }
                        }
                    }

                    profile_coming_soon = <Label> {
                        text: "Profile settings coming soon..."
                        margin: {top: 20}
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_REGULAR>{ font_size: 13.0 }
                            fn get_color(self) -> vec4 {
                                return mix((SLATE_400), (SLATE_500), self.dark_mode);
                            }
                        }
                    }
                }

                settings_tab_page = <SettingsScreen> {
                    width: Fill, height: Fill
                    visible: false
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // App Window
    // ------------------------------------------------------------------------

    App = {{App}} {
        ui: <Window> {
            window: { title: "MoFA Studio", inner_size: vec2(1400, 900) }
            pass: { clear_color: (DARK_BG) }
            flow: Overlay

            body = <Dashboard> {}

            sidebar_trigger_overlay = <View> {
                width: 28, height: 28
                abs_pos: vec2(18.0, 16.0)
                cursor: Hand
            }

            sidebar_menu_overlay = <View> {
                width: 250, height: Fit
                abs_pos: vec2(0.0, 52.0)
                visible: false
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                        sdf.fill(bg);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        let border = mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                        sdf.stroke(border, 1.0);
                        return sdf.result;
                    }
                }

                sidebar_content = <Sidebar> {}
            }

            user_btn_overlay = <View> {
                width: 60, height: 44
                abs_pos: vec2(1320.0, 10.0)
                cursor: Hand
            }

            user_menu = <View> {
                width: 140, height: Fit
                abs_pos: vec2(1250.0, 55.0)
                visible: false
                padding: 6
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                        sdf.fill(bg);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                        let border = mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                        sdf.stroke(border, 1.0);
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
                        fn get_color(self) -> vec4 { return (SLATE_500); }
                    }
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((GRAY_700), (SLATE_200), self.dark_mode);
                        }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let light_normal = (SLATE_50);
                            let light_hover = (SLATE_200);
                            let dark_normal = (SLATE_800);
                            let dark_hover = (SLATE_700);
                            let normal = mix(light_normal, dark_normal, self.dark_mode);
                            let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                            let color = mix(normal, hover_color, self.hover);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                }

                menu_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            return mix((BORDER), (BORDER_DARK), self.dark_mode);
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
                        fn get_color(self) -> vec4 { return (SLATE_500); }
                    }
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((GRAY_700), (SLATE_200), self.dark_mode);
                        }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let light_normal = (SLATE_50);
                            let light_hover = (SLATE_200);
                            let dark_normal = (SLATE_800);
                            let dark_hover = (SLATE_700);
                            let normal = mix(light_normal, dark_normal, self.dark_mode);
                            let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                            let color = mix(normal, hover_color, self.hover);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// WIDGET STRUCTS
// ============================================================================

#[derive(Live, LiveHook, Widget)]
pub struct Dashboard {
    #[deref]
    view: View,
}

impl Widget for Dashboard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    user_menu_open: bool,
    #[rust]
    sidebar_menu_open: bool,
    #[rust]
    open_tabs: Vec<TabId>,
    #[rust]
    active_tab: Option<TabId>,
    #[rust]
    last_window_size: DVec2,
    #[rust]
    sidebar_animating: bool,
    #[rust]
    sidebar_animation_start: f64,
    #[rust]
    sidebar_slide_in: bool,
    /// Registry of installed apps (populated on init)
    #[rust]
    app_registry: AppRegistry,
    /// Dark mode state
    #[rust]
    dark_mode: bool,
    /// Dark mode animation progress (0.0 = light, 1.0 = dark)
    #[rust]
    dark_mode_anim: f64,
    /// Whether dark mode animation is in progress
    #[rust]
    dark_mode_animating: bool,
    /// Animation start time
    #[rust]
    dark_mode_anim_start: f64,
    /// Whether initial theme has been applied (on first draw)
    #[rust]
    theme_initialized: bool,
}

impl LiveHook for App {
    fn after_new_from_doc(&mut self, _cx: &mut Cx) {
        // Initialize the app registry with all installed apps
        self.app_registry.register(MoFaFMApp::info());
        self.app_registry.register(MoFaSettingsApp::info());

        // Load user preferences and restore dark mode
        let prefs = Preferences::load();
        self.dark_mode = prefs.dark_mode;
        self.dark_mode_anim = if prefs.dark_mode { 1.0 } else { 0.0 };
    }
}

// ============================================================================
// APP REGISTRY METHODS
// ============================================================================

impl App {
    /// Get the number of installed apps
    #[allow(dead_code)]
    pub fn app_count(&self) -> usize {
        self.app_registry.len()
    }

    /// Get app info by ID
    #[allow(dead_code)]
    pub fn get_app_info(&self, id: &str) -> Option<&mofa_widgets::AppInfo> {
        self.app_registry.find_by_id(id)
    }

    /// Get all registered apps
    #[allow(dead_code)]
    pub fn apps(&self) -> &[mofa_widgets::AppInfo] {
        self.app_registry.apps()
    }
}

// ============================================================================
// WIDGET REGISTRATION
// ============================================================================

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        // Core widget libraries
        makepad_widgets::live_design(cx);
        mofa_widgets::live_design(cx);
        mofa_studio_shell::widgets::sidebar::live_design(cx);

        // Register apps via MofaApp trait
        // Note: Widget types in live_design! macro still require compile-time imports
        // (Makepad constraint), but registration uses the standardized trait interface
        <MoFaFMApp as MofaApp>::live_design(cx);
        <MoFaSettingsApp as MofaApp>::live_design(cx);
    }
}

// ============================================================================
// EVENT HANDLING
// ============================================================================

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());

        // Initialize theme on first draw (widgets are ready)
        if !self.theme_initialized {
            if let Event::Draw(_) = event {
                self.theme_initialized = true;
                // Apply initial dark mode from preferences (full update)
                self.apply_dark_mode_panels(cx);
                self.apply_dark_mode_screens(cx);
                // Update header theme toggle icon
                self.update_theme_toggle_icon(cx);
            }
        }

        // Window resize handling
        self.handle_window_resize(cx, event);

        // Sidebar animation
        if self.sidebar_animating {
            self.update_sidebar_animation(cx);
        }

        // Dark mode animation
        if self.dark_mode_animating {
            self.update_dark_mode_animation(cx);
        }

        // Extract actions
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

        // Handle hover events
        self.handle_user_menu_hover(cx, event);
        self.handle_sidebar_hover(cx, event);
        self.handle_theme_toggle(cx, event);

        // Handle click events
        self.handle_sidebar_clicks(cx, &actions);
        self.handle_user_menu_clicks(cx, &actions);
        self.handle_mofa_hero_buttons(cx, event);
        self.handle_tab_clicks(cx, &actions);
        self.handle_tab_close_clicks(cx, event);
    }
}

// ============================================================================
// WINDOW & LAYOUT METHODS
// ============================================================================

impl App {
    /// Handle window resize events
    fn handle_window_resize(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::WindowGeomChange(wg) = event {
            let new_size = wg.new_geom.inner_size;
            if new_size != self.last_window_size {
                self.last_window_size = new_size;
                self.update_overlay_positions(cx);
            }
        }

        if let Event::Draw(_) = event {
            let window_rect = self.ui.area().rect(cx);
            if window_rect.size.x > 0.0 && window_rect.size != self.last_window_size {
                self.last_window_size = window_rect.size;
                self.update_overlay_positions(cx);
            }
        }
    }

    /// Update overlay positions based on window size
    fn update_overlay_positions(&mut self, cx: &mut Cx) {
        let window_width = self.last_window_size.x;
        let window_height = self.last_window_size.y;

        if window_width <= 0.0 {
            return;
        }

        let user_btn_x = window_width - 80.0;
        self.ui.view(ids!(user_btn_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(user_btn_x, 10.0))
        });

        let user_menu_x = window_width - 150.0;
        self.ui.view(ids!(user_menu)).apply_over(cx, live!{
            abs_pos: (dvec2(user_menu_x, 55.0))
        });

        let max_scroll_height = (window_height - 230.0).max(200.0);
        self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content)).set_max_scroll_height(max_scroll_height);

        self.ui.redraw(cx);
    }
}

// ============================================================================
// USER MENU METHODS
// ============================================================================

impl App {
    /// Handle user menu hover
    fn handle_user_menu_hover(&mut self, cx: &mut Cx, event: &Event) {
        let user_btn = self.ui.view(ids!(user_btn_overlay));
        let user_menu = self.ui.view(ids!(user_menu));

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

        if self.user_menu_open {
            if let Event::MouseMove(mm) = event {
                let btn_rect = user_btn.area().rect(cx);
                let menu_rect = user_menu.area().rect(cx);

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
    }

    /// Handle user menu button clicks
    fn handle_user_menu_clicks(&mut self, cx: &mut Cx, actions: &[Action]) {
        if self.ui.button(ids!(user_menu.menu_profile_btn)).clicked(actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.open_or_switch_tab(cx, TabId::Profile);
        }

        if self.ui.button(ids!(user_menu.menu_settings_btn)).clicked(actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.open_or_switch_tab(cx, TabId::Settings);
        }
    }

    /// Handle header theme toggle button
    fn handle_theme_toggle(&mut self, cx: &mut Cx, event: &Event) {
        let theme_btn = self.ui.view(ids!(body.dashboard_base.header.theme_toggle));

        match event.hits(cx, theme_btn.area()) {
            Hit::FingerHoverIn(_) => {
                self.ui.view(ids!(body.dashboard_base.header.theme_toggle)).apply_over(cx, live!{
                    draw_bg: { hover: 1.0 }
                });
                self.ui.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.ui.view(ids!(body.dashboard_base.header.theme_toggle)).apply_over(cx, live!{
                    draw_bg: { hover: 0.0 }
                });
                self.ui.redraw(cx);
            }
            Hit::FingerUp(_) => {
                self.toggle_dark_mode(cx);
                self.update_theme_toggle_icon(cx);

                // Save preference to disk
                let mut prefs = Preferences::load();
                prefs.dark_mode = self.dark_mode;
                if let Err(e) = prefs.save() {
                    eprintln!("Failed to save dark mode preference: {}", e);
                }
            }
            _ => {}
        }
    }

    /// Update the theme toggle icon based on current mode
    fn update_theme_toggle_icon(&mut self, cx: &mut Cx) {
        let is_dark = self.dark_mode;
        self.ui.view(ids!(body.dashboard_base.header.theme_toggle.sun_icon)).set_visible(cx, !is_dark);
        self.ui.view(ids!(body.dashboard_base.header.theme_toggle.moon_icon)).set_visible(cx, is_dark);
        self.ui.redraw(cx);
    }
}

// ============================================================================
// SIDEBAR METHODS
// ============================================================================

impl App {
    /// Handle sidebar hover
    fn handle_sidebar_hover(&mut self, cx: &mut Cx, event: &Event) {
        let sidebar_trigger = self.ui.view(ids!(sidebar_trigger_overlay));
        let sidebar_menu = self.ui.view(ids!(sidebar_menu_overlay));

        match event.hits(cx, sidebar_trigger.area()) {
            Hit::FingerHoverIn(_) => {
                if !self.sidebar_menu_open && !self.sidebar_animating {
                    self.sidebar_menu_open = true;
                    self.start_sidebar_slide_in(cx);
                }
            }
            _ => {}
        }

        if self.sidebar_menu_open && !self.sidebar_animating {
            if let Event::MouseMove(mm) = event {
                let trigger_rect = sidebar_trigger.area().rect(cx);
                let sidebar_rect = sidebar_menu.area().rect(cx);

                let in_trigger = mm.abs.x >= trigger_rect.pos.x - 5.0
                    && mm.abs.x <= trigger_rect.pos.x + trigger_rect.size.x + 5.0
                    && mm.abs.y >= trigger_rect.pos.y - 5.0
                    && mm.abs.y <= trigger_rect.pos.y + trigger_rect.size.y + 5.0;

                let in_sidebar = mm.abs.x >= sidebar_rect.pos.x - 5.0
                    && mm.abs.x <= sidebar_rect.pos.x + sidebar_rect.size.x + 10.0
                    && mm.abs.y >= sidebar_rect.pos.y - 5.0
                    && mm.abs.y <= sidebar_rect.pos.y + sidebar_rect.size.y + 5.0;

                if !in_trigger && !in_sidebar {
                    self.sidebar_menu_open = false;
                    self.start_sidebar_slide_out(cx);
                }
            }
        }
    }

    /// Handle sidebar menu item clicks
    fn handle_sidebar_clicks(&mut self, cx: &mut Cx, actions: &[Action]) {
        // MoFA FM tab
        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.mofa_fm_tab)).clicked(actions) {
            self.sidebar_menu_open = false;
            self.start_sidebar_slide_out(cx);
            self.open_tabs.clear();
            self.active_tab = None;
            self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: true });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: false });
            self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).start_timers(cx);
            self.ui.redraw(cx);
        }

        // Settings tab
        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.settings_tab)).clicked(actions) {
            self.sidebar_menu_open = false;
            self.start_sidebar_slide_out(cx);
            self.open_tabs.clear();
            self.active_tab = None;
            self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
            self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).stop_timers(cx);
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: true });
            self.ui.redraw(cx);
        }

        // App buttons (1-20) - check if any was clicked
        let app_clicked =
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app1_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app2_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app3_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app4_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app5_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app6_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app7_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app8_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app9_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app10_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app11_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app12_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app13_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app14_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app15_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app16_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app17_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app18_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app19_btn)).clicked(actions) ||
            self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app20_btn)).clicked(actions);

        if app_clicked {
            self.sidebar_menu_open = false;
            self.start_sidebar_slide_out(cx);
            self.open_tabs.clear();
            self.active_tab = None;
            self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
            self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).stop_timers(cx);
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: true });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: false });
            self.ui.redraw(cx);
        }
    }
}

// ============================================================================
// ANIMATION METHODS
// ============================================================================

impl App {
    /// Update sidebar slide animation
    fn update_sidebar_animation(&mut self, cx: &mut Cx) {
        const ANIMATION_DURATION: f64 = 0.2;
        const SIDEBAR_WIDTH: f64 = 250.0;

        let elapsed = Cx::time_now() - self.sidebar_animation_start;
        let progress = (elapsed / ANIMATION_DURATION).min(1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);

        let x = if self.sidebar_slide_in {
            -SIDEBAR_WIDTH * (1.0 - eased)
        } else {
            -SIDEBAR_WIDTH * eased
        };

        self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(x, 52.0))
        });

        if progress >= 1.0 {
            self.sidebar_animating = false;
            if !self.sidebar_slide_in {
                self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, false);
                self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content)).collapse_show_more(cx);
            }
        }

        self.ui.redraw(cx);
    }

    /// Start sidebar slide-in animation
    fn start_sidebar_slide_in(&mut self, cx: &mut Cx) {
        self.sidebar_animating = true;
        self.sidebar_animation_start = Cx::time_now();
        self.sidebar_slide_in = true;
        self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(-250.0, 52.0))
        });
        self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, true);
        self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content)).restore_selection_state(cx);
        self.ui.redraw(cx);
    }

    /// Start sidebar slide-out animation
    fn start_sidebar_slide_out(&mut self, cx: &mut Cx) {
        self.sidebar_animating = true;
        self.sidebar_animation_start = Cx::time_now();
        self.sidebar_slide_in = false;
        self.ui.redraw(cx);
    }

    /// Toggle dark mode with animation
    pub fn toggle_dark_mode(&mut self, cx: &mut Cx) {
        self.dark_mode = !self.dark_mode;
        self.dark_mode_animating = true;
        self.dark_mode_anim_start = Cx::time_now();

        // Apply screens immediately at target value (snap, not animated)
        // This avoids calling update_dark_mode on every frame
        let target = if self.dark_mode { 1.0 } else { 0.0 };
        self.apply_dark_mode_screens_with_value(cx, target);

        self.ui.redraw(cx);
    }

    /// Update dark mode animation
    fn update_dark_mode_animation(&mut self, cx: &mut Cx) {
        let elapsed = Cx::time_now() - self.dark_mode_anim_start;
        let duration = 0.3; // 300ms animation

        // Ease-out cubic
        let t = (elapsed / duration).min(1.0);
        let eased = 1.0 - (1.0 - t).powi(3);

        // Animate from current to target
        let target = if self.dark_mode { 1.0 } else { 0.0 };
        let start = if self.dark_mode { 0.0 } else { 1.0 };
        self.dark_mode_anim = start + (target - start) * eased;

        // During animation: only update main panels (no errors)
        // Full update with screens happens only at the end
        self.apply_dark_mode_panels(cx);

        if t >= 1.0 {
            self.dark_mode_animating = false;
            self.dark_mode_anim = target;
            // Apply to ALL widgets including screens at animation end
            self.apply_dark_mode_screens(cx);
        }

        self.ui.redraw(cx);
    }

    /// Apply dark mode to main panels only (safe for animation frames, no errors)
    fn apply_dark_mode_panels(&mut self, cx: &mut Cx) {
        let dm = self.dark_mode_anim;

        // Apply to main app background (Dashboard)
        self.ui.view(ids!(body)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        // Apply to header
        self.ui.view(ids!(body.dashboard_base.header)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        // Apply to sidebar menu overlay
        self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        // Apply to sidebar content (this is safe, sidebar widget handles it internally)
        self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content))
            .update_dark_mode(cx, dm);

        // Apply to user menu
        self.ui.view(ids!(user_menu)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        // Apply to user menu buttons
        self.ui.button(ids!(user_menu.menu_profile_btn)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
            draw_text: { dark_mode: (dm) }
        });
        self.ui.button(ids!(user_menu.menu_settings_btn)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
            draw_text: { dark_mode: (dm) }
        });
        self.ui.view(ids!(user_menu.menu_divider)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        // Apply to tab overlay - only when tabs are open
        if !self.open_tabs.is_empty() {
            self.ui.view(ids!(body.tab_overlay)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dm) }
            });

            // Apply to tab bar
            self.ui.view(ids!(body.tab_overlay.tab_bar)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dm) }
            });

            // Apply to tab widgets
            if self.open_tabs.contains(&TabId::Profile) {
                self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
                self.ui.label(ids!(body.tab_overlay.tab_bar.profile_tab.tab_label)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
                self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab.close_btn)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
            }

            if self.open_tabs.contains(&TabId::Settings) {
                self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
                self.ui.label(ids!(body.tab_overlay.tab_bar.settings_tab.tab_label)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
                self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab.close_btn)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
            }

            // Tab content backgrounds
            if self.open_tabs.contains(&TabId::Profile) {
                // Profile page background
                self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
                // Profile page internal widgets
                self.ui.label(ids!(body.tab_overlay.tab_content.profile_page.profile_title)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
                self.ui.view(ids!(body.tab_overlay.tab_content.profile_page.profile_divider)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
                self.ui.view(ids!(body.tab_overlay.tab_content.profile_page.profile_row.profile_avatar)).apply_over(cx, live!{
                    draw_bg: { dark_mode: (dm) }
                });
                self.ui.label(ids!(body.tab_overlay.tab_content.profile_page.profile_row.profile_info.profile_name)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
                self.ui.label(ids!(body.tab_overlay.tab_content.profile_page.profile_row.profile_info.profile_email)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
                self.ui.label(ids!(body.tab_overlay.tab_content.profile_page.profile_coming_soon)).apply_over(cx, live!{
                    draw_text: { dark_mode: (dm) }
                });
            }
        }
    }

    /// Apply dark mode to screens (may produce errors, called once at start/end only)
    fn apply_dark_mode_screens(&mut self, cx: &mut Cx) {
        self.apply_dark_mode_screens_with_value(cx, self.dark_mode_anim);
    }

    /// Apply dark mode to screens with a specific value
    fn apply_dark_mode_screens_with_value(&mut self, cx: &mut Cx, dm: f64) {
        // Apply to MoFA FM screen
        self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page))
            .update_dark_mode(cx, dm);

        // Apply to Settings screen in main content
        self.ui.settings_screen(ids!(body.dashboard_base.content_area.main_content.content.settings_page))
            .update_dark_mode(cx, dm);

        // Apply to tab overlay content - only when tabs are open
        if !self.open_tabs.is_empty() {
            if self.open_tabs.contains(&TabId::Settings) {
                self.ui.settings_screen(ids!(body.tab_overlay.tab_content.settings_tab_page))
                    .update_dark_mode(cx, dm);
            }
        }
    }
}

// ============================================================================
// TAB MANAGEMENT METHODS
// ============================================================================

impl App {
    /// Open a tab or switch to it if already open
    fn open_or_switch_tab(&mut self, cx: &mut Cx, tab_id: TabId) {
        if !self.open_tabs.contains(&tab_id) {
            self.open_tabs.push(tab_id);
        }

        self.active_tab = Some(tab_id);
        self.update_tab_ui(cx);
    }

    /// Close a tab
    fn close_tab(&mut self, cx: &mut Cx, tab_id: TabId) {
        self.open_tabs.retain(|t| *t != tab_id);

        if self.active_tab == Some(tab_id) {
            self.active_tab = self.open_tabs.last().copied();
        }

        self.update_tab_ui(cx);
    }

    /// Handle tab widget clicks
    fn handle_tab_clicks(&mut self, cx: &mut Cx, actions: &[Action]) {
        if self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab)).finger_up(actions).is_some() {
            if self.open_tabs.contains(&TabId::Profile) {
                self.active_tab = Some(TabId::Profile);
                self.update_tab_ui(cx);
            }
        }

        if self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab)).finger_up(actions).is_some() {
            if self.open_tabs.contains(&TabId::Settings) {
                self.active_tab = Some(TabId::Settings);
                self.update_tab_ui(cx);
            }
        }
    }

    /// Handle tab close button clicks
    fn handle_tab_close_clicks(&mut self, cx: &mut Cx, event: &Event) {
        let profile_close = self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab.close_btn));
        match event.hits(cx, profile_close.area()) {
            Hit::FingerUp(_) => {
                self.close_tab(cx, TabId::Profile);
                return;
            }
            Hit::FingerHoverIn(_) => {
                self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab.close_btn))
                    .apply_over(cx, live!{ draw_bg: { hover: 1.0 } });
                self.ui.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab.close_btn))
                    .apply_over(cx, live!{ draw_bg: { hover: 0.0 } });
                self.ui.redraw(cx);
            }
            _ => {}
        }

        let settings_close = self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab.close_btn));
        match event.hits(cx, settings_close.area()) {
            Hit::FingerUp(_) => {
                self.close_tab(cx, TabId::Settings);
                return;
            }
            Hit::FingerHoverIn(_) => {
                self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab.close_btn))
                    .apply_over(cx, live!{ draw_bg: { hover: 1.0 } });
                self.ui.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab.close_btn))
                    .apply_over(cx, live!{ draw_bg: { hover: 0.0 } });
                self.ui.redraw(cx);
            }
            _ => {}
        }
    }

    /// Update tab bar and content visibility
    fn update_tab_ui(&mut self, cx: &mut Cx) {
        let profile_open = self.open_tabs.contains(&TabId::Profile);
        let settings_open = self.open_tabs.contains(&TabId::Settings);
        let any_tabs_open = !self.open_tabs.is_empty();

        let profile_active = self.active_tab == Some(TabId::Profile);
        let settings_active = self.active_tab == Some(TabId::Settings);

        let was_overlay_visible = self.ui.view(ids!(body.tab_overlay)).visible();

        self.ui.view(ids!(body.tab_overlay)).set_visible(cx, any_tabs_open);

        // Manage FM page timers
        if any_tabs_open && !was_overlay_visible {
            self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).stop_timers(cx);
        } else if !any_tabs_open && was_overlay_visible {
            self.ui.mo_fa_fmscreen(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).start_timers(cx);
        }

        // Update tab visibility
        self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab)).set_visible(cx, profile_open);
        self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab)).set_visible(cx, settings_open);

        // Update profile tab active state
        let profile_active_val = if profile_active { 1.0 } else { 0.0 };
        self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab))
            .apply_over(cx, live!{ draw_bg: { active: (profile_active_val) } });
        self.ui.label(ids!(body.tab_overlay.tab_bar.profile_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { active: (profile_active_val) } });

        // Update settings tab active state
        let settings_active_val = if settings_active { 1.0 } else { 0.0 };
        self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab))
            .apply_over(cx, live!{ draw_bg: { active: (settings_active_val) } });
        self.ui.label(ids!(body.tab_overlay.tab_bar.settings_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { active: (settings_active_val) } });

        // Hide all content pages first
        self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, false);
        self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, false);

        // Show active tab content
        match self.active_tab {
            Some(TabId::Profile) => {
                self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, true);
            }
            Some(TabId::Settings) => {
                self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, true);
            }
            None => {
                if profile_open {
                    self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, true);
                } else if settings_open {
                    self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, true);
                }
            }
        }

        self.ui.redraw(cx);
    }
}

// ============================================================================
// MOFA HERO METHODS
// ============================================================================

impl App {
    /// Handle MofaHero start/stop button clicks
    fn handle_mofa_hero_buttons(&mut self, cx: &mut Cx, event: &Event) {
        let start_view = self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view));
        match event.hits(cx, start_view.area()) {
            Hit::FingerUp(_) => {
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view)).set_visible(cx, false);
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view)).set_visible(cx, true);
                self.ui.redraw(cx);
            }
            _ => {}
        }
        let stop_view = self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view));
        match event.hits(cx, stop_view.area()) {
            Hit::FingerUp(_) => {
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view)).set_visible(cx, true);
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view)).set_visible(cx, false);
                self.ui.redraw(cx);
            }
            _ => {}
        }
    }
}

// ============================================================================
// APP ENTRY POINT
// ============================================================================

app_main!(App);

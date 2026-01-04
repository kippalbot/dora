//! MoFA Studio App - Main application shell

use makepad_widgets::*;
use mofa_studio_shell::widgets::sidebar::SidebarWidgetRefExt;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_studio_shell::widgets::sidebar::Sidebar;
    use mofa_fm::screen::MoFaFMScreen;
    use mofa_settings::screen::SettingsScreen;

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
    MOFA_LOGO = dep("crate://self/resources/mofa-logo.png")

    // Light color palette
    DARK_BG = #f5f7fa
    PANEL_BG = #ffffff
    ACCENT_BLUE = #3b82f6
    ACCENT_GREEN = #10b981
    TEXT_PRIMARY = #1f2937
    TEXT_SECONDARY = #6b7280

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
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Tab shape with rounded top corners
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y + 4.0, 6.0);
                let color = mix(#e2e8f0, #ffffff, self.active);
                sdf.fill(color);
                return sdf.result;
            }
        }

        tab_label = <Label> {
            text: "Tab"
            margin: {right: 8}
            draw_text: {
                instance active: 0.0
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                fn get_color(self) -> vec4 {
                    return mix(#64748b, #1e293b, self.active);
                }
            }
        }

        close_btn = <View> {
            width: 18, height: 18
            cursor: Hand
            show_bg: true
            draw_bg: {
                instance hover: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let c = self.rect_size * 0.5;
                    // Circle background on hover
                    sdf.circle(c.x, c.y, 8.0);
                    sdf.fill(mix(#00000000, #e2e8f0, self.hover));
                    // X icon
                    sdf.move_to(c.x - 3.0, c.y - 3.0);
                    sdf.line_to(c.x + 3.0, c.y + 3.0);
                    sdf.stroke(#94a3b8, 1.5);
                    sdf.move_to(c.x + 3.0, c.y - 3.0);
                    sdf.line_to(c.x - 3.0, c.y + 3.0);
                    sdf.stroke(#94a3b8, 1.5);
                    return sdf.result;
                }
            }
        }
    }

    // Home tab widget - no close button (always visible, label is dynamic)
    HomeTabWidget = <View> {
        width: Fit, height: 36
        flow: Right
        align: {y: 0.5}
        padding: {left: 12, right: 12, top: 0, bottom: 0}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance active: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y + 4.0, 6.0);
                let color = mix(#e2e8f0, #ffffff, self.active);
                sdf.fill(color);
                return sdf.result;
            }
        }

        // Home icon
        <Icon> {
            margin: {right: 6}
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/app.svg")
                fn get_color(self) -> vec4 { return #3b82f6; }
            }
            icon_walk: {width: 14, height: 14}
        }

        tab_label = <Label> {
            text: "MoFA FM"
            draw_text: {
                instance active: 0.0
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                fn get_color(self) -> vec4 {
                    return mix(#64748b, #1e293b, self.active);
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
        draw_bg: { color: #e2e8f0 }
    }

    // Main Dashboard Layout
    Dashboard = {{Dashboard}} <View> {
        width: Fill, height: Fill
        flow: Overlay
        show_bg: true
        draw_bg: { color: (DARK_BG) }

        // Base layer - header + content area
        dashboard_base = <View> {
            width: Fill, height: Fill
            flow: Down

            // Header at top (full width)
            header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12
                align: {y: 0.5}
                padding: {left: 20, right: 20, top: 15, bottom: 15}
                show_bg: true
                draw_bg: { color: (PANEL_BG) }

                // Hamburger button placeholder (for layout spacing)
                hamburger_placeholder = <View> {
                    width: 21, height: 21
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cy = self.rect_size.y * 0.5;
                            let cx = self.rect_size.x * 0.5;
                            // Hamburger lines
                            sdf.move_to(cx - 5.0, cy - 4.0);
                            sdf.line_to(cx + 5.0, cy - 4.0);
                            sdf.stroke(#64748b, 1.5);
                            sdf.move_to(cx - 5.0, cy);
                            sdf.line_to(cx + 5.0, cy);
                            sdf.stroke(#64748b, 1.5);
                            sdf.move_to(cx - 5.0, cy + 4.0);
                            sdf.line_to(cx + 5.0, cy + 4.0);
                            sdf.stroke(#64748b, 1.5);
                            return sdf.result;
                        }
                    }
                }

                // Logo
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

            // Content area below header
            content_area = <View> {
                width: Fill, height: Fill
                flow: Right
                padding: 20

                // Main content
                main_content = <View> {
                    width: Fill, height: Fill
                    flow: Down

                    // Content area - pages stack on top of each other, only one visible at a time
                    content = <View> {
                        width: Fill, height: Fill
                        flow: Overlay

                        // FM Page (default visible - uses MoFaFMScreen from mofa-fm app)
                        fm_page = <MoFaFMScreen> {
                            width: Fill, height: Fill
                            visible: true
                        }

                        // App Page (hidden by default, shown when clicking demo app buttons)
                        app_page = <View> {
                            width: Fill, height: Fill
                            flow: Down
                            spacing: 12
                            visible: false
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: { color: #f5f7fa }

                            <Label> {
                                text: "Demo App"
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

                        // Settings page (hidden by default) - Use actual SettingsScreen widget
                        settings_page = <SettingsScreen> {
                            width: Fill, height: Fill
                            visible: false
                        }
                    }
                }
            }
        }

        // Tab overlay - modal layer for Profile/Settings only (no duplicate MoFaFMScreen)
        tab_overlay = <View> {
            width: Fill, height: Fill
            flow: Down
            visible: false
            margin: {top: 70}  // Position below header
            show_bg: true
            draw_bg: { color: (DARK_BG) }

            // Tab bar at top - only Profile and Settings tabs (no Home tab)
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

            // Tab content area - only Profile and Settings pages
            tab_content = <View> {
                width: Fill, height: Fill
                flow: Overlay
                padding: 20

                // Profile Page content
                profile_page = <RoundedView> {
                    padding: 20
                    width: Fill, height: Fill
                    visible: false
                    show_bg: true
                    draw_bg: { color: (PANEL_BG), border_radius: 8.0 }
                    padding: 24
                    flow: Down
                    spacing: 16

                    <Label> {
                        text: "User Profile"
                        draw_text: {
                            color: (TEXT_PRIMARY)
                            text_style: <FONT_BOLD>{ font_size: 20.0 }
                        }
                    }

                    <View> {
                        width: Fill, height: 1
                        show_bg: true
                        draw_bg: { color: #e2e8f0 }
                    }

                    // User info section
                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 16
                        align: {y: 0.5}

                        // Avatar
                        <View> {
                            width: 64, height: 64
                            show_bg: true
                            draw_bg: {
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;
                                    sdf.circle(c.x, c.y, 30.0);
                                    sdf.fill(#e0e7ff);
                                    return sdf.result;
                                }
                            }
                            align: {x: 0.5, y: 0.5}
                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/user.svg")
                                    fn get_color(self) -> vec4 { return #6366f1; }
                                }
                                icon_walk: {width: 32, height: 32}
                            }
                        }

                        <View> {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: 4

                            <Label> {
                                text: "Demo User"
                                draw_text: {
                                    color: (TEXT_PRIMARY)
                                    text_style: <FONT_SEMIBOLD>{ font_size: 16.0 }
                                }
                            }
                            <Label> {
                                text: "demo@mofa.studio"
                                draw_text: {
                                    color: (TEXT_SECONDARY)
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                }
                            }
                        }
                    }

                    <Label> {
                        text: "Profile settings coming soon..."
                        margin: {top: 20}
                        draw_text: {
                            color: #94a3b8
                            text_style: <FONT_REGULAR>{ font_size: 13.0 }
                        }
                    }
                }

                // Settings Page content (in tab overlay) - Use actual SettingsScreen widget
                settings_tab_page = <SettingsScreen> {
                    width: Fill, height: Fill
                    visible: false
                }
            }
        }
    }

    App = {{App}} {
        ui: <Window> {
            window: { title: "MoFA Studio", inner_size: vec2(1400, 900) }
            pass: { clear_color: (DARK_BG) }
            flow: Overlay

            body = <Dashboard> {}

            // Invisible hover zone for sidebar trigger (hamburger button)
            sidebar_trigger_overlay = <View> {
                width: 28, height: 28
                abs_pos: vec2(18.0, 16.0)
                cursor: Hand
            }

            // Sidebar menu overlay - positioned to connect with hamburger button (no gap)
            // Height is Fit so it adapts to sidebar content
            sidebar_menu_overlay = <View> {
                width: 180, height: Fit
                abs_pos: vec2(0.0, 52.0)
                visible: false
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

                sidebar_content = <Sidebar> {}
            }

            // Invisible hover zone for user profile button
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
                }

                <View> { width: Fill, height: 1, show_bg: true, draw_bg: { color: #e5e7eb } }

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
                }
            }
        }
    }
}

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

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    user_menu_open: bool,
    #[rust]
    sidebar_menu_open: bool,
    #[rust]
    open_tabs: Vec<String>,  // Only "profile" and "settings" tabs
    #[rust]
    active_tab: Option<String>,
    #[rust]
    last_window_size: DVec2,  // Track window size for responsive positioning
    // Sidebar slide animation state
    #[rust]
    sidebar_animating: bool,
    #[rust]
    sidebar_animation_start: f64,
    #[rust]
    sidebar_slide_in: bool,  // true = sliding in, false = sliding out
    // Note: log_panel_collapsed, log_panel_width, audio_devices, splitter_dragging
    // are now managed by MoFaFMScreen widget internally
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        // Register shared widgets from mofa-widgets (must come before apps that use them)
        mofa_widgets::live_design(cx);
        // Register shell widgets (sidebar, etc.)
        mofa_studio_shell::widgets::participant_panel::live_design(cx);
        mofa_studio_shell::widgets::sidebar::live_design(cx);
        mofa_studio_shell::widgets::log_panel::live_design(cx);
        // Register app widgets
        mofa_fm::live_design(cx);
        mofa_settings::live_design(cx);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());

        // Handle window resize - update overlay positions dynamically
        if let Event::WindowGeomChange(wg) = event {
            let new_size = wg.new_geom.inner_size;
            if new_size != self.last_window_size {
                self.last_window_size = new_size;
                self.update_overlay_positions(cx);
            }
        }

        // Also check on draw to handle initial layout
        if let Event::Draw(_) = event {
            let window_rect = self.ui.area().rect(cx);
            if window_rect.size.x > 0.0 && window_rect.size != self.last_window_size {
                self.last_window_size = window_rect.size;
                self.update_overlay_positions(cx);
            }
        }

        // Update sidebar slide animation
        if self.sidebar_animating {
            self.update_sidebar_animation(cx);
        }

        // Extract actions from event
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

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
                if !self.sidebar_menu_open && !self.sidebar_animating {
                    self.sidebar_menu_open = true;
                    self.start_sidebar_slide_in(cx);
                }
            }
            _ => {}
        }

        // Hide sidebar when mouse moves away from both trigger and sidebar
        if self.sidebar_menu_open && !self.sidebar_animating {
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
                    self.start_sidebar_slide_out(cx);
                }
            }
        }

        // Note: Splitter dragging is now handled by MoFaFMScreen

        // Handle sidebar menu item clicks
        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.mofa_fm_tab)).clicked(&actions) {
            self.sidebar_menu_open = false;
            self.start_sidebar_slide_out(cx);
            // Close tab overlay to show main content area with the single MoFaFMScreen
            self.open_tabs.clear();
            self.active_tab = None;
            self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
            // Use apply_over for visibility toggling in main content area
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: true });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: false });
            self.ui.redraw(cx);
        }

        if self.ui.button(ids!(sidebar_menu_overlay.sidebar_content.settings_tab)).clicked(&actions) {
            self.sidebar_menu_open = false;
            self.start_sidebar_slide_out(cx);
            // Close tab overlay to show main content area
            self.open_tabs.clear();
            self.active_tab = None;
            self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
            // Use apply_over for visibility toggling
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: false });
            self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: true });
            self.ui.redraw(cx);
        }

        // Handle sidebar app button clicks
        let app_buttons = [
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app1_btn), "App 1"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app2_btn), "App 2"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app3_btn), "App 3"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app4_btn), "App 4"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app5_btn), "App 5"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app6_btn), "App 6"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app7_btn), "App 7"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app8_btn), "App 8"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app9_btn), "App 9"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app10_btn), "App 10"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app11_btn), "App 11"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app12_btn), "App 12"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app13_btn), "App 13"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app14_btn), "App 14"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app15_btn), "App 15"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app16_btn), "App 16"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app17_btn), "App 17"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app18_btn), "App 18"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app19_btn), "App 19"),
            (ids!(sidebar_menu_overlay.sidebar_content.apps_scroll.app20_btn), "App 20"),
        ];
        for (app_id, _app_name) in app_buttons {
            if self.ui.button(app_id).clicked(&actions) {
                self.sidebar_menu_open = false;
                self.start_sidebar_slide_out(cx);
                // Close tab overlay to show main content area
                self.open_tabs.clear();
                self.active_tab = None;
                self.ui.view(ids!(body.tab_overlay)).set_visible(cx, false);
                // Use apply_over for visibility toggling
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page)).apply_over(cx, live!{ visible: false });
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.app_page)).apply_over(cx, live!{ visible: true });
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.settings_page)).apply_over(cx, live!{ visible: false });
                self.ui.redraw(cx);
                break;
            }
        }

        // Handle user menu button clicks - open tabs
        if self.ui.button(ids!(user_menu.menu_profile_btn)).clicked(&actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.open_or_switch_tab(cx, "profile");
        }

        if self.ui.button(ids!(user_menu.menu_settings_btn)).clicked(&actions) {
            self.user_menu_open = false;
            self.ui.view(ids!(user_menu)).set_visible(cx, false);
            self.open_or_switch_tab(cx, "settings");
        }

        // Handle MofaHero start/stop button clicks (using event.hits like conference-dashboard)
        let start_view = self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view));
        match event.hits(cx, start_view.area()) {
            Hit::FingerUp(_) => {
                println!("Start MoFA clicked");
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view)).set_visible(cx, false);
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view)).set_visible(cx, true);
                self.ui.redraw(cx);
            }
            _ => {}
        }
        let stop_view = self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view));
        match event.hits(cx, stop_view.area()) {
            Hit::FingerUp(_) => {
                println!("Stop MoFA clicked");
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.start_view)).set_visible(cx, true);
                self.ui.view(ids!(body.dashboard_base.content_area.main_content.content.fm_page.mofa_hero.action_section.stop_view)).set_visible(cx, false);
                self.ui.redraw(cx);
            }
            _ => {}
        }

        // Note: MoFaFMScreen handles its own log panel toggle, splitter drag, and audio device events

        // Handle tab clicks (switch to tab)
        self.handle_tab_clicks(cx, &actions);

        // Handle tab close button clicks
        self.handle_tab_close_clicks(cx, event);
    }
}

impl App {
    /// Update overlay positions based on current window size
    /// This makes the user menu and button responsive to window resizing
    fn update_overlay_positions(&mut self, cx: &mut Cx) {
        let window_width = self.last_window_size.x;
        let window_height = self.last_window_size.y;

        if window_width <= 0.0 {
            return;  // Not ready yet
        }

        // User button overlay - positioned 80px from right edge, 10px from top
        let user_btn_x = window_width - 80.0;
        let user_btn_y = 10.0;
        self.ui.view(ids!(user_btn_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(user_btn_x, user_btn_y))
        });

        // User menu - positioned 150px from right edge (to align with button), 55px from top
        let user_menu_x = window_width - 150.0;
        let user_menu_y = 55.0;
        self.ui.view(ids!(user_menu)).apply_over(cx, live!{
            abs_pos: (dvec2(user_menu_x, user_menu_y))
        });

        // Calculate max scroll height for sidebar apps list when expanded
        // Available height = window height - header (52px) - sidebar padding (30px) - other elements (~150px)
        // This leaves room for: logo_area, mofa_fm_tab, show_more_btn, divider, settings_tab
        let max_scroll_height = (window_height - 230.0).max(200.0);  // Min 200px for usability
        self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content)).set_max_scroll_height(max_scroll_height);

        self.ui.redraw(cx);
    }

    /// Open a tab or switch to it if already open
    fn open_or_switch_tab(&mut self, cx: &mut Cx, tab_id: &str) {
        let tab_id_string = tab_id.to_string();

        // Check if tab is already open
        if !self.open_tabs.iter().any(|t| t == tab_id) {
            self.open_tabs.push(tab_id_string.clone());
        }

        // Set as active tab
        self.active_tab = Some(tab_id_string);

        // Update UI
        self.update_tab_ui(cx);
    }

    /// Close a tab
    fn close_tab(&mut self, cx: &mut Cx, tab_id: &str) {
        // Remove from open tabs
        self.open_tabs.retain(|t| t != tab_id);

        // If closing active tab, switch to another or go home
        if self.active_tab.as_deref() == Some(tab_id) {
            self.active_tab = self.open_tabs.last().cloned();
        }

        // Update UI
        self.update_tab_ui(cx);
    }

    /// Handle tab widget clicks (switch between Profile/Settings tabs)
    fn handle_tab_clicks(&mut self, cx: &mut Cx, actions: &[Action]) {
        // Check if profile tab was clicked
        if self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab)).finger_up(actions).is_some() {
            if self.open_tabs.iter().any(|t| t == "profile") {
                self.active_tab = Some("profile".to_string());
                self.update_tab_ui(cx);
            }
        }

        // Check if settings tab was clicked
        if self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab)).finger_up(actions).is_some() {
            if self.open_tabs.iter().any(|t| t == "settings") {
                self.active_tab = Some("settings".to_string());
                self.update_tab_ui(cx);
            }
        }
    }

    /// Handle tab close button clicks
    fn handle_tab_close_clicks(&mut self, cx: &mut Cx, event: &Event) {
        // Check profile tab close button
        let profile_close = self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab.close_btn));
        match event.hits(cx, profile_close.area()) {
            Hit::FingerUp(_) => {
                self.close_tab(cx, "profile");
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

        // Check settings tab close button
        let settings_close = self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab.close_btn));
        match event.hits(cx, settings_close.area()) {
            Hit::FingerUp(_) => {
                self.close_tab(cx, "settings");
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

    /// Update tab bar and content visibility based on state
    /// Tab overlay is now a modal layer for Profile/Settings only - no duplicate MoFaFMScreen
    fn update_tab_ui(&mut self, cx: &mut Cx) {
        // Check which tabs are open
        let profile_open = self.open_tabs.iter().any(|t| t == "profile");
        let settings_open = self.open_tabs.iter().any(|t| t == "settings");
        let any_tabs_open = !self.open_tabs.is_empty();

        // Check which tab is active
        let profile_active = self.active_tab.as_deref() == Some("profile");
        let settings_active = self.active_tab.as_deref() == Some("settings");

        // Show/hide tab overlay based on whether any tabs are open
        // When no tabs are open, the main content area (with the single MoFaFMScreen) is visible
        self.ui.view(ids!(body.tab_overlay)).set_visible(cx, any_tabs_open);

        // Show/hide individual tabs in tab_bar
        self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab)).set_visible(cx, profile_open);
        self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab)).set_visible(cx, settings_open);

        // Set active state on profile tab
        let profile_active_val = if profile_active { 1.0 } else { 0.0 };
        self.ui.view(ids!(body.tab_overlay.tab_bar.profile_tab))
            .apply_over(cx, live!{ draw_bg: { active: (profile_active_val) } });
        self.ui.label(ids!(body.tab_overlay.tab_bar.profile_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { active: (profile_active_val) } });

        // Set active state on settings tab
        let settings_active_val = if settings_active { 1.0 } else { 0.0 };
        self.ui.view(ids!(body.tab_overlay.tab_bar.settings_tab))
            .apply_over(cx, live!{ draw_bg: { active: (settings_active_val) } });
        self.ui.label(ids!(body.tab_overlay.tab_bar.settings_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { active: (settings_active_val) } });

        // Hide all tab content pages first
        self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, false);
        self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, false);

        // Show the appropriate tab content page based on active tab
        match self.active_tab.as_deref() {
            Some("profile") => {
                self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, true);
            }
            Some("settings") => {
                self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, true);
            }
            _ => {
                // If tabs are open but none is active, default to first open tab
                if profile_open {
                    self.ui.view(ids!(body.tab_overlay.tab_content.profile_page)).set_visible(cx, true);
                } else if settings_open {
                    self.ui.view(ids!(body.tab_overlay.tab_content.settings_tab_page)).set_visible(cx, true);
                }
            }
        }

        self.ui.redraw(cx);
    }

    // Note: All log panel, splitter, and audio device handling is now done by MoFaFMScreen

    /// Update sidebar slide animation
    /// Animates abs_pos.x from -180 (hidden) to 0 (visible) or vice versa
    fn update_sidebar_animation(&mut self, cx: &mut Cx) {
        const ANIMATION_DURATION: f64 = 0.2;  // 200ms
        const SIDEBAR_WIDTH: f64 = 180.0;

        let elapsed = Cx::time_now() - self.sidebar_animation_start;
        let progress = (elapsed / ANIMATION_DURATION).min(1.0);

        // Ease-out cubic for smooth deceleration
        let eased = 1.0 - (1.0 - progress).powi(3);

        // Calculate x position
        let x = if self.sidebar_slide_in {
            // Sliding in: -180 -> 0
            -SIDEBAR_WIDTH * (1.0 - eased)
        } else {
            // Sliding out: 0 -> -180
            -SIDEBAR_WIDTH * eased
        };

        // Apply position
        self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(x, 52.0))
        });

        // Check if animation is complete
        if progress >= 1.0 {
            self.sidebar_animating = false;
            if !self.sidebar_slide_in {
                // Animation finished sliding out, now hide the sidebar
                self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, false);
                // Collapse "Show More" when sidebar is hidden
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
        // Make visible immediately (starting at -180)
        self.ui.view(ids!(sidebar_menu_overlay)).apply_over(cx, live!{
            abs_pos: (dvec2(-180.0, 52.0))
        });
        self.ui.view(ids!(sidebar_menu_overlay)).set_visible(cx, true);
        // Restore selection state when sidebar becomes visible
        self.ui.sidebar(ids!(sidebar_menu_overlay.sidebar_content)).restore_selection_state(cx);
        self.ui.redraw(cx);
    }

    /// Start sidebar slide-out animation
    fn start_sidebar_slide_out(&mut self, cx: &mut Cx) {
        self.sidebar_animating = true;
        self.sidebar_animation_start = Cx::time_now();
        self.sidebar_slide_in = false;
        // Keep visible during animation, will hide when complete
        self.ui.redraw(cx);
    }
}

app_main!(App);

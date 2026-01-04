use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Font definitions for this widget
    FONT_REGULAR = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Regular.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
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

    // Chevron icon for expand/collapse
    ChevronRight = <Icon> {
        draw_icon: {
            svg_file: dep("crate://makepad-widgets/resources/icons/arrow.svg")
            color: #94a3b8
        }
        icon_walk: {width: 10, height: 10}
    }

    // Chevron pointing down (rotated)
    ChevronDown = <Icon> {
        draw_icon: {
            svg_file: dep("crate://makepad-widgets/resources/icons/arrow.svg")
            color: #94a3b8
            fn get_rotation_z(self) -> f64 {
                return 90.0;
            }
        }
        icon_walk: {width: 10, height: 10}
    }

    // Custom sidebar button using Button instead of RadioButton - light theme
    pub SidebarMenuButton = <Button> {
        width: Fill, height: Fit
        padding: {top: 12, bottom: 12, left: 10, right: 10}
        margin: 0
        align: {x: 0.0, y: 0.5}
        icon_walk: {width: 16, height: 16, margin: {right: 10}}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance selected: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#f8fafc, #e2e8f0, self.hover),
                    #dbeafe,
                    self.selected
                );
                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 6.0);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_REGULAR>{ font_size: 9.0 }
            color: #64748b

            fn get_color(self) -> vec4 {
                return #64748b;
            }
        }

        draw_icon: {
            color: #64748b
        }
    }

    // Show More/Less button container with arrow on right
    ShowMoreContainer = <View> {
        width: Fill, height: Fit
        flow: Right
        align: {x: 0.0, y: 0.5}
        spacing: 4

        show_more_bg = <View> {
            width: Fill, height: Fit
            cursor: Hand
            show_bg: true
            draw_bg: {
                instance hover: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let color = mix(#f8fafc, #e2e8f0, self.hover);
                    sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 6.0);
                    sdf.fill(color);
                    return sdf.result;
                }
            }

            show_more_text = <Label> {
                padding: {top: 12, bottom: 12, left: 10}
                text: "Show More"
                draw_text: {
                    text_style: <FONT_REGULAR>{ font_size: 9.0 }
                    color: #1e293b

                    fn get_color(self) -> vec4 {
                        return #1e293b;
                    }
                }
            }
        }

        arrow_label = <Label> {
            padding: {top: 12, bottom: 12, right: 10}
            text: ">"
            draw_text: {
                text_style: <FONT_REGULAR>{ font_size: 11.0 }
                color: #1e293b

                fn get_color(self) -> vec4 {
                    return #1e293b;
                }
            }
        }
    }

    // Main sidebar container - light theme with subtle rounded corners
    // Height is Fit so sidebar adapts to content (compact when collapsed)
    pub Sidebar = {{Sidebar}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 4.0
        padding: {top: 15, bottom: 15, left: 10, right: 10}
        margin: 0

        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Main rectangle with subtle rounded corners - light gray like user dropdown
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                sdf.fill(#f8fafc);

                return sdf.result;
            }
        }

        // Logo area (empty spacer)
        logo_area = <View> {
            width: Fill, height: 5
        }

        // Navigation buttons
        mofa_fm_tab = <SidebarMenuButton> {
            text: "MoFA FM"
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/fm.svg")
            }
        }

        // Apps container - height Fit so it adapts to content
        apps_wrapper = <View> {
            width: Fill, height: Fit
            flow: Down

            // ScrollYView - height Fit when collapsed (no scroll needed)
            // When expanded, height is set dynamically to enable scrolling
            apps_scroll = <ScrollYView> {
                width: Fill, height: Fit
                flow: Down
                spacing: 4
                scroll_bars: <ScrollBars> {
                    show_scroll_x: false
                    show_scroll_y: true
                }

            // First 4 apps - always visible
            app1_btn = <SidebarMenuButton> { text: "App 1", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app2_btn = <SidebarMenuButton> { text: "App 2", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app3_btn = <SidebarMenuButton> { text: "App 3", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app4_btn = <SidebarMenuButton> { text: "App 4", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }

            // Pinned app from "Show More" section - appears when an app from expanded section is selected
            pinned_app_btn = <SidebarMenuButton> {
                visible: false
                text: ""
                draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") }
            }

            // Show More button
            show_more_btn = <ShowMoreContainer> {}

            // Collapsible section for apps 5-20 (hidden by default)
            more_apps_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 4
                visible: false

                app5_btn = <SidebarMenuButton> { text: "App 5", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app6_btn = <SidebarMenuButton> { text: "App 6", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app7_btn = <SidebarMenuButton> { text: "App 7", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app8_btn = <SidebarMenuButton> { text: "App 8", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app9_btn = <SidebarMenuButton> { text: "App 9", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app10_btn = <SidebarMenuButton> { text: "App 10", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app11_btn = <SidebarMenuButton> { text: "App 11", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app12_btn = <SidebarMenuButton> { text: "App 12", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app13_btn = <SidebarMenuButton> { text: "App 13", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app14_btn = <SidebarMenuButton> { text: "App 14", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app15_btn = <SidebarMenuButton> { text: "App 15", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app16_btn = <SidebarMenuButton> { text: "App 16", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app17_btn = <SidebarMenuButton> { text: "App 17", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app18_btn = <SidebarMenuButton> { text: "App 18", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app19_btn = <SidebarMenuButton> { text: "App 19", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                app20_btn = <SidebarMenuButton> { text: "App 20", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            }
        }
        }

        // Divider before settings
        <View> {
            width: Fill, height: 1
            margin: {top: 8, bottom: 8}
            show_bg: true
            draw_bg: { color: #e2e8f0 }
        }

        settings_tab = <SidebarMenuButton> {
            text: "Settings"
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/settings.svg")
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum SidebarSelection {
    MofaFM,
    App(usize),  // 1-20
    Settings,
}

#[derive(Live, LiveHook, Widget)]
pub struct Sidebar {
    #[deref]
    view: View,

    #[rust]
    more_apps_visible: bool,

    #[rust]
    selection: Option<SidebarSelection>,  // Track current selection

    #[rust]
    pinned_app_name: Option<String>,  // Name of the pinned app from "Show More" section

    #[rust]
    max_scroll_height: f64,  // Max height for apps_scroll when expanded (set by app.rs)
}

impl Widget for Sidebar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle show more/less click
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Check if show_more_bg view was clicked
        if self.view.view(ids!(apps_wrapper.apps_scroll.show_more_btn.show_more_bg)).finger_up(&actions).is_some() {
            self.more_apps_visible = !self.more_apps_visible;

            // Toggle visibility of more apps section
            self.view.view(ids!(apps_wrapper.apps_scroll.more_apps_section))
                .set_visible(cx, self.more_apps_visible);

            // Update text and arrow labels
            if self.more_apps_visible {
                self.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.show_more_text))
                    .set_text(cx, "Show Less");
                self.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.arrow_label))
                    .set_text(cx, "^");
                // When expanded, set a max height for scrolling
                // Use stored max_scroll_height or default to 400
                let scroll_height = if self.max_scroll_height > 0.0 { self.max_scroll_height } else { 400.0 };
                self.view.view(ids!(apps_wrapper.apps_scroll)).apply_over(cx, live!{
                    height: (scroll_height)
                });
            } else {
                self.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.show_more_text))
                    .set_text(cx, "Show More");
                self.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.arrow_label))
                    .set_text(cx, ">");
                // When collapsed, reset to Fit (no scrolling needed)
                self.view.view(ids!(apps_wrapper.apps_scroll)).apply_over(cx, live!{
                    height: Fit
                });
            }

            self.view.redraw(cx);
        }

        // Handle MoFA FM tab click
        if self.view.button(ids!(mofa_fm_tab)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::MofaFM);
        }

        // Handle Settings tab click
        if self.view.button(ids!(settings_tab)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::Settings);
        }

        // Handle pinned app button click (acts same as the original app)
        if self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).clicked(actions) {
            if let Some(SidebarSelection::App(app_idx)) = &self.selection {
                if *app_idx >= 5 {
                    // Re-select the same app (refresh selection state)
                    self.handle_selection(cx, SidebarSelection::App(*app_idx));
                }
            }
        }

        // Handle app button clicks - use static ids! paths for correct resolution
        // First 4 apps are directly in apps_scroll
        if self.view.button(ids!(apps_scroll.app1_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(1));
        }
        if self.view.button(ids!(apps_scroll.app2_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(2));
        }
        if self.view.button(ids!(apps_scroll.app3_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(3));
        }
        if self.view.button(ids!(apps_scroll.app4_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(4));
        }
        // Apps 5-20 are in the more_apps_section
        if self.view.button(ids!(apps_scroll.more_apps_section.app5_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(5));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app6_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(6));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app7_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(7));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app8_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(8));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app9_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(9));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app10_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(10));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app11_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(11));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app12_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(12));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app13_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(13));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app14_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(14));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app15_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(15));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app16_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(16));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app17_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(17));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app18_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(18));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app19_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(19));
        }
        if self.view.button(ids!(apps_scroll.more_apps_section.app20_btn)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::App(20));
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl Sidebar {
    fn handle_selection(&mut self, cx: &mut Cx, selection: SidebarSelection) {
        self.selection = Some(selection.clone());

        // Clear all selections first
        self.clear_all_selections(cx);

        // Apply selected state based on what was clicked
        match &selection {
            SidebarSelection::MofaFM => {
                self.view.button(ids!(mofa_fm_tab)).apply_over(cx, live!{ draw_bg: { selected: 1.0 } });
                // Hide pinned app when MoFA FM is selected
                self.pinned_app_name = None;
                self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).set_visible(cx, false);
            }
            SidebarSelection::App(app_idx) => {
                self.set_app_button_selected(cx, *app_idx, true);

                // Handle pinned app display for "Show More" section apps (5-20)
                if *app_idx >= 5 {
                    let app_name = format!("App {}", app_idx);
                    self.pinned_app_name = Some(app_name.clone());

                    self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).set_text(cx, &app_name);
                    self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).set_visible(cx, true);
                    self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).apply_over(cx, live!{
                        draw_bg: { selected: 1.0 }
                    });
                } else {
                    self.pinned_app_name = None;
                    self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).set_visible(cx, false);
                }
            }
            SidebarSelection::Settings => {
                self.view.button(ids!(settings_tab)).apply_over(cx, live!{ draw_bg: { selected: 1.0 } });
                // Hide pinned app when Settings is selected
                self.pinned_app_name = None;
                self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).set_visible(cx, false);
            }
        }

        self.view.redraw(cx);
    }

    fn clear_all_selections(&mut self, cx: &mut Cx) {
        // Clear MoFA FM and Settings tabs
        self.view.button(ids!(mofa_fm_tab)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(settings_tab)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });

        // Clear first 4 apps
        self.view.button(ids!(apps_wrapper.apps_scroll.app1_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.app2_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.app3_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.app4_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });

        // Clear apps 5-20 in more_apps_section
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app5_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app6_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app7_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app8_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app9_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app10_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app11_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app12_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app13_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app14_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app15_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app16_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app17_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app18_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app19_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
        self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app20_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });

        // Clear pinned app button
        self.view.button(ids!(apps_wrapper.apps_scroll.pinned_app_btn)).apply_over(cx, live!{ draw_bg: { selected: 0.0 } });
    }

    fn set_app_button_selected(&mut self, cx: &mut Cx, app_idx: usize, selected: bool) {
        let selected_val = if selected { 1.0 } else { 0.0 };

        match app_idx {
            1 => self.view.button(ids!(apps_wrapper.apps_scroll.app1_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            2 => self.view.button(ids!(apps_wrapper.apps_scroll.app2_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            3 => self.view.button(ids!(apps_wrapper.apps_scroll.app3_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            4 => self.view.button(ids!(apps_wrapper.apps_scroll.app4_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            5 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app5_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            6 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app6_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            7 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app7_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            8 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app8_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            9 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app9_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            10 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app10_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            11 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app11_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            12 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app12_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            13 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app13_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            14 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app14_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            15 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app15_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            16 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app16_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            17 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app17_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            18 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app18_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            19 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app19_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            20 => self.view.button(ids!(apps_wrapper.apps_scroll.more_apps_section.app20_btn)).apply_over(cx, live!{ draw_bg: { selected: (selected_val) } }),
            _ => {}
        }
    }
}

impl SidebarRef {
    /// Set the maximum scroll height for the apps list when expanded
    /// This should be called by app.rs when window size changes
    pub fn set_max_scroll_height(&self, max_height: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.max_scroll_height = max_height;
        }
    }

    /// Collapse the "Show More" section when sidebar is hidden
    pub fn collapse_show_more(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            if inner.more_apps_visible {
                inner.more_apps_visible = false;

                // Hide the more apps section
                inner.view.view(ids!(apps_wrapper.apps_scroll.more_apps_section))
                    .set_visible(cx, false);

                // Update text and arrow labels
                inner.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.show_more_text))
                    .set_text(cx, "Show More");
                inner.view.label(ids!(apps_wrapper.apps_scroll.show_more_btn.arrow_label))
                    .set_text(cx, ">");

                // Reset scroll height to Fit
                inner.view.view(ids!(apps_wrapper.apps_scroll)).apply_over(cx, live!{
                    height: Fit
                });

                inner.view.redraw(cx);
            }
        }
    }

    /// Restore the selection visual state (call when sidebar becomes visible)
    pub fn restore_selection_state(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            // First clear all selections
            inner.clear_all_selections(cx);

            // Then restore based on current selection
            if let Some(selection) = inner.selection.clone() {
                match selection {
                    SidebarSelection::MofaFM => {
                        inner.view.button(ids!(mofa_fm_tab)).apply_over(cx, live!{ draw_bg: { selected: 1.0 } });
                    }
                    SidebarSelection::App(app_idx) => {
                        inner.set_app_button_selected(cx, app_idx, true);

                        // Restore pinned app for Show More apps (5-20)
                        if app_idx >= 5 {
                            let app_name = format!("App {}", app_idx);
                            inner.view.button(ids!(apps_scroll.pinned_app_btn)).set_text(cx, &app_name);
                            inner.view.button(ids!(apps_scroll.pinned_app_btn)).set_visible(cx, true);
                            inner.view.button(ids!(apps_scroll.pinned_app_btn)).apply_over(cx, live!{
                                draw_bg: { selected: 1.0 }
                            });
                        }
                    }
                    SidebarSelection::Settings => {
                        inner.view.button(ids!(settings_tab)).apply_over(cx, live!{ draw_bg: { selected: 1.0 } });
                    }
                }
            }
            inner.view.redraw(cx);
        }
    }
}

// Navigation uses button clicks, handled in app.rs

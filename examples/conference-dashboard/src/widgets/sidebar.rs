use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::widgets::theme::FONT_FAMILY;
    use crate::widgets::theme::FONT_REGULAR;
    use crate::widgets::theme::FONT_BOLD;

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

    // Main sidebar container - light theme with subtle rounded corners
    pub Sidebar = {{Sidebar}} {
        width: Fill, height: Fill
        flow: Down
        spacing: 8.0
        padding: {top: 15, bottom: 15, left: 10, right: 10}
        margin: 0

        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Main rectangle with subtle rounded corners
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                sdf.fill(#ffffff);

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
            draw_bg: { selected: 1.0 }
            draw_text: { color: #1e40af }
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/fm.svg")
                color: #1e40af
            }
        }

        // Divider
        <View> {
            width: Fill, height: 1
            margin: {top: 8, bottom: 8}
            show_bg: true
            draw_bg: { color: #e2e8f0 }
        }

        // Demo apps section label
        <Label> {
            text: "Demo Apps"
            margin: {left: 4, bottom: 4}
            draw_text: {
                text_style: { font_size: 8.0 }
                color: #94a3b8
            }
        }

        // Scrollable apps list
        apps_scroll = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            spacing: 4
            scroll_bars: <ScrollBars> {
                show_scroll_x: false
                show_scroll_y: true
            }

            app1_btn = <SidebarMenuButton> { text: "App 1", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app2_btn = <SidebarMenuButton> { text: "App 2", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app3_btn = <SidebarMenuButton> { text: "App 3", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
            app4_btn = <SidebarMenuButton> { text: "App 4", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
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

#[derive(Live, LiveHook, Widget)]
pub struct Sidebar {
    #[deref]
    view: View,
}

impl Widget for Sidebar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

// Navigation uses button clicks, handled in app.rs

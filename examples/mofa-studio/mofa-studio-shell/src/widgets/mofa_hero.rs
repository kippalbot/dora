//! MofaHero Widget - System status bar with Dataflow, Audio Buffer, CPU, and Memory panels

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import from shared theme (single source of truth)
    use mofa_widgets::theme::*;

    // Local panel background (slightly darker than WHITE for contrast)
    HERO_PANEL_BG = (GRAY_100)

    // Icons
    ICO_START = dep("crate://self/resources/icons/start.svg")
    ICO_STOP = dep("crate://self/resources/icons/stop.svg")

    // Action button (start/stop toggle)
    ActionButton = <Button> {
        width: 36, height: 36
        align: {x: 0.5, y: 0.5}
        icon_walk: {width: 24, height: 24}
        draw_icon: {
            svg_file: (ICO_START)
            instance is_running: 0.0  // 0=stopped(green play), 1=running(red stop)
            fn get_color(self) -> vec4 {
                let green = vec4(0.13, 0.77, 0.37, 1.0);
                let red = vec4(0.95, 0.25, 0.25, 1.0);
                return mix(green, red, self.is_running);
            }
        }
        draw_bg: {
            fn pixel(self) -> vec4 {
                return vec4(0.0, 0.0, 0.0, 0.0);  // Transparent
            }
        }
    }

    // Reusable status dot component
    StatusDot = <View> {
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

    // Reusable LED gauge component (10 segments, blue to red)
    LedGauge = <View> {
        width: Fill, height: 20
        show_bg: true
        draw_bg: {
            instance fill_pct: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Light background (GRAY_200)
                sdf.rect(0.0, 0.0, self.rect_size.x, self.rect_size.y);
                sdf.fill((GRAY_200));

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

    // Dataflow status button
    DataflowButton = <Button> {
        width: Fill, height: 20
        align: {x: 0.5, y: 0.5}
        text: "Ready"
        draw_text: {
            color: (WHITE)
            text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            fn get_color(self) -> vec4 {
                return self.color;
            }
        }
        draw_bg: {
            instance status: 0.0  // 0=ready(gray), 1=connected(green), 2=error(red)
            border_radius: 4.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Colors for each status
                let gray = vec4(0.612, 0.639, 0.686, 1.0);    // Ready
                let green = vec4(0.13, 0.77, 0.37, 1.0);      // Connected
                let red = vec4(0.95, 0.25, 0.25, 1.0);        // Error

                let color = mix(
                    mix(gray, green, step(0.5, self.status)),
                    red,
                    step(1.5, self.status)
                );

                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    // Status section template
    StatusSection = <RoundedView> {
        width: 150, height: Fill
        padding: 6
        draw_bg: {
            color: (HERO_PANEL_BG)
            border_radius: 2.0
        }
        flow: Down
        spacing: 4
        align: {x: 0.5, y: 0.0}
    }

    pub MofaHero = {{MofaHero}} <View> {
        width: Fill, height: 70
        flow: Right
        spacing: 12

        // Action section (start/stop button)
        action_section = <RoundedView> {
            width: 150, height: Fill
            padding: 6
            draw_bg: {
                color: (HERO_PANEL_BG)
                border_radius: 2.0
            }
            flow: Down
            spacing: 4
            align: {x: 0.5, y: 0.0}

            // Start state (visible when not running)
            start_view = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 4
                align: {x: 0.5, y: 0.0}
                cursor: Hand

                action_start_label = <Label> {
                    text: "Start MoFA"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }

                start_btn = <View> {
                    width: 24, height: 20
                    align: {x: 0.5, y: 0.5}
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_START)
                            fn get_color(self) -> vec4 {
                                return vec4(0.133, 0.773, 0.373, 1.0);  // Green #22c55e
                            }
                        }
                        icon_walk: {width: 20, height: 20}
                    }
                }
            }

            // Stop state (hidden by default, shown when running)
            stop_view = <View> {
                visible: false
                width: Fill, height: Fill
                flow: Down
                spacing: 4
                align: {x: 0.5, y: 0.0}
                cursor: Hand

                action_stop_label = <Label> {
                    text: "Stop MoFA"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }

                stop_btn = <View> {
                    width: 24, height: 20
                    align: {x: 0.5, y: 0.5}
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_STOP)
                            fn get_color(self) -> vec4 {
                                return vec4(0.937, 0.267, 0.267, 1.0);  // Red #ef4444
                            }
                        }
                        icon_walk: {width: 20, height: 20}
                    }
                }
            }
        }

        // Dataflow status section
        connection_section = <StatusSection> {
            connection_header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.5, y: 0.5}

                connection_dot = <View> {
                    width: 10, height: 10
                    show_bg: true
                    draw_bg: {
                        color: (GRAY_400)

                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let center = self.rect_size * 0.5;
                            let radius = min(center.x, center.y) - 0.5;
                            sdf.circle(center.x, center.y, radius);
                            sdf.fill(self.color);
                            return sdf.result;
                        }
                    }
                }

                connection_label = <Label> {
                    text: "Dataflow"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }
            }

            dataflow_btn_container = <View> {
                width: Fill, height: 20
                align: {x: 0.5, y: 0.5}
                dataflow_btn = <DataflowButton> {}
            }

            // Spacer to match 3-row layout
            <Label> {
                text: " "
                draw_text: {
                    color: (GRAY_700)
                    text_style: { font_size: 10.0 }
                }
            }
        }

        // Audio Buffer section
        buffer_section = <StatusSection> {
            buffer_header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.5, y: 0.5}

                buffer_status_dot = <StatusDot> {}

                buffer_label = <Label> {
                    text: "Audio Buffer"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }
            }

            buffer_gauge = <LedGauge> {}

            buffer_pct_label = <Label> {
                text: "0%"
                draw_text: {
                    color: (GRAY_700)
                    text_style: { font_size: 10.0 }
                }
            }
        }

        // CPU section
        cpu_section = <StatusSection> {
            cpu_header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.5, y: 0.5}

                cpu_status_dot = <StatusDot> {}

                cpu_label = <Label> {
                    text: "CPU"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }
            }

            cpu_gauge = <LedGauge> {}

            cpu_pct_label = <Label> {
                text: "0%"
                draw_text: {
                    color: (GRAY_700)
                    text_style: { font_size: 10.0 }
                }
            }
        }

        // Memory section
        memory_section = <StatusSection> {
            memory_header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.5, y: 0.5}

                memory_status_dot = <StatusDot> {}

                memory_label = <Label> {
                    text: "Memory"
                    draw_text: {
                        color: (GRAY_700)
                        text_style: { font_size: 10.0 }
                    }
                }
            }

            memory_gauge = <LedGauge> {}

            memory_pct_label = <Label> {
                text: "0%"
                draw_text: {
                    color: (GRAY_700)
                    text_style: { font_size: 10.0 }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MofaHero {
    #[deref]
    view: View,
}

impl Widget for MofaHero {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::widgets::theme::FONT_FAMILY;
    use crate::widgets::theme::FONT_REGULAR;
    use crate::widgets::theme::FONT_BOLD;
    use crate::widgets::theme::FONT_SEMIBOLD;

    // MoFa FM placeholder screen
    pub MoFaFMScreen = {{MoFaFMScreen}} {
        width: Fill, height: Fill
        flow: Down
        align: {x: 0.5, y: 0.5}
        spacing: 20

        show_bg: true
        draw_bg: {
            color: #f8fafc
        }

        // Icon placeholder
        <View> {
            width: 80, height: 80
            align: {x: 0.5, y: 0.5}
            show_bg: true
            draw_bg: {
                instance radius: 40.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let c = self.rect_size * 0.5;

                    // Background circle
                    sdf.circle(c.x, c.y, self.radius);
                    sdf.fill(#e0e7ff);

                    // Center circle (radio icon)
                    sdf.circle(c.x, c.y, 12.0);
                    sdf.fill(#6366f1);

                    // Inner ring
                    sdf.circle(c.x, c.y, 22.0);
                    sdf.stroke(#6366f1, 2.5);

                    // Outer ring
                    sdf.circle(c.x, c.y, 32.0);
                    sdf.stroke(#818cf8, 2.0);

                    return sdf.result;
                }
            }
        }

        // Title
        <Label> {
            text: "MoFa FM"
            draw_text: {
                color: #1e293b
                text_style: <FONT_BOLD>{ font_size: 28.0 }
            }
        }

        // Subtitle
        <Label> {
            text: "Coming Soon..."
            draw_text: {
                color: #64748b
                text_style: <FONT_REGULAR>{ font_size: 14.0 }
            }
        }

        // Description
        <View> {
            width: 400, height: Fit
            align: {x: 0.5, y: 0.5}
            padding: {top: 20}

            <Label> {
                width: Fill
                text: "MoFa FM will provide AI-powered audio streaming and podcast features. Stay tuned for updates!"
                draw_text: {
                    color: #94a3b8
                    text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    wrap: Word
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MoFaFMScreen {
    #[deref]
    view: View,
}

impl Widget for MoFaFMScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

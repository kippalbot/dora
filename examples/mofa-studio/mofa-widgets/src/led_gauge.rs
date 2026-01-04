//! Buffer Gauge Widget - Visual representation of audio buffer fill level

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    pub BufferGauge = {{BufferGauge}} <View> {
        width: Fill, height: 80
        show_bg: true

        draw_bg: {
            instance fill_pct: 0.0

            fn get_fill_color(self, pct: float) -> vec4 {
                // Red when above 80%, green otherwise
                if pct > 0.8 {
                    return #ef4444;
                } else {
                    return #22c55e;
                }
            }

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Light background
                sdf.box(4.0, 4.0, self.rect_size.x - 8.0, self.rect_size.y - 8.0, 3.0);
                sdf.fill(#f3f4f6);

                // Fill bar
                let bar_width = (self.rect_size.x - 16.0) * self.fill_pct;
                if bar_width > 0.0 {
                    sdf.box(8.0, 8.0, bar_width, self.rect_size.y - 16.0, 2.0);
                    sdf.fill(self.get_fill_color(self.fill_pct));
                }

                // Border
                sdf.box(4.0, 4.0, self.rect_size.x - 8.0, self.rect_size.y - 8.0, 3.0);
                sdf.stroke(#d1d5db, 1.5);

                return sdf.result;
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct BufferGauge {
    #[deref]
    view: View,
}

impl Widget for BufferGauge {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

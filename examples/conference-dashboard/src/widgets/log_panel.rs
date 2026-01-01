//! Log Panel Widget - Display system log messages with Markdown support

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    LOG_BG = #f9fafb

    pub LogPanel = {{LogPanel}} <View> {
        width: Fill, height: Fill
        show_bg: true
        draw_bg: { color: (LOG_BG) }

        log_scroll = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            padding: 8

            log_content = <Markdown> {
                width: Fill, height: Fit
                font_size: 10.0
                font_color: #4b5563
                paragraph_spacing: 4

                draw_normal: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_bold: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_italic: {
                    text_style: {
                        font_size: 10.0
                    }
                }
                draw_fixed: {
                    text_style: {
                        font_size: 9.0
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct LogPanel {
    #[deref]
    view: View,
}

impl Widget for LogPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

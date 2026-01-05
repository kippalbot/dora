//! Add Provider Modal - Dialog for adding custom providers

use makepad_widgets::*;
use crate::data::{Provider, ProviderId, ProviderType, ProviderConnectionStatus};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    // Modal text input
    ModalTextInput = <TextInput> {
        width: Fill, height: 44
        padding: {left: 12, right: 12, top: 10, bottom: 10}

        draw_bg: {
            instance radius: 6.0
            instance border_width: 1.0
            instance border_color: (GRAY_300)

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(
                    self.border_width,
                    self.border_width,
                    self.rect_size.x - self.border_width * 2.0,
                    self.rect_size.y - self.border_width * 2.0,
                    max(1.0, self.radius - self.border_width)
                );
                sdf.fill((WHITE));
                sdf.stroke(self.border_color, self.border_width);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_REGULAR>{ font_size: 11.0 }
            color: (TEXT_PRIMARY)
        }

        draw_selection: {
            color: (BLUE_200)
        }

        draw_cursor: {
            color: (ACCENT_BLUE)
        }
    }

    // Modal button - primary
    ModalPrimaryButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix((ACCENT_BLUE), (BLUE_600), self.hover),
                    (BLUE_700),
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (WHITE)

            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }

        text: "Add Provider"
    }

    // Modal button - secondary (cancel)
    ModalSecondaryButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix((WHITE), (GRAY_50), self.hover),
                    (GRAY_100),
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                sdf.stroke((GRAY_300), 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (GRAY_700)

            fn get_color(self) -> vec4 {
                return (GRAY_700);
            }
        }

        text: "Cancel"
    }

    // Add provider modal dialog
    pub AddProviderModal = {{AddProviderModal}} {
        width: Fill, height: Fill
        flow: Overlay
        visible: false

        // Overlay background
        overlay = <View> {
            width: Fill, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return vec4(0.0, 0.0, 0.0, 0.5);
                }
            }
        }

        // Center the dialog
        dialog_container = <View> {
            width: Fill, height: Fill
            align: {x: 0.5, y: 0.5}

            // Modal dialog
            dialog = <View> {
                width: 480, height: Fit
                padding: 24
                flow: Down
                spacing: 20

                show_bg: true
                draw_bg: {
                    instance radius: 12.0

                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.radius);
                        sdf.fill((WHITE));
                        return sdf.result;
                    }
                }

                // Header
                header = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}

                    <Label> {
                        text: "Add Custom Provider"
                        draw_text: {
                            color: (SLATE_800)
                            text_style: <FONT_BOLD>{ font_size: 16.0 }
                        }
                    }
                }

                // Name field
                name_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    <Label> {
                        text: "Provider Name"
                        draw_text: {
                            color: (GRAY_700)
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                        }
                    }

                    name_input = <ModalTextInput> {
                        empty_text: "My Custom Provider"
                    }
                }

                // API Host field
                host_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    <Label> {
                        text: "API Host"
                        draw_text: {
                            color: (GRAY_700)
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                        }
                    }

                    host_input = <ModalTextInput> {
                        empty_text: "https://api.example.com/v1"
                    }
                }

                // API Key field (optional)
                key_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    <Label> {
                        text: "API Key (optional)"
                        draw_text: {
                            color: (GRAY_700)
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                        }
                    }

                    key_input = <ModalTextInput> {
                        empty_text: "sk-..."
                        is_password: true
                    }
                }

                // Buttons
                actions = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {x: 1.0, y: 0.5}
                    spacing: 12
                    margin: {top: 8}

                    cancel_button = <ModalSecondaryButton> {}
                    add_button = <ModalPrimaryButton> {}
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct AddProviderModal {
    #[deref]
    view: View,
}

impl Widget for AddProviderModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AddProviderModalRef {
    /// Show the modal
    pub fn show(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.set_visible(cx, true);

            // Clear inputs
            inner.view.text_input(ids!(name_input)).set_text(cx, "");
            inner.view.text_input(ids!(host_input)).set_text(cx, "");
            inner.view.text_input(ids!(key_input)).set_text(cx, "");

            inner.view.redraw(cx);
        }
    }

    /// Hide the modal
    pub fn hide(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.set_visible(cx, false);
            inner.view.redraw(cx);
        }
    }

    /// Check if modal is visible
    pub fn is_visible(&self) -> bool {
        self.borrow().map(|inner| inner.view.visible()).unwrap_or(false)
    }

    /// Get the form values and create a Provider
    pub fn get_provider(&self) -> Option<Provider> {
        self.borrow().and_then(|inner| {
            let name = inner.view.text_input(ids!(name_input)).text();
            let host = inner.view.text_input(ids!(host_input)).text();
            let key = {
                let k = inner.view.text_input(ids!(key_input)).text();
                if k.is_empty() { None } else { Some(k) }
            };

            if name.is_empty() || host.is_empty() {
                return None;
            }

            // Generate a unique ID from the name
            let id = ProviderId::from(
                name.to_lowercase()
                    .replace(" ", "_")
                    .replace("-", "_")
            );

            Some(Provider {
                id,
                name,
                url: host,
                api_key: key,
                provider_type: ProviderType::Custom,
                enabled: true,
                models: vec![],
                is_custom: true,
                connection_status: ProviderConnectionStatus::Disconnected,
            })
        })
    }
}

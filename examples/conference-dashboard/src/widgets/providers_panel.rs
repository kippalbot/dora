use makepad_widgets::*;
use crate::data::ProviderId;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::widgets::theme::FONT_FAMILY;
    use crate::widgets::theme::FONT_REGULAR;
    use crate::widgets::theme::FONT_BOLD;
    use crate::widgets::theme::FONT_SEMIBOLD;

    ICO_OPENAI = dep("crate://self/resources/icons/openai.svg")
    ICO_DEEPSEEK = dep("crate://self/resources/icons/deepseek.svg")
    IMG_QWEN = dep("crate://self/resources/icons/qwen.png")

    // Provider item background - for hover and selection effects
    ProviderItemBg = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        flow: Right
        spacing: 10
        align: {x: 0.0, y: 0.5}
        cursor: Hand

        show_bg: true
        draw_bg: {
            instance hover: 0.0
            instance selected: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#ffffff, #f1f5f9, self.hover),
                    #eff6ff,
                    self.selected
                );
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 0.0);
                sdf.fill(color);

                // Left border for selected
                if self.selected > 0.5 {
                    sdf.box(0.0, 0.0, 3.0, self.rect_size.y, 0.0);
                    sdf.fill(#3b82f6);
                }

                return sdf.result;
            }
        }
    }

    // Provider item in the list - using Button instead of RadioButton
    ProviderItem = <Button> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        margin: 0

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance selected: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#ffffff, #f1f5f9, self.hover),
                    #eff6ff,
                    self.selected
                );
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 0.0);
                sdf.fill(color);

                // Left border for selected
                if self.selected > 0.5 {
                    sdf.box(0.0, 0.0, 3.0, self.rect_size.y, 0.0);
                    sdf.fill(#3b82f6);
                }

                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_REGULAR>{ font_size: 12.0 }
            color: #374151

            fn get_color(self) -> vec4 {
                return #374151;
            }
        }
    }

    // Add provider button
    AddProviderButton = <Button> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#f8fafc, #f1f5f9, self.hover),
                    #e2e8f0,
                    self.pressed
                );
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 0.0);
                sdf.fill(color);

                // Top border
                sdf.box(0.0, 0.0, self.rect_size.x, 1.0, 0.0);
                sdf.fill(#e5e7eb);

                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: #3b82f6

            fn get_color(self) -> vec4 {
                return #3b82f6;
            }
        }

        text: "+ Add Custom Provider"
    }

    // Providers panel - left side of settings
    pub ProvidersPanel = {{ProvidersPanel}} {
        width: 280, height: Fill
        flow: Down
        spacing: 0

        show_bg: true
        draw_bg: {
            color: #ffffff
        }

        // Header
        header = <View> {
            width: Fill, height: Fit
            padding: {left: 16, right: 16, top: 16, bottom: 12}

            <Label> {
                text: "Providers"
                draw_text: {
                    color: #1e293b
                    text_style: <FONT_BOLD>{ font_size: 14.0 }
                }
            }
        }

        // Provider list
        list_container = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 0

            openai_item = <ProviderItemBg> {
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_OPENAI)
                        fn get_color(self) -> vec4 {
                            return #10A37F;
                        }
                    }
                    icon_walk: {width: 24, height: 24}
                }
                <Label> {
                    text: "OpenAI"
                    draw_text: {
                        color: #374151
                        text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    }
                }
            }

            deepseek_item = <ProviderItemBg> {
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_DEEPSEEK)
                        fn get_color(self) -> vec4 {
                            return #4D6BFE;
                        }
                    }
                    icon_walk: {width: 20, height: 20}
                }
                <Label> {
                    text: "DeepSeek"
                    draw_text: {
                        color: #374151
                        text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    }
                }
            }

            alibaba_item = <ProviderItemBg> {
                <Image> {
                    width: 20, height: 20
                    source: (IMG_QWEN)
                }
                <Label> {
                    text: "Alibaba Cloud (Qwen)"
                    draw_text: {
                        color: #374151
                        text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    }
                }
            }
        }

        // Spacer
        <View> { width: Fill, height: Fill }

        // Add button at bottom
        add_button = <AddProviderButton> {}
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ProvidersPanel {
    #[deref]
    view: View,

    #[rust]
    selected_provider_id: Option<ProviderId>,
}

impl Widget for ProvidersPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ProvidersPanelRef {
    /// Get the currently selected provider ID
    pub fn selected_provider_id(&self) -> Option<ProviderId> {
        self.borrow().and_then(|inner| inner.selected_provider_id.clone())
    }

    /// Set the selected provider
    pub fn select_provider(&self, cx: &mut Cx, provider_id: &ProviderId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.selected_provider_id = Some(provider_id.clone());
            inner.view.redraw(cx);
        }
    }

    /// Map item name to provider ID
    pub fn item_to_provider_id(name: &str) -> Option<ProviderId> {
        match name {
            "openai_item" => Some(ProviderId::from("openai")),
            "deepseek_item" => Some(ProviderId::from("deepseek")),
            "alibaba_item" => Some(ProviderId::from("alibaba_cloud")),
            _ => None,
        }
    }

    /// Map button name to provider ID (legacy alias)
    pub fn button_to_provider_id(name: &str) -> Option<ProviderId> {
        Self::item_to_provider_id(name)
    }
}

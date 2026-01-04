//! Providers Panel - List of AI providers

use makepad_widgets::*;
use crate::data::ProviderId;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::FONT_FAMILY;
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_BOLD;
    use mofa_widgets::theme::FONT_SEMIBOLD;

    ICO_OPENAI = dep("crate://self/resources/icons/openai.svg")
    ICO_DEEPSEEK = dep("crate://self/resources/icons/deepseek.svg")
    IMG_QWEN = dep("crate://self/resources/icons/qwen.png")

    // Provider item - matching moly-ai pattern
    ProviderItemBg = <RoundedView> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        margin: 0
        show_bg: true
        draw_bg: {
            border_radius: 0
        }
        cursor: Hand

        // Use same pattern as moly-ai - set color directly on view
        align: {x: 0.0, y: 0.5}
        icon_walk: {width: 24, height: 24, margin: {right: 10}}
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
                        fn get_color(self) -> vec4 { return #10A37F; }
                    }
                    icon_walk: {width: 24, height: 24, margin: {right: 10}}
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
                        fn get_color(self) -> vec4 { return #4D6BFE; }
                    }
                    icon_walk: {width: 20, height: 20, margin: {right: 10}}
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
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_DEEPSEEK)
                        fn get_color(self) -> vec4 { return #6366f1; }
                    }
                    icon_walk: {width: 20, height: 20, margin: {right: 10}}
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

#[derive(Clone, Debug, DefaultNone)]
pub enum ProvidersPanelAction {
    None,
    Selected(ProviderId),
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

        let uid = self.widget_uid();

        // Provider items for hover and click handling
        let items = [
            ids!(list_container.openai_item),
            ids!(list_container.deepseek_item),
            ids!(list_container.alibaba_item),
        ];

        // Handle hover effects using FingerHover events (must be before early return)
        // Skip hover effect on selected item
        for item_id in &items {
            let item = self.view.view(item_id.clone());
            match event.hits(cx, item.area()) {
                Hit::FingerHoverIn(_) => {
                    // Only apply hover if not currently selected
                    let is_selected = match self.selected_provider_id.as_ref().map(|id| id.as_str()) {
                        Some("openai") => item_id == &ids!(list_container.openai_item),
                        Some("deepseek") => item_id == &ids!(list_container.deepseek_item),
                        Some("alibaba_cloud") => item_id == &ids!(list_container.alibaba_item),
                        _ => false,
                    };
                    if !is_selected {
                        self.view.view(item_id.clone()).apply_over(cx, live!{
                            draw_bg: { color: #f1f5f9 }
                        });
                        self.view.redraw(cx);
                    }
                }
                Hit::FingerHoverOut(_) => {
                    // Only reset to white if not currently selected
                    let is_selected = match self.selected_provider_id.as_ref().map(|id| id.as_str()) {
                        Some("openai") => item_id == &ids!(list_container.openai_item),
                        Some("deepseek") => item_id == &ids!(list_container.deepseek_item),
                        Some("alibaba_cloud") => item_id == &ids!(list_container.alibaba_item),
                        _ => false,
                    };
                    if !is_selected {
                        self.view.view(item_id.clone()).apply_over(cx, live!{
                            draw_bg: { color: #ffffff }
                        });
                        self.view.redraw(cx);
                    }
                }
                _ => {}
            }
        }

        // Extract actions - return early if not an Actions event
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Handle provider item clicks
        let mut new_selection: Option<ProviderId> = None;

        if self.view.view(ids!(list_container.openai_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("openai"));
        }
        if self.view.view(ids!(list_container.deepseek_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("deepseek"));
        }
        if self.view.view(ids!(list_container.alibaba_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("alibaba_cloud"));
        }

        if let Some(id) = new_selection {
            // Only process if different from current selection
            if self.selected_provider_id.as_ref() != Some(&id) {
                let selected = id.as_str();
                // First reset all to white
                for item_id in &items {
                    self.view.view(item_id.clone()).apply_over(cx, live!{
                        draw_bg: { color: #ffffff }
                    });
                }
                // Then set selected to blue
                match selected {
                    "openai" => {
                        self.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{
                            draw_bg: { color: #dbeafe }
                        });
                    }
                    "deepseek" => {
                        self.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{
                            draw_bg: { color: #dbeafe }
                        });
                    }
                    "alibaba_cloud" => {
                        self.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{
                            draw_bg: { color: #dbeafe }
                        });
                    }
                    _ => {}
                }
                self.selected_provider_id = Some(id.clone());
                self.view.redraw(cx);
                cx.widget_action(uid, &scope.path, ProvidersPanelAction::Selected(id));
            }
        }
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

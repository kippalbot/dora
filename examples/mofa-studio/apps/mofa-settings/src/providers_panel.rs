//! Providers Panel - List of AI providers

use makepad_widgets::*;
use crate::data::ProviderId;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

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
        flow: Right
        align: {x: 0.0, y: 0.5}
    }

    // Add provider button
    AddProviderButton = <Button> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let light_base = mix((SLATE_50), (HOVER_BG), self.hover);
                let dark_base = mix((SLATE_800), (SLATE_700), self.hover);
                let base = mix(light_base, dark_base, self.dark_mode);
                let light_pressed = (SLATE_200);
                let dark_pressed = (SLATE_600);
                let pressed_color = mix(light_pressed, dark_pressed, self.dark_mode);
                let color = mix(base, pressed_color, self.pressed);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 0.0);
                sdf.fill(color);

                // Top border
                let border = mix((BORDER), (BORDER_DARK), self.dark_mode);
                sdf.box(0.0, 0.0, self.rect_size.x, 1.0, 0.0);
                sdf.fill(border);

                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }

            fn get_color(self) -> vec4 {
                return mix((ACCENT_BLUE), (ACCENT_BLUE_DARK), self.dark_mode);
            }
        }

        text: "+ Add Custom Provider"
    }

    // Provider item - using RoundedView with manual hover via apply_over
    ProviderItem = <RoundedView> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        margin: 0
        show_bg: true
        draw_bg: {
            border_radius: 0
            color: (WHITE)
        }
        cursor: Hand
        flow: Right
        align: {x: 0.0, y: 0.5}
    }

    // Provider label
    ProviderLabel = <Label> {
        draw_text: {
            color: (GRAY_700)
            text_style: <FONT_REGULAR>{ font_size: 12.0 }
        }
    }


    // Providers panel - left side of settings
    pub ProvidersPanel = {{ProvidersPanel}} {
        width: 280, height: Fill
        flow: Down
        spacing: 0

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn get_color(self) -> vec4 {
                return mix((WHITE), (SLATE_800), self.dark_mode);
            }
        }

        // Header
        header = <View> {
            width: Fill, height: Fit
            padding: {left: 16, right: 16, top: 16, bottom: 12}

            header_label = <Label> {
                text: "Providers"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_BOLD>{ font_size: 14.0 }
                    fn get_color(self) -> vec4 {
                        return mix((SLATE_800), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Provider list
        list_container = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 0

            openai_item = <ProviderItem> {
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_OPENAI)
                        fn get_color(self) -> vec4 { return #10A37F; }
                    }
                    icon_walk: {width: 24, height: 24, margin: {right: 10}}
                }
                openai_label = <ProviderLabel> {
                    text: "OpenAI"
                }
            }

            deepseek_item = <ProviderItem> {
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_DEEPSEEK)
                        fn get_color(self) -> vec4 { return #4D6BFE; }
                    }
                    icon_walk: {width: 20, height: 20, margin: {right: 10}}
                }
                deepseek_label = <ProviderLabel> {
                    text: "DeepSeek"
                }
            }

            alibaba_item = <ProviderItem> {
                <Icon> {
                    draw_icon: {
                        svg_file: (ICO_DEEPSEEK)
                        fn get_color(self) -> vec4 { return #6366f1; }
                    }
                    icon_walk: {width: 20, height: 20, margin: {right: 10}}
                }
                alibaba_label = <ProviderLabel> {
                    text: "Alibaba Cloud (Qwen)"
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

    #[rust]
    dark_mode: bool,
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

        // Handle hover effects using FingerHover events
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
                        if self.dark_mode {
                            // SLATE_700 hover color in dark mode (#334155)
                            self.view.view(item_id.clone()).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.2, 0.25, 0.33, 1.0)) }
                            });
                        } else {
                            // SLATE_100 hover color in light mode (#f1f5f9)
                            self.view.view(item_id.clone()).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.95, 0.96, 0.98, 1.0)) }
                            });
                        }
                        self.view.redraw(cx);
                    }
                }
                Hit::FingerHoverOut(_) => {
                    // Only reset if not currently selected
                    let is_selected = match self.selected_provider_id.as_ref().map(|id| id.as_str()) {
                        Some("openai") => item_id == &ids!(list_container.openai_item),
                        Some("deepseek") => item_id == &ids!(list_container.deepseek_item),
                        Some("alibaba_cloud") => item_id == &ids!(list_container.alibaba_item),
                        _ => false,
                    };
                    if !is_selected {
                        if self.dark_mode {
                            // SLATE_800 normal color in dark mode (#1f293b)
                            self.view.view(item_id.clone()).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.12, 0.16, 0.23, 1.0)) }
                            });
                        } else {
                            // White (#ffffff)
                            self.view.view(item_id.clone()).apply_over(cx, live!{
                                draw_bg: { color: (vec4(1.0, 1.0, 1.0, 1.0)) }
                            });
                        }
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
                // First reset all to normal
                for item_id in &items {
                    if self.dark_mode {
                        // Dark normal: #1f293b
                        self.view.view(item_id.clone()).apply_over(cx, live!{
                            draw_bg: { color: (vec4(0.12, 0.16, 0.23, 1.0)) }
                        });
                    } else {
                        // Light normal: #ffffff
                        self.view.view(item_id.clone()).apply_over(cx, live!{
                            draw_bg: { color: (vec4(1.0, 1.0, 1.0, 1.0)) }
                        });
                    }
                }
                // Then set selected color
                match selected {
                    "openai" => {
                        if self.dark_mode {
                            // Dark selected: #1f3a5f
                            self.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.12, 0.23, 0.37, 1.0)) }
                            });
                        } else {
                            // Light selected: #dbeafe
                            self.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.86, 0.92, 1.0, 1.0)) }
                            });
                        }
                    }
                    "deepseek" => {
                        if self.dark_mode {
                            self.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.12, 0.23, 0.37, 1.0)) }
                            });
                        } else {
                            self.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.86, 0.92, 1.0, 1.0)) }
                            });
                        }
                    }
                    "alibaba_cloud" => {
                        if self.dark_mode {
                            self.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.12, 0.23, 0.37, 1.0)) }
                            });
                        } else {
                            self.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{
                                draw_bg: { color: (vec4(0.86, 0.92, 1.0, 1.0)) }
                            });
                        }
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

    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Store dark mode state for hover/selection logic
            inner.dark_mode = dark_mode > 0.5;

            // Panel background
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Header label
            inner.view.label(ids!(header.header_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Provider items - apply background and text colors using vec4
            // Colors: normal dark=#1f293b, normal light=#ffffff
            //         selected dark=#1f3a5f, selected light=#dbeafe
            //         text dark=#f1f5f9, text light=#374151
            let selected = inner.selected_provider_id.as_ref().map(|id| id.as_str());
            let is_dark = inner.dark_mode;

            // Color constants as vec4
            let dark_normal = vec4(0.12, 0.16, 0.23, 1.0);      // #1f293b
            let light_normal = vec4(1.0, 1.0, 1.0, 1.0);        // #ffffff
            let dark_selected = vec4(0.12, 0.23, 0.37, 1.0);    // #1f3a5f
            let light_selected = vec4(0.86, 0.92, 1.0, 1.0);    // #dbeafe
            let dark_text = vec4(0.95, 0.96, 0.98, 1.0);        // #f1f5f9
            let light_text = vec4(0.22, 0.25, 0.32, 1.0);       // #374151

            // OpenAI item
            let is_openai_selected = selected == Some("openai");
            if is_openai_selected && is_dark {
                inner.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{ draw_bg: { color: (dark_selected) } });
            } else if is_openai_selected {
                inner.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{ draw_bg: { color: (light_selected) } });
            } else if is_dark {
                inner.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{ draw_bg: { color: (dark_normal) } });
            } else {
                inner.view.view(ids!(list_container.openai_item)).apply_over(cx, live!{ draw_bg: { color: (light_normal) } });
            }

            // DeepSeek item
            let is_deepseek_selected = selected == Some("deepseek");
            if is_deepseek_selected && is_dark {
                inner.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{ draw_bg: { color: (dark_selected) } });
            } else if is_deepseek_selected {
                inner.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{ draw_bg: { color: (light_selected) } });
            } else if is_dark {
                inner.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{ draw_bg: { color: (dark_normal) } });
            } else {
                inner.view.view(ids!(list_container.deepseek_item)).apply_over(cx, live!{ draw_bg: { color: (light_normal) } });
            }

            // Alibaba item
            let is_alibaba_selected = selected == Some("alibaba_cloud");
            if is_alibaba_selected && is_dark {
                inner.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{ draw_bg: { color: (dark_selected) } });
            } else if is_alibaba_selected {
                inner.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{ draw_bg: { color: (light_selected) } });
            } else if is_dark {
                inner.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{ draw_bg: { color: (dark_normal) } });
            } else {
                inner.view.view(ids!(list_container.alibaba_item)).apply_over(cx, live!{ draw_bg: { color: (light_normal) } });
            }

            // Provider labels - update text colors
            if is_dark {
                inner.view.label(ids!(list_container.openai_item.openai_label)).apply_over(cx, live!{ draw_text: { color: (dark_text) } });
                inner.view.label(ids!(list_container.deepseek_item.deepseek_label)).apply_over(cx, live!{ draw_text: { color: (dark_text) } });
                inner.view.label(ids!(list_container.alibaba_item.alibaba_label)).apply_over(cx, live!{ draw_text: { color: (dark_text) } });
            } else {
                inner.view.label(ids!(list_container.openai_item.openai_label)).apply_over(cx, live!{ draw_text: { color: (light_text) } });
                inner.view.label(ids!(list_container.deepseek_item.deepseek_label)).apply_over(cx, live!{ draw_text: { color: (light_text) } });
                inner.view.label(ids!(list_container.alibaba_item.alibaba_label)).apply_over(cx, live!{ draw_text: { color: (light_text) } });
            }

            // Add button
            inner.view.button(ids!(add_button)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }
}

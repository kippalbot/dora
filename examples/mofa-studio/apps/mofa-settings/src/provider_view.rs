//! Provider View - Right panel for provider configuration

use makepad_widgets::*;
use crate::data::{Provider, ProviderId, ProviderConnectionStatus};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::FONT_FAMILY;
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_BOLD;
    use mofa_widgets::theme::FONT_SEMIBOLD;

    // Custom text input style
    SettingsTextInput = <TextInput> {
        width: Fill, height: 44
        padding: {left: 12, right: 12, top: 10, bottom: 10}

        draw_bg: {
            instance radius: 6.0
            instance border_width: 1.0
            instance border_color: #cbd5e1

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(
                    self.border_width,
                    self.border_width,
                    self.rect_size.x - self.border_width * 2.0,
                    self.rect_size.y - self.border_width * 2.0,
                    max(1.0, self.radius - self.border_width)
                );
                sdf.fill(#e2e8f0);
                sdf.stroke(self.border_color, self.border_width);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_REGULAR>{ font_size: 11.0 }
            color: #1f2937

            fn get_color(self) -> vec4 {
                return #1f2937;
            }
        }

        // Empty/placeholder text styling
        draw_label: {
            text_style: <FONT_REGULAR>{ font_size: 11.0 }
            color: #6b7280

            fn get_color(self) -> vec4 {
                return #6b7280;
            }
        }

        draw_selection: {
            color: #bfdbfe
        }

        draw_cursor: {
            color: #3b82f6
        }
    }

    // Save button style
    SaveButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#3b82f6, #2563ff, self.hover),
                    #1d4fff,
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: #ffffff

            fn get_color(self) -> vec4 {
                return #ffffff;
            }
        }

        text: "Save"
    }

    // Remove button style (for custom providers)
    RemoveButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix(#fef2f2, #fee2e2, self.hover),
                    #fecaca,
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                sdf.stroke(#ef4444, 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: #ef4444

            fn get_color(self) -> vec4 {
                return #ef4444;
            }
        }

        text: "Remove"
    }

    // Sync button - active state
    SyncButton = <Button> {
        width: Fit, height: 32
        padding: {left: 12, right: 12, top: 6, bottom: 6}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance disabled: 0.0
            instance radius: 4.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let active_color = mix(
                    mix(#10b981, #059669, self.hover),
                    #047857,
                    self.pressed
                );
                let disabled_color = #d1d5db;
                let color = mix(active_color, disabled_color, self.disabled);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            color: #ffffff

            fn get_color(self) -> vec4 {
                return #ffffff;
            }
        }

        text: "Sync Models"
    }

    // Model radio item
    ModelRadioItem = <View> {
        width: Fill, height: Fit
        flow: Right
        spacing: 8
        padding: {top: 6, bottom: 6}
        align: {y: 0.5}
        cursor: Hand

        radio_circle = <View> {
            width: 16, height: 16
            show_bg: true
            draw_bg: {
                instance selected: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let center = self.rect_size * 0.5;
                    let radius = min(center.x, center.y) - 1.0;

                    // Outer circle
                    sdf.circle(center.x, center.y, radius);
                    sdf.stroke(mix(#9ca3af, #3b82f6, self.selected), 1.5);

                    // Inner dot when selected
                    if self.selected > 0.5 {
                        sdf.circle(center.x, center.y, radius * 0.5);
                        sdf.fill(#3b82f6);
                    }

                    return sdf.result;
                }
            }
        }

        model_name = <Label> {
            text: "model-name"
            draw_text: {
                color: #374151
                text_style: <FONT_REGULAR>{ font_size: 11.0 }
            }
        }
    }

    // Provider view - right panel showing provider details
    pub ProviderView = {{ProviderView}} {
        width: Fill, height: Fill
        flow: Down
        padding: 30
        spacing: 24

        show_bg: true
        draw_bg: {
            color: #f8fafc
        }

        // Header
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            spacing: 12

            provider_name = <Label> {
                text: "Select a Provider"
                draw_text: {
                    color: #1e293b
                    text_style: <FONT_BOLD>{ font_size: 20.0 }
                }
            }

            // Status indicator
            status_label = <Label> {
                text: ""
                draw_text: {
                    color: #6b7280
                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                }
            }
        }

        // Content area
        content = <View> {
            width: Fill, height: Fill
            flow: Down
            spacing: 20

            // API Host field
            host_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                <Label> {
                    text: "API Host"
                    draw_text: {
                        color: #374151
                        text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                    }
                }

                api_host_input = <SettingsTextInput> {
                    empty_text: "https://api.example.com/v1"
                }

                <Label> {
                    text: "The base URL for API requests"
                    draw_text: {
                        color: #6b7280
                        text_style: <FONT_REGULAR>{ font_size: 10.0 }
                    }
                }
            }

            // API Key field
            key_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                <Label> {
                    text: "API Key"
                    draw_text: {
                        color: #374151
                        text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                    }
                }

                api_key_input = <SettingsTextInput> {
                    empty_text: "sk-..."
                    is_password: true
                }

                <Label> {
                    text: "Your API key (stored locally)"
                    draw_text: {
                        color: #6b7280
                        text_style: <FONT_REGULAR>{ font_size: 10.0 }
                    }
                }
            }

            // Available models with sync button
            models_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8

                // Header row with label and sync button
                models_header = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 12

                    <Label> {
                        text: "Available Models"
                        draw_text: {
                            color: #374151
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                        }
                    }

                    <View> { width: Fill, height: 1 }

                    sync_button = <SyncButton> {}
                }

                // Sync status message
                sync_status = <Label> {
                    width: Fill
                    text: ""
                    draw_text: {
                        color: #6b7280
                        text_style: <FONT_REGULAR>{ font_size: 10.0 }
                    }
                }

                // Models list container (scrollable)
                models_list_container = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 0
                    padding: {top: 4}

                    // Placeholder when no models
                    no_models_label = <Label> {
                        width: Fill
                        text: "Click 'Sync Models' to fetch available models"
                        draw_text: {
                            color: #9ca3af
                            text_style: <FONT_REGULAR>{ font_size: 11.0 }
                        }
                    }

                    // Dynamic model list using PortalList
                    models_list = <PortalList> {
                        width: Fill, height: Fit
                        flow: Down

                        model_item = <ModelRadioItem> {}
                    }
                }
            }

            // Spacer
            <View> { width: Fill, height: Fill }

            // Action buttons
            actions = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12

                save_button = <SaveButton> {}

                remove_button = <RemoveButton> {
                    visible: false
                }
            }
        }

        // Empty state - shown when no provider selected
        empty_state = <View> {
            width: Fill, height: Fill
            align: {x: 0.5, y: 0.5}

            <View> {
                width: Fit, height: Fit
                flow: Down
                align: {x: 0.5, y: 0.5}
                spacing: 8

                <Label> {
                    text: "Select a Provider"
                    draw_text: {
                        color: #94a3b8
                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                    }
                }

                <Label> {
                    text: "Choose a provider from the list to configure"
                    draw_text: {
                        color: #cbd5e1
                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ProviderView {
    #[deref]
    view: View,

    #[rust]
    current_provider_id: Option<ProviderId>,

    #[rust]
    show_content: bool,

    #[rust]
    available_models: Vec<String>,

    #[rust]
    selected_model: Option<String>,

    #[rust]
    is_syncing: bool,

    #[rust]
    selected_model_index: Option<usize>,
}

impl Widget for ProviderView {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle model item clicks
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Check for clicks on model items in the list
        let models_list = self.view.portal_list(ids!(models_list));
        for (index, item) in models_list.items_with_actions(actions) {
            // Check if this item was clicked by looking at the finger_up action on the view
            if item.as_view().finger_up(actions).is_some() {
                // Update selected model
                if index < self.available_models.len() {
                    self.selected_model_index = Some(index);
                    self.selected_model = Some(self.available_models[index].clone());
                    self.view.redraw(cx);
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Draw the PortalList with model items
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                // Set the range based on number of models
                list.set_item_range(cx, 0, self.available_models.len());

                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id < self.available_models.len() {
                        let item = list.item(cx, item_id, live_id!(model_item));
                        let model_name = &self.available_models[item_id];
                        let is_selected = self.selected_model_index == Some(item_id);

                        // Set the model name label
                        item.label(ids!(model_name)).set_text(cx, model_name);

                        // Set the radio button selected state
                        let selected_val = if is_selected { 1.0 } else { 0.0 };
                        item.view(ids!(radio_circle)).apply_over(cx, live!{
                            draw_bg: { selected: (selected_val) }
                        });

                        item.draw_all(cx, scope);
                    }
                }
            }
        }
        DrawStep::done()
    }
}

impl ProviderViewRef {
    /// Load a provider's details into the view
    pub fn load_provider(&self, cx: &mut Cx, provider: &Provider) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.current_provider_id = Some(provider.id.clone());
            inner.show_content = true;
            inner.available_models = provider.models.clone();
            inner.selected_model = provider.models.first().cloned();
            inner.selected_model_index = if provider.models.is_empty() { None } else { Some(0) };

            // Update header
            inner.view.label(ids!(provider_name)).set_text(cx, &provider.name);

            // Show content, hide empty state
            inner.view.view(ids!(content)).set_visible(cx, true);
            inner.view.view(ids!(empty_state)).set_visible(cx, false);

            // Set input values
            inner.view.text_input(ids!(api_host_input)).set_text(cx, &provider.url);
            inner.view.text_input(ids!(api_key_input)).set_text(cx,
                provider.api_key.as_deref().unwrap_or("")
            );

            // Update sync button state based on API key
            let has_api_key = provider.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);
            inner.view.button(ids!(sync_button)).apply_over(cx, live!{
                draw_bg: { disabled: (if has_api_key { 0.0 } else { 1.0 }) }
            });

            // Update sync status text
            if !has_api_key {
                inner.view.label(ids!(sync_status)).set_text(cx, "Enter API key to sync models");
            } else {
                inner.view.label(ids!(sync_status)).set_text(cx, "");
            }

            // Show models if available
            Self::display_models_internal(&mut inner, cx, &provider.models);

            // Show/hide remove button for custom providers
            inner.view.button(ids!(remove_button)).set_visible(cx, provider.is_custom);

            // Update status
            let status_text = match &provider.connection_status {
                ProviderConnectionStatus::Disconnected => "",
                ProviderConnectionStatus::Connecting => "Connecting...",
                ProviderConnectionStatus::Connected => "Connected",
                ProviderConnectionStatus::Error(_) => "Error",
            };
            inner.view.label(ids!(status_label)).set_text(cx, status_text);

            inner.view.redraw(cx);
        }
    }

    /// Internal helper to display models
    fn display_models_internal(inner: &mut std::cell::RefMut<ProviderView>, cx: &mut Cx, models: &[String]) {
        // Show/hide no models label based on whether we have models
        inner.view.label(ids!(no_models_label)).set_visible(cx, models.is_empty());

        // Store models in the struct - the PortalList will render them in draw_walk
        inner.available_models = models.to_vec();

        // Set first model as selected if any exist and no selection yet
        if !models.is_empty() && inner.selected_model_index.is_none() {
            inner.selected_model_index = Some(0);
            inner.selected_model = Some(models[0].clone());
        }

        // Trigger redraw to update the PortalList
        inner.view.redraw(cx);
    }

    /// Display fetched models
    pub fn display_models(&self, cx: &mut Cx, models: Vec<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.is_syncing = false;

            // Reset selection when new models are loaded
            inner.selected_model_index = if models.is_empty() { None } else { Some(0) };
            inner.selected_model = models.first().cloned();

            Self::display_models_internal(&mut inner, cx, &models);

            if models.is_empty() {
                inner.view.label(ids!(sync_status)).set_text(cx, "No models found");
            } else {
                inner.view.label(ids!(sync_status)).set_text(cx, &format!("Found {} models", models.len()));
            }

            inner.view.redraw(cx);
        }
    }

    /// Clear the view (show empty state)
    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.current_provider_id = None;
            inner.show_content = false;
            inner.available_models.clear();
            inner.selected_model = None;
            inner.selected_model_index = None;

            // Hide content, show empty state
            inner.view.view(ids!(content)).set_visible(cx, false);
            inner.view.view(ids!(empty_state)).set_visible(cx, true);

            inner.view.label(ids!(provider_name)).set_text(cx, "Select a Provider");
            inner.view.label(ids!(status_label)).set_text(cx, "");

            inner.view.redraw(cx);
        }
    }

    /// Get the current provider ID being edited
    pub fn current_provider_id(&self) -> Option<ProviderId> {
        self.borrow().and_then(|inner| inner.current_provider_id.clone())
    }

    /// Get the current form values
    pub fn get_form_values(&self) -> Option<(String, Option<String>)> {
        self.borrow().map(|inner| {
            let api_host = inner.view.text_input(ids!(api_host_input)).text();
            let api_key = {
                let key = inner.view.text_input(ids!(api_key_input)).text();
                if key.is_empty() { None } else { Some(key) }
            };

            (api_host, api_key)
        })
    }
}

use makepad_widgets::*;
use crate::data::{Provider, ProviderId, Preferences};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::widgets::theme::FONT_FAMILY;
    use crate::widgets::theme::FONT_REGULAR;
    use crate::widgets::theme::FONT_BOLD;

    use crate::widgets::providers_panel::ProvidersPanel;
    use crate::widgets::provider_view::ProviderView;
    use crate::widgets::add_provider_modal::AddProviderModal;

    // Divider line
    VerticalDivider = <View> {
        width: 1, height: Fill
        show_bg: true
        draw_bg: {
            color: #e5e7eb
        }
    }

    // Settings screen container
    pub SettingsScreen = {{SettingsScreen}} {
        width: Fill, height: Fill
        flow: Overlay

        // Main content
        content = <View> {
            width: Fill, height: Fill
            flow: Right

            // Left panel - provider list
            providers_panel = <ProvidersPanel> {}

            // Divider
            <VerticalDivider> {}

            // Right panel - provider details
            provider_view = <ProviderView> {}
        }

        // Modal overlay (hidden by default)
        add_provider_modal = <AddProviderModal> {}
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct SettingsScreen {
    #[deref]
    view: View,

    #[rust]
    preferences: Option<Preferences>,

    #[rust]
    selected_provider_id: Option<ProviderId>,
}

impl Widget for SettingsScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Handle provider selection from view items
        if self.view.view(ids!(providers_panel.list_container.openai_item)).finger_up(actions).is_some() {
            let id = ProviderId::from("openai");
            if self.selected_provider_id.as_ref() != Some(&id) {
                self.selected_provider_id = Some(id.clone());
                self.load_provider_to_view(cx, &id);
            }
        }
        if self.view.view(ids!(providers_panel.list_container.deepseek_item)).finger_up(actions).is_some() {
            let id = ProviderId::from("deepseek");
            if self.selected_provider_id.as_ref() != Some(&id) {
                self.selected_provider_id = Some(id.clone());
                self.load_provider_to_view(cx, &id);
            }
        }
        if self.view.view(ids!(providers_panel.list_container.alibaba_item)).finger_up(actions).is_some() {
            let id = ProviderId::from("alibaba_cloud");
            if self.selected_provider_id.as_ref() != Some(&id) {
                self.selected_provider_id = Some(id.clone());
                self.load_provider_to_view(cx, &id);
            }
        }

        // Handle add provider button
        if self.view.button(ids!(providers_panel.add_button)).clicked(actions) {
            self.view.view(ids!(add_provider_modal)).set_visible(cx, true);
            self.view.redraw(cx);
        }

        // Handle modal cancel button
        if self.view.button(ids!(add_provider_modal.cancel_button)).clicked(actions) {
            self.view.view(ids!(add_provider_modal)).set_visible(cx, false);
            self.view.redraw(cx);
        }

        // Handle modal add button
        if self.view.button(ids!(add_provider_modal.add_button)).clicked(actions) {
            // Get the values from the modal inputs
            let name = self.view.text_input(ids!(add_provider_modal.name_input)).text();
            let host = self.view.text_input(ids!(add_provider_modal.host_input)).text();
            let key = self.view.text_input(ids!(add_provider_modal.key_input)).text();

            if !name.is_empty() && !host.is_empty() {
                let id = ProviderId::from(
                    name.to_lowercase().replace(" ", "_").replace("-", "_")
                );

                let provider = Provider {
                    id: id.clone(),
                    name,
                    url: host,
                    api_key: if key.is_empty() { None } else { Some(key) },
                    provider_type: crate::data::ProviderType::Custom,
                    enabled: true,
                    models: vec![],
                    is_custom: true,
                    connection_status: crate::data::ProviderConnectionStatus::Disconnected,
                };

                self.add_custom_provider(cx, provider);
                self.view.view(ids!(add_provider_modal)).set_visible(cx, false);
                self.view.redraw(cx);
            }
        }

        // Handle save button
        if self.view.button(ids!(provider_view.save_button)).clicked(actions) {
            self.save_current_provider(cx);
        }

        // Handle remove button
        if self.view.button(ids!(provider_view.remove_button)).clicked(actions) {
            self.remove_current_provider(cx);
        }

        // Handle sync button
        if self.view.button(ids!(provider_view.sync_button)).clicked(actions) {
            self.sync_models(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SettingsScreen {
    fn load_provider_to_view(&mut self, cx: &mut Cx, provider_id: &ProviderId) {
        if let Some(prefs) = &self.preferences {
            if let Some(provider) = prefs.providers.iter().find(|p| &p.id == provider_id) {
                // Update the provider view with the selected provider's details
                self.view.label(ids!(provider_view.provider_name)).set_text(cx, &provider.name);
                self.view.text_input(ids!(provider_view.api_host_input)).set_text(cx, &provider.url);
                self.view.text_input(ids!(provider_view.api_key_input)).set_text(cx,
                    provider.api_key.as_deref().unwrap_or("")
                );

                // Display saved models or clear the list
                let saved_models = provider.models.clone();
                let has_api_key = provider.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);

                let model_item_ids = [
                    ids!(provider_view.model_item_1),
                    ids!(provider_view.model_item_2),
                    ids!(provider_view.model_item_3),
                    ids!(provider_view.model_item_4),
                    ids!(provider_view.model_item_5),
                    ids!(provider_view.model_item_6),
                    ids!(provider_view.model_item_7),
                    ids!(provider_view.model_item_8),
                    ids!(provider_view.model_item_9),
                    ids!(provider_view.model_item_10),
                ];
                let model_label_ids = [
                    ids!(provider_view.model_item_1.model_name),
                    ids!(provider_view.model_item_2.model_name),
                    ids!(provider_view.model_item_3.model_name),
                    ids!(provider_view.model_item_4.model_name),
                    ids!(provider_view.model_item_5.model_name),
                    ids!(provider_view.model_item_6.model_name),
                    ids!(provider_view.model_item_7.model_name),
                    ids!(provider_view.model_item_8.model_name),
                    ids!(provider_view.model_item_9.model_name),
                    ids!(provider_view.model_item_10.model_name),
                ];

                // Show saved models or clear
                for (i, (item_id, label_id)) in model_item_ids.iter().zip(model_label_ids.iter()).enumerate() {
                    if i < saved_models.len() {
                        self.view.view(item_id.clone()).set_visible(cx, true);
                        self.view.label(label_id.clone()).set_text(cx, &saved_models[i]);
                    } else {
                        self.view.view(item_id.clone()).set_visible(cx, false);
                    }
                }

                // Update no models label and sync status
                self.view.label(ids!(provider_view.no_models_label)).set_visible(cx, saved_models.is_empty());
                if !saved_models.is_empty() {
                    self.view.label(ids!(provider_view.sync_status)).set_text(cx, &format!("{} models", saved_models.len()));
                } else if has_api_key {
                    self.view.label(ids!(provider_view.sync_status)).set_text(cx, "");
                } else {
                    self.view.label(ids!(provider_view.sync_status)).set_text(cx, "Enter API key to sync models");
                }

                // Update sync button state based on API key
                if has_api_key {
                    self.view.button(ids!(provider_view.sync_button)).apply_over(cx, live!{
                        draw_bg: { disabled: 0.0 }
                    });
                } else {
                    self.view.button(ids!(provider_view.sync_button)).apply_over(cx, live!{
                        draw_bg: { disabled: 1.0 }
                    });
                }

                // Show content, hide empty state
                self.view.view(ids!(provider_view.content)).set_visible(cx, true);
                self.view.view(ids!(provider_view.empty_state)).set_visible(cx, false);
                self.view.button(ids!(provider_view.remove_button)).set_visible(cx, provider.is_custom);

                self.view.redraw(cx);
            }
        }
    }

    fn save_current_provider(&mut self, _cx: &mut Cx) {
        if let Some(provider_id) = &self.selected_provider_id {
            let api_host = self.view.text_input(ids!(provider_view.api_host_input)).text();
            let api_key = {
                let key = self.view.text_input(ids!(provider_view.api_key_input)).text();
                if key.is_empty() { None } else { Some(key) }
            };

            if let Some(prefs) = &mut self.preferences {
                if let Some(provider) = prefs.providers.iter_mut().find(|p| &p.id == provider_id) {
                    provider.url = api_host;
                    provider.api_key = api_key;

                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save preferences: {}", e);
                    } else {
                        ::log::info!("Saved provider settings for {}", provider_id.as_str());
                    }
                }
            }
        }
    }

    fn remove_current_provider(&mut self, cx: &mut Cx) {
        if let Some(provider_id) = self.selected_provider_id.clone() {
            if let Some(prefs) = &mut self.preferences {
                // Only remove custom providers
                if let Some(idx) = prefs.providers.iter().position(|p| p.id == provider_id && p.is_custom) {
                    prefs.providers.remove(idx);

                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save preferences: {}", e);
                    }

                    // Clear the view and selection
                    self.selected_provider_id = None;
                    self.view.view(ids!(provider_view.content)).set_visible(cx, false);
                    self.view.view(ids!(provider_view.empty_state)).set_visible(cx, true);
                    self.view.label(ids!(provider_view.provider_name)).set_text(cx, "Select a Provider");
                    self.view.redraw(cx);
                }
            }
        }
    }

    fn add_custom_provider(&mut self, cx: &mut Cx, provider: Provider) {
        if let Some(prefs) = &mut self.preferences {
            let id = provider.id.clone();
            prefs.providers.push(provider);

            if let Err(e) = prefs.save() {
                eprintln!("Failed to save preferences: {}", e);
            }

            // Select the new provider
            self.selected_provider_id = Some(id.clone());
            self.load_provider_to_view(cx, &id);
        }
    }

    fn sync_models(&mut self, cx: &mut Cx) {
        // Check if API key is provided
        let api_key = self.view.text_input(ids!(provider_view.api_key_input)).text();
        if api_key.is_empty() {
            self.view.label(ids!(provider_view.sync_status)).set_text(cx, "Enter API key to sync models");
            self.view.redraw(cx);
            return;
        }

        // Get provider info
        let api_host = self.view.text_input(ids!(provider_view.api_host_input)).text();

        // Set syncing state
        self.view.label(ids!(provider_view.sync_status)).set_text(cx, "Syncing models...");
        self.view.button(ids!(provider_view.sync_button)).apply_over(cx, live!{
            draw_bg: { disabled: 1.0 }
        });
        self.view.redraw(cx);

        // For now, simulate fetching models based on provider
        // In a real implementation, this would make an async HTTP request
        let models = if let Some(provider_id) = &self.selected_provider_id {
            match provider_id.as_str() {
                "openai" => vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-4".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
                "deepseek" => vec![
                    "deepseek-chat".to_string(),
                    "deepseek-coder".to_string(),
                ],
                "alibaba_cloud" => vec![
                    "qwen-turbo".to_string(),
                    "qwen-plus".to_string(),
                    "qwen-max".to_string(),
                ],
                _ => vec!["custom-model".to_string()],
            }
        } else {
            vec![]
        };

        // Persist models to provider in preferences
        if let Some(provider_id) = &self.selected_provider_id {
            if let Some(prefs) = &mut self.preferences {
                if let Some(provider) = prefs.providers.iter_mut().find(|p| &p.id == provider_id) {
                    provider.models = models.clone();
                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save models: {}", e);
                    }
                }
            }
        }

        // Update the models display
        self.display_models(cx, models);
    }

    fn display_models(&mut self, cx: &mut Cx, models: Vec<String>) {
        // Hide no models label if we have models
        self.view.label(ids!(provider_view.no_models_label)).set_visible(cx, models.is_empty());

        // Update model items
        let model_item_ids = [
            ids!(provider_view.model_item_1),
            ids!(provider_view.model_item_2),
            ids!(provider_view.model_item_3),
            ids!(provider_view.model_item_4),
            ids!(provider_view.model_item_5),
            ids!(provider_view.model_item_6),
            ids!(provider_view.model_item_7),
            ids!(provider_view.model_item_8),
            ids!(provider_view.model_item_9),
            ids!(provider_view.model_item_10),
        ];

        // Label IDs for each model item
        let model_label_ids = [
            ids!(provider_view.model_item_1.model_name),
            ids!(provider_view.model_item_2.model_name),
            ids!(provider_view.model_item_3.model_name),
            ids!(provider_view.model_item_4.model_name),
            ids!(provider_view.model_item_5.model_name),
            ids!(provider_view.model_item_6.model_name),
            ids!(provider_view.model_item_7.model_name),
            ids!(provider_view.model_item_8.model_name),
            ids!(provider_view.model_item_9.model_name),
            ids!(provider_view.model_item_10.model_name),
        ];

        for (i, (item_id, label_id)) in model_item_ids.iter().zip(model_label_ids.iter()).enumerate() {
            if i < models.len() {
                self.view.view(item_id.clone()).set_visible(cx, true);
                self.view.label(label_id.clone()).set_text(cx, &models[i]);
            } else {
                self.view.view(item_id.clone()).set_visible(cx, false);
            }
        }

        // Update sync status
        if models.is_empty() {
            self.view.label(ids!(provider_view.sync_status)).set_text(cx, "No models found");
        } else {
            self.view.label(ids!(provider_view.sync_status)).set_text(cx, &format!("Found {} models", models.len()));
        }

        // Re-enable sync button
        self.view.button(ids!(provider_view.sync_button)).apply_over(cx, live!{
            draw_bg: { disabled: 0.0 }
        });

        self.view.redraw(cx);
    }
}

impl SettingsScreenRef {
    /// Initialize the settings screen with preferences
    pub fn init(&self, cx: &mut Cx, preferences: Preferences) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.preferences = Some(preferences);
            inner.view.redraw(cx);
        }
    }

    /// Reload preferences from disk
    pub fn reload_preferences(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.preferences = Some(Preferences::load());
            inner.view.redraw(cx);
        }
    }
}

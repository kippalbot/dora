# MoFA Studio App Development Guide

> How to create apps for MoFA Studio using the MofaApp plugin system

---

## Overview

MoFA Studio uses a trait-based plugin system for apps. Each app:
1. Implements the `MofaApp` trait
2. Provides widgets via Makepad's `live_design!` macro
3. Registers with the shell at compile time

**Key Constraint**: Makepad requires compile-time widget type resolution. Apps cannot be loaded dynamically at runtime.

---

## Quick Start

### 1. Create App Crate

```bash
cd examples/mofa-studio/apps
cargo new mofa-myapp --lib
```

### 2. Configure Cargo.toml

```toml
[package]
name = "mofa-myapp"
version = "0.1.0"
edition = "2021"

[dependencies]
makepad-widgets = { workspace = true }
mofa-widgets = { path = "../../mofa-widgets" }
```

### 3. Implement MofaApp Trait

```rust
// src/lib.rs
pub mod screen;

use makepad_widgets::Cx;
use mofa_widgets::{MofaApp, AppInfo};

/// App descriptor - required for plugin system
pub struct MoFaMyApp;

impl MofaApp for MoFaMyApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "My App",           // Display name in UI
            id: "mofa-myapp",         // Unique identifier
            description: "My custom MoFA app",
        }
    }

    fn live_design(cx: &mut Cx) {
        screen::live_design(cx);
    }
}

/// Backwards-compatible registration function
pub fn live_design(cx: &mut Cx) {
    MoFaMyApp::live_design(cx);
}
```

### 4. Create Main Screen Widget

```rust
// src/screen.rs
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::widgets::*;

    // Import shared theme (required)
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_MEDIUM;
    use mofa_widgets::theme::DARK_BG;
    use mofa_widgets::theme::TEXT_PRIMARY;

    // Define your screen widget
    pub MyAppScreen = {{MyAppScreen}} {
        width: Fill, height: Fill
        flow: Down
        padding: 20

        show_bg: true
        draw_bg: { color: (DARK_BG) }

        <Label> {
            text: "My App"
            draw_text: {
                text_style: <FONT_MEDIUM> { font_size: 24.0 }
                color: (TEXT_PRIMARY)
            }
        }

        content = <View> {
            width: Fill, height: Fill
            // Your app content here
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[deref]
    view: View,
}

impl Widget for MyAppScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
```

---

## Shell Integration

### 5. Add to Workspace

Edit `examples/mofa-studio/Cargo.toml`:

```toml
[workspace]
members = [
    "mofa-widgets",
    "mofa-studio-shell",
    "apps/mofa-fm",
    "apps/mofa-settings",
    "apps/mofa-myapp",  # Add your app
]
```

### 6. Add Shell Dependency

Edit `mofa-studio-shell/Cargo.toml`:

```toml
[dependencies]
mofa-myapp = { path = "../apps/mofa-myapp" }
```

### 7. Register in Shell

Edit `mofa-studio-shell/src/app.rs`:

```rust
// Add imports
use mofa_myapp::MoFaMyApp;

// In live_design! macro - add widget type import
live_design! {
    use mofa_myapp::screen::MyAppScreen;  // Compile-time requirement
    // ...
}

// In LiveHook::after_new_from_doc - register app info
impl LiveHook for App {
    fn after_new_from_doc(&mut self, _cx: &mut Cx) {
        self.app_registry.register(MoFaFMApp::info());
        self.app_registry.register(MoFaSettingsApp::info());
        self.app_registry.register(MoFaMyApp::info());  // Add this
    }
}

// In LiveRegister::live_register - register widgets
impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        // ...
        <MoFaMyApp as MofaApp>::live_design(cx);  // Add this
    }
}
```

---

## Optional Features

### Timer Management

If your app uses interval timers (animations, polling), add timer control methods to your screen's Ref type:

```rust
// src/screen.rs
use makepad_widgets::*;

#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[deref]
    view: View,
    #[rust]
    update_timer: Timer,
}

impl MyAppScreen {
    fn start_animation(&mut self, cx: &mut Cx) {
        self.update_timer = cx.start_interval(0.05);  // 50ms interval
    }
}

// Add timer control methods to the auto-generated Ref type
impl MyAppScreenRef {
    /// Stop timers - call this when hiding the widget
    pub fn stop_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            cx.stop_timer(inner.update_timer);
        }
    }

    /// Start timers - call this when showing the widget
    pub fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_timer = cx.start_interval(0.05);
        }
    }
}
```

**Shell Integration:**

The shell must call these methods when switching apps:

```rust
// In shell's app switch logic
fn show_my_app(&mut self, cx: &mut Cx) {
    self.ui.my_app_screen(ids!(...my_app)).start_timers(cx);
    self.ui.view(ids!(...my_app)).set_visible(cx, true);
}

fn hide_my_app(&mut self, cx: &mut Cx) {
    self.ui.my_app_screen(ids!(...my_app)).stop_timers(cx);
    self.ui.view(ids!(...my_app)).set_visible(cx, false);
}
```

**Reference**: See `mofa-fm/src/screen.rs` for a complete example with audio meter timers.

### Using Shared Widgets

Import widgets from `mofa-widgets`:

```rust
live_design! {
    use mofa_widgets::waveform_view::WaveformView;
    use mofa_widgets::led_gauge::LedGauge;
    use mofa_widgets::participant_panel::ParticipantPanel;

    pub MyAppScreen = {{MyAppScreen}} {
        // Use shared widgets
        waveform = <WaveformView> { }
        gauge = <LedGauge> { }
    }
}
```

---

## Project Structure

```
apps/mofa-myapp/
├── Cargo.toml
└── src/
    ├── lib.rs          # MofaApp impl, exports
    ├── screen.rs       # Main screen widget
    └── components.rs   # Optional: sub-components
```

### Recommended lib.rs Pattern

```rust
//! MoFA MyApp - Description of your app

pub mod screen;
// pub mod components;  // Optional: additional modules

// Re-export main widget for shell's live_design! macro
pub use screen::MyAppScreen;

use makepad_widgets::Cx;
use mofa_widgets::{MofaApp, AppInfo};

pub struct MoFaMyApp;

impl MofaApp for MoFaMyApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "My App",
            id: "mofa-myapp",
            description: "Description here",
        }
    }

    fn live_design(cx: &mut Cx) {
        screen::live_design(cx);
        // components::live_design(cx);  // If you have sub-components
    }
}

/// Backwards-compatible registration function
pub fn live_design(cx: &mut Cx) {
    MoFaMyApp::live_design(cx);
}
```

---

## Theme Integration

Always use the shared theme from `mofa_widgets::theme`:

```rust
live_design! {
    // Fonts
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_MEDIUM;
    use mofa_widgets::theme::FONT_SEMIBOLD;
    use mofa_widgets::theme::FONT_BOLD;

    // Colors (Light mode)
    use mofa_widgets::theme::DARK_BG;
    use mofa_widgets::theme::PANEL_BG;
    use mofa_widgets::theme::ACCENT_BLUE;
    use mofa_widgets::theme::TEXT_PRIMARY;
    use mofa_widgets::theme::TEXT_SECONDARY;

    // Colors (Dark mode variants)
    use mofa_widgets::theme::DARK_BG_DARK;
    use mofa_widgets::theme::PANEL_BG_DARK;
    use mofa_widgets::theme::TEXT_PRIMARY_DARK;
    use mofa_widgets::theme::TEXT_SECONDARY_DARK;
}
```

**Do NOT** define fonts or colors locally in your app.

---

## Dark Mode Support

MoFA Studio supports runtime dark/light mode switching. Apps should implement dark mode to maintain visual consistency.

### Adding Dark Mode to Widgets

Use `instance dark_mode` with `mix()` in shaders:

```rust
live_design! {
    use mofa_widgets::theme::*;

    pub MyWidget = {{MyWidget}} <RoundedView> {
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0  // 0.0=light, 1.0=dark

            fn get_color(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }
        }

        label = <Label> {
            draw_text: {
                instance dark_mode: 0.0
                fn get_color(self) -> vec4 {
                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                }
            }
        }
    }
}
```

### Adding update_dark_mode Method

Implement on your screen's Ref type so the shell can propagate theme changes:

```rust
impl MyAppScreenRef {
    /// Update dark mode for this screen
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Update panel backgrounds
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Update labels
            inner.view.label(ids!(header.title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }
}
```

### Shell Integration

The shell calls `update_dark_mode` on your screen when the theme toggles:

```rust
// In shell's apply_dark_mode_screens
self.ui.my_app_screen(ids!(my_app_page)).update_dark_mode(cx, dark_mode);
```

### Important: vec4 in apply_over

**Hex colors do NOT work in `apply_over` at runtime!** Use `vec4()` format:

```rust
// ❌ FAILS - hex colors don't work in apply_over
self.view.apply_over(cx, live!{ draw_bg: { color: #1f293b } });

// ✅ WORKS - vec4 format
self.view.apply_over(cx, live!{ draw_bg: { color: (vec4(0.12, 0.16, 0.23, 1.0)) } });
```

### Color Reference (vec4 format)

| Purpose | Light Mode | Dark Mode |
|---------|------------|-----------|
| Panel background | `vec4(1.0, 1.0, 1.0, 1.0)` | `vec4(0.12, 0.16, 0.23, 1.0)` |
| Text primary | `vec4(0.12, 0.16, 0.22, 1.0)` | `vec4(0.95, 0.96, 0.98, 1.0)` |
| Hover background | `vec4(0.95, 0.96, 0.98, 1.0)` | `vec4(0.2, 0.25, 0.33, 1.0)` |

---

## Checklist

Before submitting your app:

**MofaApp Trait:**
- [ ] Implements `MofaApp` trait with valid `info()` and `live_design()`
- [ ] Exports main screen widget for shell's `live_design!` macro

**Theme & Dark Mode:**
- [ ] Uses shared theme (no local font/color definitions)
- [ ] Widgets have `instance dark_mode: 0.0` for themeable elements
- [ ] Implements `update_dark_mode()` on screen Ref type
- [ ] Uses `vec4()` for runtime color changes in `apply_over()`

**Timer Management (if applicable):**
- [ ] Implements `stop_timers()` and `start_timers()` on Ref type
- [ ] Shell calls timer methods when hiding/showing app

**Integration:**
- [ ] Added to workspace `Cargo.toml`
- [ ] Added as dependency in `mofa-studio-shell/Cargo.toml`
- [ ] Registered in shell's `LiveHook::after_new_from_doc`
- [ ] Registered in shell's `LiveRegister::live_register`
- [ ] Widget type imported in shell's `live_design!` macro
- [ ] Shell calls `update_dark_mode()` on theme toggle
- [ ] `cargo build` passes with no errors

---

## Reference Apps

| App | Description | Features |
|-----|-------------|----------|
| `mofa-fm` | Audio streaming | Timer management, shader animations |
| `mofa-settings` | Provider config | Modal dialogs, form inputs, state management |

---

## Troubleshooting

### "no function named `live_design_with`"

Your widget type isn't properly imported in the shell's `live_design!` macro:
```rust
live_design! {
    use mofa_myapp::screen::MyAppScreen;  // Must be here
}
```

### "trait bound `MoFaMyApp: MofaApp` is not satisfied"

Check your imports:
```rust
use mofa_widgets::{MofaApp, AppInfo};  // Both needed
```

### Timer keeps running when app is hidden

Implement timer control and ensure shell calls `stop_timers()` on visibility change.

### Fonts/colors don't match other apps

Use `mofa_widgets::theme::*` instead of defining locally.

---

*Last Updated: 2026-01-04*

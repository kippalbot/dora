# MoFA Studio App Development Guide

> How to create apps for MoFA Studio using the MofaApp plugin system

---

## At a Glance

**Time to Create a Basic App**: ~30 minutes
**Shell Integration Required**: Edit 3 files (workspace, shell dependencies, shell app.rs)
**Architecture**: Trait-based plugin system with compile-time registration
**State Management**: Shell coordinator pattern (no Redux-style stores)
**Key Constraint**: Makepad requires compile-time widget type resolution

### The 3-Step Shell Integration

To add your app to the shell, you only need to edit **3 files**:

1. **Workspace** (`Cargo.toml`): Add `"apps/mofa-myapp"` to members
2. **Shell Dependencies** (`mofa-studio-shell/Cargo.toml`): Add `mofa-myapp = { path = "../apps/mofa-myapp" }`
3. **Shell Registration** (`mofa-studio-shell/src/app.rs`):
   - Import: `use mofa_myapp::MoFaMyApp;`
   - Register: `self.app_registry.register(MoFaMyApp::info());`
   - Call: `<MoFaMyApp as MofaApp>::live_design(cx);`

That's it! The shell doesn't need to know about your app's internals.

---

## Overview

MoFA Studio uses a trait-based plugin system for apps. Each app:
1. Implements the `MofaApp` trait
2. Provides widgets via Makepad's `live_design!` macro
3. Registers with the shell at compile time

**Key Constraint**: Makepad requires compile-time widget type resolution. Apps cannot be loaded dynamically at runtime.

**Why This Design?**
- **Black-box apps**: Shell doesn't know about app internals
- **Trait-based registration**: Standardized `MofaApp` interface
- **Metadata-only registry**: Runtime app discovery without dynamic loading
- **StateChangeListener**: Standardized state change notifications

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

#### When to Use Timers

Timers are appropriate for:
- ✅ **Real-time visualizations** (audio waveforms, animations, level meters)
- ✅ **Polling when no event source exists** (system stats, external APIs)
- ✅ **Smooth animations** (requesting next frame via `cx.new_next_frame()`)
- ✅ **High-frequency updates** (20-60 FPS for visual feedback)

Timers are NOT appropriate for:
- ❌ **Event-driven data** (if there's a callback/notification, use that instead)
- ❌ **Expensive operations on UI thread** (use background threads)
- ❌ **Low-frequency updates** that can be triggered by user actions

**Best Practice:** Always implement `stop_timers()` and `start_timers()` methods so the shell can pause your app when hidden. This prevents wasted CPU and battery.

#### Example: Timer for Audio Visualization (Appropriate)

```rust
// GOOD: Timer is correct for continuous audio visualization
#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[deref]
    view: View,
    #[rust]
    audio_timer: Timer,
}

impl MyAppScreen {
    fn start_mic_monitoring(&mut self, cx: &mut Cx) {
        // 50ms = 20 FPS for smooth audio level visualization
        self.audio_timer = cx.start_interval(0.05);
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Poll shared state (cheap operation)
        if self.audio_timer.is_event(event).is_some() {
            self.update_mic_level(cx);
        }
    }
}

// Required: Timer lifecycle control
impl MyAppScreenRef {
    pub fn stop_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            cx.stop_timer(inner.audio_timer);
        }
    }

    pub fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.audio_timer = cx.start_interval(0.05);
        }
    }
}
```

#### Example: Event-Driven Updates (Better than Timer)

```rust
// BETTER: If there's an event source, use it instead of polling
#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[deref]
    view: View,
    #[rust]
    data_receiver: mpsc::Receiver<DataEvent>,
}

impl MyAppScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Check for events from background thread
        while let Ok(event) = self.data_receiver.try_recv() {
            match event {
                DataEvent::Update(value) => self.update_display(cx, value),
            }
        }

        // Only use timer for redraw, not for data fetching
        if self.redraw_timer.is_event(event).is_some() {
            self.view.redraw(cx);
        }
    }
}
```

#### Pattern: Simple Timer Implementation

If your app needs interval timers:

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

**Before creating custom widgets, check `mofa-widgets` for reusable components!**

Available shared widgets:

| Widget | Module | Description | Use Case |
|--------|--------|-------------|----------|
| `WaveformView` | `waveform_view` | 8-band FFT spectrum analyzer | Audio visualization |
| `LedGauge` | `led_gauge` | LED level meter with smoothing | Audio levels, signal strength |
| `ParticipantPanel` | `participant_panel` | Video conference participant | Multi-user displays |
| `LogPanel` | `log_panel` | Scrolling log viewer | Debug output, event logs |
| `MofaHero` | `mofa_hero` | System stats dashboard | Resource monitoring |

Import and use:

```rust
live_design! {
    use mofa_widgets::waveform_view::WaveformView;
    use mofa_widgets::led_gauge::LedGauge;
    use mofa_widgets::participant_panel::ParticipantPanel;
    use mofa_widgets::log_panel::LogPanel;

    pub MyAppScreen = {{MyAppScreen}} {
        // Use shared widgets
        waveform = <WaveformView> { }
        gauge = <LedGauge> { }
        log = <LogPanel> { }
    }
}
```

**Benefits of using shared widgets:**
- Consistent styling and behavior across apps
- Tested and optimized components
- Automatic dark mode support
- Less code to maintain

---

## Architecture: How It All Works

### The MofaApp Trait System

```rust
// In mofa-widgets/src/app_trait.rs
pub trait MofaApp {
    fn info() -> AppInfo where Self: Sized;
    fn live_design(cx: &mut Cx);
}

pub struct AppInfo {
    pub name: &'static str,      // Display name
    pub id: &'static str,        // Unique identifier
    pub description: &'static str,
}
```

**How it works:**
1. Your app implements `MofaApp` trait with metadata and widget registration
2. Shell registers your app's `AppInfo` in `AppRegistry` (for sidebar, etc.)
3. Shell calls your app's `live_design()` to register widgets with Makepad
4. Shell can display app list without knowing app internals

**Why not dynamic loading?**
- Makepad's `live_design!` macro requires concrete types at compile time
- No trait objects or `dyn Widget` in widget trees
- Widget types must be known for code generation
- **Trade-off**: Manual registration (3 lines) for type safety and performance

### State Management: Shell Coordinator Pattern

**Important**: Redux-style stores are **NOT feasible** in Makepad due to widget ownership model.

Instead, use the **shell coordinator pattern**:

```rust
// Shell owns all shared state
impl App {
    #[rust]
    app_state: AppState,  // Single source of truth
}

pub struct AppState {
    pub dark_mode: bool,
    pub providers: Vec<Provider>,
    pub active_dataflow: bool,
}

// Shell broadcasts changes via WidgetRef methods
impl App {
    fn notify_dark_mode_change(&mut self, cx: &mut Cx) {
        let dark_mode = if self.app_state.dark_mode { 1.0 } else { 0.0 };
        self.ui.mo_fa_fmscreen(ids!(fm_page))
            .on_dark_mode_change(cx, dark_mode);
        self.ui.settings_screen(ids!(settings_page))
            .on_dark_mode_change(cx, dark_mode);
    }
}
```

**Your app implements `StateChangeListener` to receive updates:**

```rust
use mofa_widgets::StateChangeListener;

impl StateChangeListener for MyAppScreenRef {
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.redraw(cx);
        }
    }
}
```

**See also**: `STATE_MANAGEMENT_ANALYSIS.md` for complete architecture details.

### Widget Ownership and Access Patterns

**Widget state is owned by the widget:**
```rust
#[derive(Live, LiveHook, Widget)]
pub struct MyAppScreen {
    #[rust]  // Owned by this widget
    counter: usize,
    #[rust]
    timer: Timer,
}
```

**Cross-boundary access via WidgetRef:**
```rust
// Shell accesses app widget
self.ui.my_app_screen(ids!(my_app)).do_something(cx);

// WidgetRef provides borrow_mut() for mutable access
impl MyAppScreenRef {
    pub fn update_counter(&self, cx: &mut Cx, value: usize) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.counter = value;
            inner.view.redraw(cx);
        }
    }
}
```

**Note**: Avoid `Arc<Mutex<T>>` for shared state - conflicts with WidgetRef borrow checking.

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

### Implementing StateChangeListener Trait

The `StateChangeListener` trait standardizes how apps receive state updates from the shell.
Import and implement it on your screen's Ref type:

```rust
use mofa_widgets::StateChangeListener;

impl StateChangeListener for MyAppScreenRef {
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
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

The shell calls `on_dark_mode_change` via the `StateChangeListener` trait when the theme toggles:

```rust
use mofa_widgets::StateChangeListener;

// In shell's apply_dark_mode_screens
self.ui.my_app_screen(ids!(my_app_page)).on_dark_mode_change(cx, dark_mode);
```

This trait-based approach ensures all apps handle state changes consistently.

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

## Reference Apps

Study these apps to understand common patterns:

| App | Location | Key Features | Learn From |
|-----|----------|--------------|------------|
| **MoFA FM** | `apps/mofa-fm/` | Audio streaming, mic visualization, timer management, shader animations | `screen.rs` for timers, `audio.rs` for shared state |
| **Settings** | `apps/mofa-settings/` | Provider configuration, modal dialogs, form inputs, data persistence | `screen.rs` for modals, `data/provider.rs` for data models |
| **MoFA Studio Shell** | `mofa-studio-shell/` | App registry, navigation, tab management, dark mode toggle | `src/app.rs` for shell coordinator pattern |

### Code Examples by Feature

**Timer Management (audio visualization)**
- `apps/mofa-fm/src/screen.rs:1162` - 50ms audio level timer
- `apps/mofa-fm/src/screen.rs:992` - `stop_timers()`/`start_timers()` implementation

**Dark Mode Support**
- `apps/mofa-fm/src/screen.rs:940` - `StateChangeListener` implementation
- `mofa-studio-shell/src/app.rs:1540` - Shell calls `on_dark_mode_change()`

**Modal Dialogs**
- `apps/mofa-settings/src/screen.rs:280` - Add provider modal
- `apps/mofa-settings/src/screen.rs:385` - Modal state management

**Data Persistence**
- `apps/mofa-settings/src/data/provider.rs` - Provider data model
- `apps/mofa-settings/src/data/storage.rs` - File-based storage

**Shared State (Arc<Mutex<T>>)** - Special case only!
- `apps/mofa-fm/src/audio.rs:27` - AudioManager with mic level sharing
- Required for audio callbacks (no Cx access in callback)

---

## Testing Your App

### Build and Run

```bash
# From workspace root
cargo build

# Run the application
cargo run
```

### Verification Checklist

**Before submitting your app:**

**MofaApp Trait:**
- [ ] Implements `MofaApp` trait with valid `info()` and `live_design()`
- [ ] Exports main screen widget for shell's `live_design!` macro
- [ ] App registered in `AppRegistry` (check sidebar)

**Theme & Dark Mode:**
- [ ] Uses shared theme (no local font/color definitions)
- [ ] Widgets have `instance dark_mode: 0.0` for themeable elements
- [ ] Implements `StateChangeListener` trait on screen Ref type
- [ ] Uses `vec4()` for runtime color changes in `apply_over()`
- [ ] Test: Toggle dark mode in Settings, verify your app responds

**Timer Management (if applicable):**
- [ ] Implements `stop_timers()` and `start_timers()` on Ref type
- [ ] Shell calls timer methods when hiding/showing app
- [ ] Test: Switch away from your app, verify CPU usage drops

**Integration:**
- [ ] Added to workspace `Cargo.toml`
- [ ] Added as dependency in `mofa-studio-shell/Cargo.toml`
- [ ] Registered in shell's `LiveHook::after_new_from_doc`
- [ ] Registered in shell's `LiveRegister::live_register`
- [ ] Widget type imported in shell's `live_design!` macro
- [ ] Shell calls `on_dark_mode_change()` via StateChangeListener trait
- [ ] `cargo build` passes with no errors
- [ ] App appears in sidebar and launches correctly

**Code Quality:**
- [ ] No `unwrap()` or `expect()` in production code (use proper error handling)
- [ ] No hardcoded colors or fonts (use theme)
- [ ] Widgets are properly sized (Fill/Fixed)
- [ ] Event handling doesn't block UI thread

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

### "borrow mut failed" or "already borrowed"

You're trying to borrow the same widget twice in one call chain:

```rust
// ❌ FAILS - double borrow
self.ui.my_app(ids!(app)).borrow_mut().and_then(|mut inner| {
    inner.view.apply_over(cx, live!{ ... });  // First borrow
    inner.do_something();  // Second borrow fails!
});

// ✅ WORKS - single borrow
if let Some(mut inner) = self.ui.my_app(ids!(app)).borrow_mut() {
    inner.view.apply_over(cx, live!{ ... });
    inner.do_something();  // Same borrow context
}
```

### App doesn't appear in sidebar

Check that you registered the app info in `LiveHook::after_new_from_doc`:

```rust
fn after_new_from_doc(&mut self, _cx: &mut Cx) {
    self.app_registry.register(MoFaMyApp::info());  // Must be here
}
```

---

## Additional Resources

### Documentation

| Document | Purpose | Location |
|----------|---------|----------|
| **This Guide** | App development quick start | `APP_DEVELOPMENT_GUIDE.md` |
| **State Management Analysis** | Architecture deep dive | `STATE_MANAGEMENT_ANALYSIS.md` |
| **Timer Usage Analysis** | When to use timers | `TIMER_USAGE_ANALYSIS.md` |
| **Roadmap** | Strategic planning | `roadmap-glm.md` |
| **Checklist** | Master task list | `CHECKLIST.md` |

### External Resources

- **Makepad Documentation**: https://makepad.nl/
- **Makepad Examples**: https://github.com/makepad/makepad/tree/master/examples
- **Rust Book**: https://doc.rust-lang.org/book/

### Getting Help

1. **Check existing apps** - `apps/mofa-fm/` and `apps/mofa-settings/` have examples for most patterns
2. **Read the analysis docs** - `STATE_MANAGEMENT_ANALYSIS.md` explains architecture decisions
3. **Search the codebase** - Use `rg` or `grep` to find similar patterns

---

## Next Steps

After creating your app:

1. **Test thoroughly** - Build, run, toggle dark mode, switch apps
2. **Add documentation** - Comment your code, especially complex patterns
3. **Share your app** - Submit PR to MoFA Studio repository
4. **Iterate** - Gather feedback and improve

---

**Happy coding!** 🚀

*Last Updated: 2026-01-04*

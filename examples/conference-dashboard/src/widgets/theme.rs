//! Shared theme definitions for Conference Dashboard widgets

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Font definitions with Chinese and Emoji support (pub for cross-module use)
    pub FONT_FAMILY = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Regular.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_REGULAR = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Regular.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_MEDIUM = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Medium.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_SEMIBOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-SemiBold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_BOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Bold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
}

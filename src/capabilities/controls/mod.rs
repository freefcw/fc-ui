pub mod avatar_group;
pub mod calendar;
pub mod carousel;
pub mod checkbox;
pub mod collapsible;
pub mod color_picker;
pub mod combobox;
pub mod copy_button;
pub mod date_picker;
pub mod drag_drop;
pub mod dropdown;
pub mod form;
pub mod hotkey_input;
pub mod inline_edit;
pub mod input;
pub(crate) mod input_format;
pub mod input_state;
pub mod mention_input;
pub mod number_input;
pub mod otp_input;
pub mod radio;
pub mod range_slider;
pub mod rating;
pub mod search_input;
pub mod select;
pub mod slider;
pub mod sortable_list;
pub mod stepper;
pub mod tag_input;
pub(crate) mod text_util;
pub mod textarea;
pub mod textarea_state;
pub mod time_picker;
pub mod toggle;
pub mod toggle_group;
pub mod validation;

/// Window snap margin shared by the Select and Combobox dropdown overlays.
pub(crate) const DROPDOWN_MARGIN: gpui::Pixels = gpui::px(4.0);

/// Resolve the (background, border, text) colors for an input-family shell
/// from variant, disabled and validation-error state. Shared by Input and
/// Textarea so both stay visually identical.
pub(crate) fn input_shell_colors(
    variant: input::InputVariant,
    disabled: bool,
    has_error: bool,
    tokens: &crate::capabilities::foundation::theme::ThemeTokens,
) -> (gpui::Hsla, gpui::Hsla, gpui::Hsla) {
    if disabled {
        (
            tokens.muted.opacity(0.5),
            tokens.border,
            tokens.muted_foreground,
        )
    } else if has_error {
        match variant {
            input::InputVariant::Default => {
                (tokens.background, tokens.destructive, tokens.foreground)
            }
            input::InputVariant::Outline => {
                (tokens.background, tokens.destructive, tokens.foreground)
            }
            input::InputVariant::Ghost => (
                gpui::transparent_black(),
                tokens.destructive.opacity(0.3),
                tokens.foreground,
            ),
        }
    } else {
        match variant {
            input::InputVariant::Default => (tokens.background, tokens.input, tokens.foreground),
            input::InputVariant::Outline => (tokens.background, tokens.border, tokens.foreground),
            input::InputVariant::Ghost => (
                gpui::transparent_black(),
                tokens.border.opacity(0.3),
                tokens.foreground,
            ),
        }
    }
}

/// Shared shell for anchored dropdown overlays (select, combobox): a deferred
/// overlay snapped to the window with a popover-styled, occluding container,
/// rendered at overlay priority 1 so it floats above page content.
pub(crate) fn anchored_dropdown(
    width: gpui::Pixels,
    theme: &crate::capabilities::foundation::theme::Theme,
    content: impl gpui::IntoElement,
) -> impl gpui::IntoElement {
    use gpui::{prelude::*, *};

    deferred(
        anchored()
            .snap_to_window_with_margin(Edges::all(DROPDOWN_MARGIN))
            .child(
                div().occlude().w(width).child(
                    div()
                        .occlude()
                        .mt(DROPDOWN_MARGIN)
                        .bg(theme.tokens.popover)
                        .border_2()
                        .border_color(theme.tokens.ring)
                        .rounded(theme.tokens.radius_md)
                        .shadow_xl()
                        .overflow_hidden()
                        .child(content),
                ),
            ),
    )
    .with_priority(1)
}

/// Register controls that historically precede image viewer keybindings.
pub(crate) fn init_before_image_viewer(cx: &mut gpui::App) {
    input::init(cx);
    textarea::init(cx);
    otp_input::init(cx);
    select::init_select(cx);
    combobox::init_combobox(cx);
    date_picker::init(cx);
    dropdown::init_dropdown(cx);
}

/// Register controls that historically follow image viewer keybindings.
pub(crate) fn init_after_image_viewer(cx: &mut gpui::App) {
    inline_edit::init(cx);
    mention_input::init_mention_input(cx);
}

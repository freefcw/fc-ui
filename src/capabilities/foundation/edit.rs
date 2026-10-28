//! Shared text-editing visual and interaction constants plus the caret blink
//! task. Living in `foundation` keeps the definitions consistent across the
//! `controls` and `editor` capabilities.

use gpui::{px, Context, Pixels, Task};
use std::time::Duration;

/// Opacity of the primary-color selection highlight quad.
pub(crate) const SELECTION_HIGHLIGHT_OPACITY: f32 = 0.25;
/// Caret width.
pub(crate) const CARET_WIDTH: Pixels = px(2.0);
/// Caret blink period.
pub(crate) const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
/// Max delay between clicks still counted as a double/triple click.
pub(crate) const MULTI_CLICK_WINDOW: Duration = Duration::from_millis(500);
/// Max pointer travel between clicks still counted as a double click.
pub(crate) const MULTI_CLICK_SLOP: Pixels = px(5.0);
/// IME composition underline thickness.
pub(crate) const IME_UNDERLINE_THICKNESS: Pixels = px(1.0);

/// Spawn the caret blink loop shared by every text-editing state. The task
/// toggles caret visibility through `set_visible` every
/// [`CARET_BLINK_INTERVAL`] until the entity is released or the task is
/// dropped by a subsequent blink restart.
pub(crate) fn spawn_caret_blink<T: 'static>(
    cx: &mut Context<T>,
    mut set_visible: impl FnMut(&mut T, &mut Context<T>) + 'static,
) -> Task<()> {
    cx.spawn(async move |this, cx| loop {
        cx.background_executor().timer(CARET_BLINK_INTERVAL).await;
        let ok = this.update(cx, |state, cx| set_visible(state, cx));
        if ok.is_err() {
            break;
        }
    })
}

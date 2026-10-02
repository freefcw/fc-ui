use gpui::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static SHIMMER_ID: AtomicU32 = AtomicU32::new(0);

/// Shimmer 扫过效果。扫光在 -200px..600px 区间内循环移动，
/// 适用于宽度不超过约 600px 的容器。
#[derive(IntoElement)]
pub struct Shimmer {
    id: ElementId,
    base: Div,
    duration: Duration,
}

impl Shimmer {
    pub fn new() -> Self {
        let auto_id = SHIMMER_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id: ElementId::Name(format!("shimmer-{auto_id}").into()),
            base: div(),
            duration: Duration::from_millis(1500),
        }
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

impl Default for Shimmer {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Shimmer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let duration = self.duration;

        self.base.relative().overflow_hidden().child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .w(px(200.0))
                .bg(gpui::linear_gradient(
                    90.0,
                    gpui::linear_color_stop(gpui::transparent_black(), 0.0),
                    gpui::linear_color_stop(hsla(0.0, 0.0, 1.0, 0.15), 1.0),
                ))
                .with_animation(
                    ElementId::Name(format!("{}-sweep", self.id).into()),
                    Animation::new(duration).repeat().with_easing(gpui::linear),
                    move |this, delta| {
                        let start = px(-200.0);
                        let end = px(600.0);
                        let current = start + (end - start) * delta;
                        this.left(current)
                    },
                ),
        )
    }
}

impl Styled for Shimmer {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Shimmer {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Shimmer {}

impl ParentElement for Shimmer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements)
    }
}

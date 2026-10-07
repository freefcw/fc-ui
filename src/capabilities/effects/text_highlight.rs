use crate::capabilities::motion::animations::easings;
use gpui::{prelude::FluentBuilder as _, *};
use std::time::Duration;

#[derive(IntoElement)]
pub struct TextHighlight {
    id: ElementId,
    color: Option<Hsla>,
    duration: Duration,
    delay: Duration,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl TextHighlight {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            color: None,
            duration: Duration::from_millis(600),
            delay: Duration::from_millis(0),
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

impl Styled for TextHighlight {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TextHighlight {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for TextHighlight {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let highlight_color = self.color.unwrap_or(hsla(0.15, 0.9, 0.6, 0.3));
        let anim_id: ElementId = ElementId::Name(format!("{}-sweep", self.id).into());
        let user_style = self.style;

        let overlay = div()
            .id(anim_id.clone())
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .bg(highlight_color)
            .with_animation(
                anim_id,
                Animation::new(self.duration)
                    .with_easing(easings::ease_out_cubic)
                    .with_delay(self.delay),
                move |el, t| el.w(relative(t)),
            );

        let mut container = div().relative().child(overlay).map(|mut el| {
            el.style().refine(&user_style);
            el
        });

        for child in self.children {
            container = container.child(div().relative().child(child));
        }

        container
    }
}

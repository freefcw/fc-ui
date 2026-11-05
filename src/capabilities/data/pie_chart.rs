use crate::capabilities::data::pie_geometry::{
    positive_finite_total, render_dot_ring, render_legend, segment_geometry,
};
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};

fn pixels_to_f32(p: Pixels) -> f32 {
    p / px(1.0)
}

#[derive(Clone)]
pub struct PieChartSegment {
    pub label: SharedString,
    pub value: f64,
    pub color: Option<Hsla>,
}

impl PieChartSegment {
    pub fn new(label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            label: label.into(),
            value: value.max(0.0),
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum PieChartVariant {
    #[default]
    Pie,
    Donut,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum PieChartLabelPosition {
    #[default]
    None,
    Legend,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum PieChartSize {
    Sm,
    #[default]
    Md,
    Lg,
    Custom(u32),
}

impl PieChartSize {
    fn to_pixels(self) -> Pixels {
        match self {
            PieChartSize::Sm => px(120.0),
            PieChartSize::Md => px(200.0),
            PieChartSize::Lg => px(280.0),
            PieChartSize::Custom(size) => px(size as f32),
        }
    }
}

#[derive(IntoElement)]
pub struct PieChart {
    segments: Vec<PieChartSegment>,
    variant: PieChartVariant,
    label_position: PieChartLabelPosition,
    show_percentages: bool,
    center_label: Option<SharedString>,
    size: PieChartSize,
    donut_thickness: f32,
    style: StyleRefinement,
}

impl PieChart {
    pub fn new(segments: Vec<PieChartSegment>) -> Self {
        Self {
            segments,
            variant: PieChartVariant::Pie,
            label_position: PieChartLabelPosition::None,
            show_percentages: false,
            center_label: None,
            size: PieChartSize::Md,
            donut_thickness: 0.35,
            style: StyleRefinement::default(),
        }
    }

    pub fn pie(segments: Vec<PieChartSegment>) -> Self {
        Self::new(segments).variant(PieChartVariant::Pie)
    }

    pub fn donut(segments: Vec<PieChartSegment>) -> Self {
        Self::new(segments).variant(PieChartVariant::Donut)
    }

    pub fn variant(mut self, variant: PieChartVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: PieChartSize) -> Self {
        self.size = size;
        self
    }

    pub fn size_px(mut self, size_val: u32) -> Self {
        self.size = PieChartSize::Custom(size_val);
        self
    }

    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    pub fn center_label(mut self, label: impl Into<SharedString>) -> Self {
        self.center_label = Some(label.into());
        self
    }

    pub fn donut_thickness(mut self, thickness: f32) -> Self {
        self.donut_thickness = thickness.clamp(0.1, 0.9);
        self
    }

    pub fn label_position(mut self, position: PieChartLabelPosition) -> Self {
        self.label_position = position;
        self
    }
}

impl Styled for PieChart {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PieChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chart_size = self.size.to_pixels();
        let show_legend = self.label_position == PieChartLabelPosition::Legend;
        let show_percentages = self.show_percentages;
        let user_style = self.style;

        let total = positive_finite_total(&self.segments);

        let chart = if total == 0.0 || self.segments.is_empty() {
            render_empty_chart(chart_size, cx)
        } else {
            render_pie_chart(
                chart_size,
                &self.segments,
                total,
                self.variant,
                self.donut_thickness,
                self.center_label.clone(),
                cx,
            )
        };

        let legend = if show_legend {
            Some(render_legend(
                &self.segments,
                total,
                show_percentages,
                &use_theme(cx),
            ))
        } else {
            None
        };

        div()
            .flex()
            .gap(px(24.0))
            .items_center()
            .child(chart)
            .when_some(legend, |this, legend| this.child(legend))
            .map(|this| {
                let mut d = this;
                d.style().refine(&user_style);
                d
            })
    }
}

fn render_pie_chart(
    chart_size: Pixels,
    segments: &[PieChartSegment],
    total: f64,
    variant: PieChartVariant,
    donut_thickness: f32,
    center_label: Option<SharedString>,
    cx: &App,
) -> Div {
    let theme = use_theme(cx);
    let size_f32 = pixels_to_f32(chart_size);
    let center = size_f32 * 0.5;
    let outer_radius = size_f32 * 0.5;
    let inner_radius = if variant == PieChartVariant::Donut {
        outer_radius * (1.0 - donut_thickness)
    } else {
        0.0
    };

    let segment_data = segment_geometry(segments, total);

    if segment_data.len() == 1 {
        return render_single_segment(
            chart_size,
            segment_data[0].color,
            inner_radius,
            variant,
            center_label,
            cx,
        );
    }

    let mut container = render_dot_ring(chart_size, inner_radius, outer_radius, &segment_data);

    if variant == PieChartVariant::Donut {
        let inner_size = inner_radius * 2.0 - 4.0;
        let inner_offset = center - inner_radius + 2.0;

        container = container.child(
            div()
                .absolute()
                .size(px(inner_size))
                .rounded(px(9999.0))
                .bg(theme.tokens.background)
                .left(px(inner_offset))
                .top(px(inner_offset))
                .flex()
                .items_center()
                .justify_center()
                .when_some(center_label, |this, label| {
                    this.child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.tokens.foreground)
                            .child(label),
                    )
                }),
        );
    }

    container
}

fn render_single_segment(
    chart_size: Pixels,
    color: Hsla,
    inner_radius: f32,
    variant: PieChartVariant,
    center_label: Option<SharedString>,
    cx: &App,
) -> Div {
    let theme = use_theme(cx);
    let size_f32 = pixels_to_f32(chart_size);
    let center = size_f32 * 0.5;

    let mut container = div()
        .size(chart_size)
        .rounded(px(9999.0))
        .relative()
        .bg(color);

    if variant == PieChartVariant::Donut {
        let inner_size = inner_radius * 2.0;
        let inner_offset = center - inner_radius;

        container = container.child(
            div()
                .absolute()
                .size(px(inner_size))
                .rounded(px(9999.0))
                .bg(theme.tokens.background)
                .left(px(inner_offset))
                .top(px(inner_offset))
                .flex()
                .items_center()
                .justify_center()
                .when_some(center_label, |this, label| {
                    this.child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.tokens.foreground)
                            .child(label),
                    )
                }),
        );
    }

    container
}

fn render_empty_chart(chart_size: Pixels, cx: &App) -> Div {
    let theme = use_theme(cx);

    div()
        .size(chart_size)
        .rounded(px(9999.0))
        .bg(theme.tokens.muted)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_sm()
                .text_color(theme.tokens.muted_foreground)
                .child("No data"),
        )
}

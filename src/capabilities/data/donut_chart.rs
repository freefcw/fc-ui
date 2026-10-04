use crate::capabilities::data::pie_chart::PieChartSegment;
use crate::capabilities::data::pie_geometry::{
    color_at_angle, positive_finite_total, render_legend, segment_geometry,
};
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum DonutChartSize {
    Sm,
    #[default]
    Md,
    Lg,
    Custom(u32),
}

impl DonutChartSize {
    fn to_pixels(self) -> Pixels {
        match self {
            DonutChartSize::Sm => px(120.0),
            DonutChartSize::Md => px(200.0),
            DonutChartSize::Lg => px(280.0),
            DonutChartSize::Custom(size) => px(size as f32),
        }
    }
}

#[derive(IntoElement)]
pub struct DonutChart {
    segments: Vec<PieChartSegment>,
    inner_radius: f32,
    center_label: Option<SharedString>,
    center_value: Option<SharedString>,
    size: DonutChartSize,
    show_legend: bool,
    show_percentages: bool,
    style: StyleRefinement,
}

impl DonutChart {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            inner_radius: 0.6,
            center_label: None,
            center_value: None,
            size: DonutChartSize::default(),
            show_legend: false,
            show_percentages: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn segments(mut self, segments: Vec<PieChartSegment>) -> Self {
        self.segments = segments;
        self
    }

    pub fn segment(mut self, segment: PieChartSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn inner_radius(mut self, ratio: f32) -> Self {
        self.inner_radius = ratio.clamp(0.0, 0.9);
        self
    }

    pub fn center_label(mut self, label: impl Into<SharedString>) -> Self {
        self.center_label = Some(label.into());
        self
    }

    pub fn center_value(mut self, value: impl Into<SharedString>) -> Self {
        self.center_value = Some(value.into());
        self
    }

    pub fn size(mut self, size: DonutChartSize) -> Self {
        self.size = size;
        self
    }

    pub fn size_px(mut self, size_val: u32) -> Self {
        self.size = DonutChartSize::Custom(size_val);
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }
}

impl Default for DonutChart {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for DonutChart {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DonutChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;
        let chart_size = self.size.to_pixels();
        let show_legend = self.show_legend;
        let show_percentages = self.show_percentages;

        let total = positive_finite_total(&self.segments);

        let chart = if total == 0.0 || self.segments.is_empty() {
            render_empty(chart_size, &theme)
        } else {
            render_donut(
                chart_size,
                &self.segments,
                total,
                self.inner_radius,
                self.center_label.clone(),
                self.center_value.clone(),
                &theme,
            )
        };

        let legend = if show_legend {
            Some(render_legend(
                &self.segments,
                total,
                show_percentages,
                &theme,
            ))
        } else {
            None
        };

        div()
            .flex()
            .gap(px(24.0))
            .items_center()
            .child(chart)
            .when_some(legend, |this, l| this.child(l))
            .map(|this| {
                let mut d = this;
                d.style().refine(&user_style);
                d
            })
    }
}

fn render_empty(chart_size: Pixels, theme: &crate::capabilities::foundation::theme::Theme) -> Div {
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

fn render_donut(
    chart_size: Pixels,
    segments: &[PieChartSegment],
    total: f64,
    inner_ratio: f32,
    center_label: Option<SharedString>,
    center_value: Option<SharedString>,
    theme: &crate::capabilities::foundation::theme::Theme,
) -> Div {
    let size_f32 = chart_size / px(1.0);
    let center = size_f32 * 0.5;
    let outer_radius = size_f32 * 0.5;
    let inner_radius = outer_radius * inner_ratio;

    let segment_data = segment_geometry(segments, total);

    if segment_data.len() == 1 {
        return render_single_segment(
            chart_size,
            segment_data[0].color,
            inner_radius,
            center_label,
            center_value,
            theme,
        );
    }

    let ring_width = outer_radius - inner_radius;
    let ring_count = ((ring_width / 3.0).max(1.0) as usize).min(20);

    let mut container = div()
        .size(chart_size)
        .rounded(px(9999.0))
        .relative()
        .overflow_hidden();

    for ring_idx in 0..ring_count {
        let ring_radius = inner_radius + (ring_idx as f32 + 0.5) * (ring_width / ring_count as f32);
        let circumference = std::f32::consts::TAU * ring_radius;
        let dots_in_ring = (circumference / 4.0).max(16.0) as usize;

        for i in 0..dots_in_ring {
            let angle = -std::f32::consts::FRAC_PI_2
                + (i as f32 / dots_in_ring as f32) * std::f32::consts::TAU;
            let color = color_at_angle(angle, &segment_data);
            let x = center + ring_radius * angle.cos() - 2.0;
            let y = center + ring_radius * angle.sin() - 2.0;

            container = container.child(
                div()
                    .absolute()
                    .size(px(5.0))
                    .rounded(px(9999.0))
                    .bg(color)
                    .left(px(x))
                    .top(px(y)),
            );
        }
    }

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
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(2.0))
            .when_some(center_value, |this, val| {
                this.child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.tokens.foreground)
                        .child(val),
                )
            })
            .when_some(center_label, |this, lbl| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme.tokens.muted_foreground)
                        .child(lbl),
                )
            }),
    );

    container
}

fn render_single_segment(
    chart_size: Pixels,
    color: Hsla,
    inner_radius: f32,
    center_label: Option<SharedString>,
    center_value: Option<SharedString>,
    theme: &crate::capabilities::foundation::theme::Theme,
) -> Div {
    let size_f32 = chart_size / px(1.0);
    let center = size_f32 * 0.5;
    let inner_size = inner_radius * 2.0;
    let inner_offset = center - inner_radius;

    div()
        .size(chart_size)
        .rounded(px(9999.0))
        .relative()
        .bg(color)
        .child(
            div()
                .absolute()
                .size(px(inner_size))
                .rounded(px(9999.0))
                .bg(theme.tokens.background)
                .left(px(inner_offset))
                .top(px(inner_offset))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .when_some(center_value, |this, val| {
                    this.child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.tokens.foreground)
                            .child(val),
                    )
                })
                .when_some(center_label, |this, lbl| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(theme.tokens.muted_foreground)
                            .child(lbl),
                    )
                }),
        )
}

use crate::capabilities::data::palette::default_color;
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};

fn pixels_to_f32(p: Pixels) -> f32 {
    p / px(1.0)
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
struct ValueDomain {
    min: f64,
    max: f64,
    span: f64,
    zero_ratio: f32,
}

impl ValueDomain {
    fn from_values(values: impl IntoIterator<Item = f64>) -> Self {
        let (min, max) = values
            .into_iter()
            .map(finite_or_zero)
            .fold((0.0_f64, 0.0_f64), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let span = (max - min).max(f64::EPSILON);
        Self {
            min,
            max,
            span,
            zero_ratio: (max / span) as f32,
        }
    }

    fn vertical_y(self, value: f64, height: Pixels) -> Pixels {
        height * ((self.max - finite_or_zero(value)) / self.span) as f32
    }

    fn vertical_segment(self, start: f64, end: f64, height: Pixels) -> (Pixels, Pixels) {
        let start_y = self.vertical_y(start, height);
        let end_y = self.vertical_y(end, height);
        (start_y.min(end_y), (start_y - end_y).abs())
    }

    fn horizontal_x(self, value: f64) -> f32 {
        ((finite_or_zero(value) - self.min) / self.span) as f32
    }

    fn horizontal_segment(self, start: f64, end: f64) -> (f32, f32) {
        let start_x = self.horizontal_x(start);
        let end_x = self.horizontal_x(end);
        (start_x.min(end_x), (start_x - end_x).abs())
    }

    fn zero_y(self, height: Pixels) -> Pixels {
        height * self.zero_ratio
    }
}

fn stacked_domain(series: &[BarChartSeries], label_count: usize) -> ValueDomain {
    ValueDomain::from_values((0..label_count).flat_map(|index| {
        let mut positive = 0.0;
        let mut negative = 0.0;
        for value in series {
            let value = finite_or_zero(value.data.get(index).copied().unwrap_or(0.0));
            if value >= 0.0 {
                positive += value;
            } else {
                negative += value;
            }
        }
        [positive, negative]
    }))
}

#[derive(Clone)]
pub struct BarChartData {
    pub label: SharedString,
    /// Finite values are rendered around a zero axis. Non-finite values are
    /// treated as zero.
    pub value: f64,
    pub color: Option<Hsla>,
}

impl BarChartData {
    pub fn new(label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Clone)]
pub struct BarChartSeries {
    pub name: SharedString,
    /// Finite values are rendered around a zero axis. Non-finite values render
    /// as 0.
    pub data: Vec<f64>,
    pub color: Option<Hsla>,
}

impl BarChartSeries {
    pub fn new(name: impl Into<SharedString>, data: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            data,
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum BarChartOrientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum BarChartMode {
    #[default]
    Single,
    Grouped,
    Stacked,
}

#[derive(IntoElement)]
pub struct BarChart {
    data: Vec<BarChartData>,
    series: Vec<BarChartSeries>,
    labels: Vec<SharedString>,
    orientation: BarChartOrientation,
    mode: BarChartMode,
    show_values: bool,
    show_grid: bool,
    show_legend: bool,
    show_axis_labels: bool,
    bar_width: Option<Pixels>,
    gap: Pixels,
    height: Pixels,
    style: StyleRefinement,
}

impl BarChart {
    pub fn new(data: Vec<BarChartData>) -> Self {
        Self {
            data,
            series: Vec::new(),
            labels: Vec::new(),
            orientation: BarChartOrientation::default(),
            mode: BarChartMode::Single,
            show_values: false,
            show_grid: false,
            show_legend: false,
            show_axis_labels: true,
            bar_width: None,
            gap: px(8.0),
            height: px(300.0),
            style: StyleRefinement::default(),
        }
    }

    pub fn multi_series(labels: Vec<impl Into<SharedString>>, series: Vec<BarChartSeries>) -> Self {
        Self {
            data: Vec::new(),
            series,
            labels: labels.into_iter().map(|l| l.into()).collect(),
            orientation: BarChartOrientation::default(),
            mode: BarChartMode::Grouped,
            show_values: false,
            show_grid: false,
            show_legend: true,
            show_axis_labels: true,
            bar_width: None,
            gap: px(8.0),
            height: px(300.0),
            style: StyleRefinement::default(),
        }
    }

    pub fn vertical(mut self) -> Self {
        self.orientation = BarChartOrientation::Vertical;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.orientation = BarChartOrientation::Horizontal;
        self
    }

    pub fn orientation(mut self, orientation: BarChartOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn stacked(mut self) -> Self {
        self.mode = BarChartMode::Stacked;
        self
    }

    pub fn grouped(mut self) -> Self {
        self.mode = BarChartMode::Grouped;
        self
    }

    pub fn mode(mut self, mode: BarChartMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    pub fn show_axis_labels(mut self, show: bool) -> Self {
        self.show_axis_labels = show;
        self
    }

    pub fn bar_width(mut self, width: Pixels) -> Self {
        self.bar_width = Some(width);
        self
    }

    pub fn gap(mut self, gap: Pixels) -> Self {
        self.gap = gap;
        self
    }

    pub fn chart_height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    fn render_single_vertical(self, theme: &crate::capabilities::foundation::theme::Theme) -> Div {
        let domain = ValueDomain::from_values(self.data.iter().map(|d| d.value));

        let chart_height = self.height;
        let bar_width = self.bar_width.unwrap_or(px(40.0));
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;

        let grid_lines = if show_grid {
            Some(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .children((0..5).map(|_| {
                        div()
                            .w_full()
                            .h(px(1.0))
                            .bg(theme.tokens.border.opacity(0.3))
                    })),
            )
        } else {
            None
        };

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .relative()
                    .h(chart_height)
                    .w_full()
                    .when_some(grid_lines, |this, grid| this.child(grid))
                    .child(
                        div()
                            .relative()
                            .h_full()
                            .w_full()
                            .flex()
                            .items_end()
                            .justify_around()
                            .gap(gap)
                            .px(px(16.0))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .right_0()
                                    .top(domain.zero_y(chart_height))
                                    .h(px(1.0))
                                    .bg(theme.tokens.border.opacity(0.7)),
                            )
                            .children(self.data.iter().enumerate().map(|(i, item)| {
                                let value = finite_or_zero(item.value);
                                let (top, bar_height) =
                                    domain.vertical_segment(0.0, value, chart_height);
                                let label_top = if value >= 0.0 {
                                    top - px(18.0)
                                } else {
                                    top + bar_height + px(2.0)
                                };
                                let bar_color = item.color.unwrap_or_else(|| default_color(i));

                                div()
                                    .flex_1()
                                    .h_full()
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div().relative().w(bar_width).h_full().child(
                                            div()
                                                .absolute()
                                                .left_0()
                                                .right_0()
                                                .top(top)
                                                .h(bar_height)
                                                .bg(bar_color)
                                                .rounded(theme.tokens.radius_sm),
                                        ),
                                    )
                                    .when(show_values, |this| {
                                        this.child(
                                            div()
                                                .absolute()
                                                .top(label_top)
                                                .left_0()
                                                .right_0()
                                                .text_xs()
                                                .text_color(theme.tokens.muted_foreground)
                                                .text_center()
                                                .child(format!("{:.0}", value)),
                                        )
                                    })
                            })),
                    ),
            )
            .when(show_axis_labels, |this| {
                this.child(div().flex().justify_around().px(px(16.0)).children(
                    self.data.iter().map(|item| {
                        div()
                            .text_xs()
                            .text_color(theme.tokens.muted_foreground)
                            .text_center()
                            .child(item.label.clone())
                    }),
                ))
            })
    }

    fn render_single_horizontal(
        self,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Div {
        let domain = ValueDomain::from_values(self.data.iter().map(|d| d.value));

        let bar_height = self.bar_width.unwrap_or(px(24.0));
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;
        let grid_color = theme.tokens.border.opacity(0.3);

        div()
            .flex()
            .flex_col()
            .gap(gap)
            .children(self.data.iter().enumerate().map(|(i, item)| {
                let value = finite_or_zero(item.value);
                let (left, width_percent) = domain.horizontal_segment(0.0, value);
                let bar_color = item.color.unwrap_or_else(|| default_color(i));
                let label = item.label.clone();

                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(show_axis_labels, |this| {
                        this.child(
                            div()
                                .w(px(80.0))
                                .text_xs()
                                .text_color(theme.tokens.muted_foreground)
                                .text_right()
                                .overflow_hidden()
                                .child(label),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .h(bar_height)
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(relative(domain.zero_ratio))
                                    .w(px(1.0))
                                    .bg(theme.tokens.border.opacity(0.7)),
                            )
                            .when(show_grid, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .flex()
                                        .justify_between()
                                        .children(
                                            (0..5)
                                                .map(|_| div().h_full().w(px(1.0)).bg(grid_color)),
                                        ),
                                )
                            })
                            .child({
                                let width = width_percent;
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(relative(left))
                                    .w(relative(width))
                                    .bg(bar_color)
                                    .rounded(theme.tokens.radius_sm)
                            }),
                    )
                    .when(show_values, |this| {
                        this.child(
                            div()
                                .w(px(50.0))
                                .text_xs()
                                .text_color(theme.tokens.muted_foreground)
                                .child(format!("{:.0}", value)),
                        )
                    })
            }))
    }

    fn render_multi_vertical_grouped(
        self,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Div {
        let domain =
            ValueDomain::from_values(self.series.iter().flat_map(|s| s.data.iter()).copied());

        let chart_height = self.height;
        let series_count = self.series.len();
        let bar_width = self.bar_width.unwrap_or(px(24.0));
        let single_bar_width = px(pixels_to_f32(bar_width) / series_count.max(1) as f32);
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;
        let show_legend = self.show_legend;

        let grid_lines = if show_grid {
            Some(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .children((0..5).map(|_| {
                        div()
                            .w_full()
                            .h(px(1.0))
                            .bg(theme.tokens.border.opacity(0.3))
                    })),
            )
        } else {
            None
        };

        let labels = self.labels.clone();
        let series_for_legend = self.series.clone();
        let label_count = labels.len();

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .relative()
                    .h(chart_height)
                    .w_full()
                    .when_some(grid_lines, |this, grid| this.child(grid))
                    .child(
                        div()
                            .h_full()
                            .w_full()
                            .flex()
                            .items_end()
                            .justify_around()
                            .gap(gap)
                            .px(px(16.0))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .right_0()
                                    .top(domain.zero_y(chart_height))
                                    .h(px(1.0))
                                    .bg(theme.tokens.border.opacity(0.7)),
                            )
                            .children((0..label_count).map(|label_idx| {
                                div().flex().items_end().h_full().gap(px(2.0)).children(
                                    self.series.iter().enumerate().map(|(series_idx, series)| {
                                        let value = finite_or_zero(
                                            series.data.get(label_idx).copied().unwrap_or(0.0),
                                        );
                                        let (top, bar_height) =
                                            domain.vertical_segment(0.0, value, chart_height);
                                        let label_top = if value >= 0.0 {
                                            top - px(18.0)
                                        } else {
                                            top + bar_height + px(2.0)
                                        };
                                        let bar_color = series
                                            .color
                                            .unwrap_or_else(|| default_color(series_idx));

                                        div()
                                            .relative()
                                            .w(single_bar_width)
                                            .h_full()
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left_0()
                                                    .right_0()
                                                    .top(top)
                                                    .h(bar_height)
                                                    .bg(bar_color)
                                                    .rounded(theme.tokens.radius_sm),
                                            )
                                            .when(show_values, |this| {
                                                this.child(
                                                    div()
                                                        .absolute()
                                                        .top(label_top)
                                                        .left_0()
                                                        .right_0()
                                                        .text_xs()
                                                        .text_color(theme.tokens.muted_foreground)
                                                        .text_center()
                                                        .child(format!("{:.0}", value)),
                                                )
                                            })
                                    }),
                                )
                            })),
                    ),
            )
            .when(show_axis_labels, |this| {
                this.child(
                    div()
                        .flex()
                        .justify_around()
                        .px(px(16.0))
                        .children(labels.iter().map(|label| {
                            div()
                                .text_xs()
                                .text_color(theme.tokens.muted_foreground)
                                .text_center()
                                .child(label.clone())
                        })),
                )
            })
            .when(show_legend, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(16.0))
                        .justify_center()
                        .children(series_for_legend.iter().enumerate().map(|(i, s)| {
                            let color = s.color.unwrap_or_else(|| default_color(i));
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(s.name.clone()),
                                )
                        })),
                )
            })
    }

    fn render_multi_vertical_stacked(
        self,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Div {
        let label_count = self.labels.len();
        let domain = stacked_domain(&self.series, label_count);

        let chart_height = self.height;
        let bar_width = self.bar_width.unwrap_or(px(40.0));
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;
        let show_legend = self.show_legend;

        let grid_lines = if show_grid {
            Some(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .children((0..5).map(|_| {
                        div()
                            .w_full()
                            .h(px(1.0))
                            .bg(theme.tokens.border.opacity(0.3))
                    })),
            )
        } else {
            None
        };

        let labels = self.labels.clone();
        let series_for_legend = self.series.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .relative()
                    .h(chart_height)
                    .w_full()
                    .when_some(grid_lines, |this, grid| this.child(grid))
                    .child(
                        div()
                            .h_full()
                            .w_full()
                            .flex()
                            .items_end()
                            .justify_around()
                            .gap(gap)
                            .px(px(16.0))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .right_0()
                                    .top(domain.zero_y(chart_height))
                                    .h(px(1.0))
                                    .bg(theme.tokens.border.opacity(0.7)),
                            )
                            .children((0..label_count).map(|label_idx| {
                                let total_value: f64 = self
                                    .series
                                    .iter()
                                    .map(|series| {
                                        finite_or_zero(
                                            series.data.get(label_idx).copied().unwrap_or(0.0),
                                        )
                                    })
                                    .sum();
                                let mut positive = 0.0;
                                let mut negative = 0.0;

                                div()
                                    .flex_1()
                                    .h_full()
                                    .flex()
                                    .justify_center()
                                    .when(show_values, |this| {
                                        let label_top = if total_value >= 0.0 {
                                            domain.zero_y(chart_height) - px(18.0)
                                        } else {
                                            domain.zero_y(chart_height) + px(2.0)
                                        };
                                        this.child(
                                            div()
                                                .absolute()
                                                .top(label_top)
                                                .left_0()
                                                .right_0()
                                                .text_xs()
                                                .text_color(theme.tokens.muted_foreground)
                                                .text_center()
                                                .child(format!("{:.0}", total_value)),
                                        )
                                    })
                                    .child(div().relative().w(bar_width).h_full().children(
                                        self.series.iter().enumerate().map(
                                            |(series_idx, series)| {
                                                let value = finite_or_zero(
                                                    series
                                                        .data
                                                        .get(label_idx)
                                                        .copied()
                                                        .unwrap_or(0.0),
                                                );
                                                let (start, end) = if value >= 0.0 {
                                                    let start = positive;
                                                    positive += value;
                                                    (start, positive)
                                                } else {
                                                    let start = negative;
                                                    negative += value;
                                                    (start, negative)
                                                };
                                                let (top, height) = domain.vertical_segment(
                                                    start,
                                                    end,
                                                    chart_height,
                                                );
                                                let bar_color = series
                                                    .color
                                                    .unwrap_or_else(|| default_color(series_idx));

                                                div()
                                                    .absolute()
                                                    .left_0()
                                                    .right_0()
                                                    .top(top)
                                                    .h(height)
                                                    .bg(bar_color)
                                            },
                                        ),
                                    ))
                            })),
                    ),
            )
            .when(show_axis_labels, |this| {
                this.child(
                    div()
                        .flex()
                        .justify_around()
                        .px(px(16.0))
                        .children(labels.iter().map(|label| {
                            div()
                                .text_xs()
                                .text_color(theme.tokens.muted_foreground)
                                .text_center()
                                .child(label.clone())
                        })),
                )
            })
            .when(show_legend, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(16.0))
                        .justify_center()
                        .children(series_for_legend.iter().enumerate().map(|(i, s)| {
                            let color = s.color.unwrap_or_else(|| default_color(i));
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(s.name.clone()),
                                )
                        })),
                )
            })
    }

    fn render_multi_horizontal_grouped(
        self,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Div {
        let domain =
            ValueDomain::from_values(self.series.iter().flat_map(|s| s.data.iter()).copied());

        let series_count = self.series.len();
        let bar_height = self.bar_width.unwrap_or(px(16.0));
        let single_bar_height = px(pixels_to_f32(bar_height) / series_count.max(1) as f32);
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;
        let show_legend = self.show_legend;
        let grid_color = theme.tokens.border.opacity(0.3);

        let labels = self.labels.clone();
        let series_for_legend = self.series.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(gap)
                    .children(labels.iter().enumerate().map(|(label_idx, label)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(show_axis_labels, |this| {
                                this.child(
                                    div()
                                        .w(px(80.0))
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .text_right()
                                        .overflow_hidden()
                                        .child(label.clone()),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .relative()
                                    .h(bar_height)
                                    .when(show_grid, |this| {
                                        this.child(
                                            div()
                                                .absolute()
                                                .inset_0()
                                                .flex()
                                                .justify_between()
                                                .children((0..5).map(|_| {
                                                    div().h_full().w(px(1.0)).bg(grid_color)
                                                })),
                                        )
                                    })
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .left(relative(domain.zero_ratio))
                                            .w(px(1.0))
                                            .bg(theme.tokens.border.opacity(0.7)),
                                    )
                                    .children(self.series.iter().enumerate().map(
                                        |(series_idx, series)| {
                                            let value = finite_or_zero(
                                                series.data.get(label_idx).copied().unwrap_or(0.0),
                                            );
                                            let (left, width_percent) =
                                                domain.horizontal_segment(0.0, value);
                                            let bar_color = series
                                                .color
                                                .unwrap_or_else(|| default_color(series_idx));

                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .top(single_bar_height * series_idx as f32)
                                                        .left(relative(left))
                                                        .h(single_bar_height)
                                                        .w(relative(width_percent))
                                                        .bg(bar_color)
                                                        .rounded_r(theme.tokens.radius_sm),
                                                )
                                                .when(show_values, |this| {
                                                    this.child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(
                                                                theme.tokens.muted_foreground,
                                                            )
                                                            .child(format!("{:.0}", value)),
                                                    )
                                                })
                                        },
                                    )),
                            )
                    })),
            )
            .when(show_legend, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(16.0))
                        .justify_center()
                        .children(series_for_legend.iter().enumerate().map(|(i, s)| {
                            let color = s.color.unwrap_or_else(|| default_color(i));
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(s.name.clone()),
                                )
                        })),
                )
            })
    }

    fn render_multi_horizontal_stacked(
        self,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Div {
        let domain = stacked_domain(&self.series, self.labels.len());

        let bar_height = self.bar_width.unwrap_or(px(24.0));
        let gap = self.gap;
        let show_values = self.show_values;
        let show_grid = self.show_grid;
        let show_axis_labels = self.show_axis_labels;
        let show_legend = self.show_legend;
        let grid_color = theme.tokens.border.opacity(0.3);

        let labels = self.labels.clone();
        let series_for_legend = self.series.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(gap)
                    .children(labels.iter().enumerate().map(|(label_idx, label)| {
                        let total_value: f64 = self
                            .series
                            .iter()
                            .map(|series| {
                                finite_or_zero(series.data.get(label_idx).copied().unwrap_or(0.0))
                            })
                            .sum();
                        let mut positive = 0.0;
                        let mut negative = 0.0;

                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(show_axis_labels, |this| {
                                this.child(
                                    div()
                                        .w(px(80.0))
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .text_right()
                                        .overflow_hidden()
                                        .child(label.clone()),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .relative()
                                    .h(bar_height)
                                    .when(show_grid, |this| {
                                        this.child(
                                            div()
                                                .absolute()
                                                .inset_0()
                                                .flex()
                                                .justify_between()
                                                .children((0..5).map(|_| {
                                                    div().h_full().w(px(1.0)).bg(grid_color)
                                                })),
                                        )
                                    })
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .left(relative(domain.zero_ratio))
                                            .w(px(1.0))
                                            .bg(theme.tokens.border.opacity(0.7)),
                                    )
                                    .child(div().absolute().inset_0().children(
                                        self.series.iter().enumerate().map(
                                            |(series_idx, series)| {
                                                let value = finite_or_zero(
                                                    series
                                                        .data
                                                        .get(label_idx)
                                                        .copied()
                                                        .unwrap_or(0.0),
                                                );
                                                let (start, end) = if value >= 0.0 {
                                                    let start = positive;
                                                    positive += value;
                                                    (start, positive)
                                                } else {
                                                    let start = negative;
                                                    negative += value;
                                                    (start, negative)
                                                };
                                                let (left, width) =
                                                    domain.horizontal_segment(start, end);
                                                let bar_color = series
                                                    .color
                                                    .unwrap_or_else(|| default_color(series_idx));

                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .bottom_0()
                                                    .left(relative(left))
                                                    .w(relative(width))
                                                    .bg(bar_color)
                                            },
                                        ),
                                    )),
                            )
                            .when(show_values, |this| {
                                this.child(
                                    div()
                                        .w(px(50.0))
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(format!("{:.0}", total_value)),
                                )
                            })
                    })),
            )
            .when(show_legend, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(16.0))
                        .justify_center()
                        .children(series_for_legend.iter().enumerate().map(|(i, s)| {
                            let color = s.color.unwrap_or_else(|| default_color(i));
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(s.name.clone()),
                                )
                        })),
                )
            })
    }
}

impl Styled for BarChart {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BarChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style.clone();

        let is_single = self.series.is_empty();
        let is_vertical = self.orientation == BarChartOrientation::Vertical;
        let is_stacked = self.mode == BarChartMode::Stacked;

        let content = match (is_single, is_vertical, is_stacked) {
            (true, true, _) => self.render_single_vertical(&theme),
            (true, false, _) => self.render_single_horizontal(&theme),
            (false, true, false) => self.render_multi_vertical_grouped(&theme),
            (false, true, true) => self.render_multi_vertical_stacked(&theme),
            (false, false, false) => self.render_multi_horizontal_grouped(&theme),
            (false, false, true) => self.render_multi_horizontal_stacked(&theme),
        };

        content.map(|this| {
            let mut div = this;
            div.style().refine(&user_style);
            div
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ValueDomain;
    use super::{BarChart, BarChartSeries};
    use crate::capabilities::foundation::theme::{install_theme, Theme};
    use gpui::{div, px, Context, IntoElement, ParentElement, Render, Styled, TestApp, Window};

    #[test]
    fn signed_domain_places_zero_between_positive_and_negative_values() {
        let domain = ValueDomain::from_values([10.0, -5.0, f64::NAN]);
        assert!((domain.zero_ratio - (10.0 / 15.0) as f32).abs() < f32::EPSILON);

        let (positive_top, positive_height) = domain.vertical_segment(0.0, 10.0, px(300.0));
        let (negative_top, negative_height) = domain.vertical_segment(0.0, -5.0, px(300.0));
        assert_eq!(positive_top, px(0.0));
        assert_eq!(positive_height, px(200.0));
        assert_eq!(negative_top, px(200.0));
        assert_eq!(negative_height, px(100.0));
    }

    #[test]
    fn non_finite_values_are_zero_but_negative_values_are_preserved() {
        let domain = ValueDomain::from_values([f64::INFINITY, -4.0]);
        assert_eq!(domain.min, -4.0);
        assert_eq!(domain.max, 0.0);
        let (left, width) = domain.horizontal_segment(0.0, -4.0);
        assert_eq!(left, 0.0);
        assert_eq!(width, 1.0);
    }

    struct SignedBarChartView;

    impl Render for SignedBarChartView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(
                    BarChart::multi_series(
                        vec!["Jan", "Feb"],
                        vec![
                            BarChartSeries::new("Revenue", vec![10.0, -5.0]),
                            BarChartSeries::new("Cost", vec![-3.0, 4.0]),
                        ],
                    )
                    .stacked()
                    .show_values(true),
                )
                .child(
                    BarChart::multi_series(
                        vec!["Jan", "Feb"],
                        vec![
                            BarChartSeries::new("Revenue", vec![10.0, -5.0]),
                            BarChartSeries::new("Cost", vec![-3.0, 4.0]),
                        ],
                    )
                    .horizontal()
                    .grouped()
                    .show_values(true),
                )
        }
    }

    #[gpui::test]
    fn signed_bar_chart_renders_without_panic() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, _| SignedBarChartView);
        window.draw();
    }
}

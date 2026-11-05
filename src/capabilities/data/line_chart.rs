use crate::capabilities::data::chart_common::{
    chart_legend_row, format_tick_value, line_area_axis_padding, paint_area_under_line,
    paint_horizontal_grid_lines, paint_series_line, paint_vertical_grid_lines, LegendText,
};
use crate::capabilities::data::{
    chart::{is_finite_data_point, DataPoint, DataRange},
    palette::default_color,
};
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};

#[derive(Clone, Debug)]
pub struct LineChartPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<SharedString>,
}

impl LineChartPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, label: None }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct LineChartSeries {
    pub name: SharedString,
    pub points: Vec<LineChartPoint>,
    pub color: Option<Hsla>,
    pub show_points: bool,
    pub fill_area: bool,
}

impl LineChartSeries {
    pub fn new(name: impl Into<SharedString>, points: Vec<LineChartPoint>) -> Self {
        Self {
            name: name.into(),
            points,
            color: None,
            show_points: false,
            fill_area: false,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    pub fn fill_area(mut self, fill: bool) -> Self {
        self.fill_area = fill;
        self
    }
}

fn range_from_series(
    series: &[LineChartSeries],
    y_min_override: Option<f64>,
    y_max_override: Option<f64>,
) -> DataRange {
    let points: Vec<_> = series
        .iter()
        .flat_map(|series| series.points.iter())
        .map(|point| DataPoint::new(point.x, point.y))
        .collect();
    let range = DataRange::from_points(&points);
    let y_min = y_min_override
        .filter(|value| value.is_finite())
        .unwrap_or(range.y_min);
    let y_max = y_max_override
        .filter(|value| value.is_finite())
        .unwrap_or(range.y_max);

    DataRange::new(range.x_min, range.x_max, y_min, y_max)
}

#[derive(IntoElement)]
pub struct LineChart {
    series: Vec<LineChartSeries>,
    show_grid: bool,
    show_x_axis: bool,
    show_y_axis: bool,
    x_axis_labels: Vec<SharedString>,
    y_min: Option<f64>,
    y_max: Option<f64>,
    smooth: bool,
    show_legend: bool,
    style: StyleRefinement,
}

impl LineChart {
    pub fn new(series: Vec<LineChartSeries>) -> Self {
        Self {
            series,
            show_grid: true,
            show_x_axis: true,
            show_y_axis: true,
            x_axis_labels: Vec::new(),
            y_min: None,
            y_max: None,
            smooth: false,
            show_legend: true,
            style: StyleRefinement::default(),
        }
    }

    pub fn single(series: LineChartSeries) -> Self {
        Self::new(vec![series])
    }

    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    pub fn show_x_axis(mut self, show: bool) -> Self {
        self.show_x_axis = show;
        self
    }

    pub fn show_y_axis(mut self, show: bool) -> Self {
        self.show_y_axis = show;
        self
    }

    pub fn smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }

    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_min = Some(min);
        self.y_max = Some(max);
        self
    }

    pub fn x_labels(mut self, labels: Vec<impl Into<SharedString>>) -> Self {
        self.x_axis_labels = labels.into_iter().map(|l| l.into()).collect();
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }
}

impl Styled for LineChart {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

struct PaintData {
    series: Vec<LineChartSeries>,
    show_grid: bool,
    smooth: bool,
    y_min: Option<f64>,
    y_max: Option<f64>,
    grid_color: Hsla,
    padding_left: f32,
    padding_right: f32,
    padding_top: f32,
    padding_bottom: f32,
}

impl RenderOnce for LineChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;

        let series = self.series.clone();
        let show_grid = self.show_grid;
        let show_x_axis = self.show_x_axis;
        let show_y_axis = self.show_y_axis;
        let smooth = self.smooth;
        let y_min = self.y_min;
        let y_max = self.y_max;
        let x_axis_labels = self.x_axis_labels.clone();

        let grid_color = theme.tokens.border;
        let text_color = theme.tokens.muted_foreground;

        let (padding_left, padding_right, padding_top, padding_bottom) =
            line_area_axis_padding(show_x_axis, show_y_axis);

        let series_for_legend = series.clone();

        let data_range = range_from_series(&series, y_min, y_max);

        let y_labels: Vec<String> = if show_y_axis {
            (0..=5)
                .map(|i| {
                    let normalized = i as f64 / 5.0;
                    let value = data_range.y_min
                        + (data_range.y_max - data_range.y_min) * (1.0 - normalized);
                    format_tick_value(value, None)
                })
                .collect()
        } else {
            Vec::new()
        };

        let paint_data = PaintData {
            series,
            show_grid,
            smooth,
            y_min,
            y_max,
            grid_color,
            padding_left,
            padding_right,
            padding_top,
            padding_bottom,
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .map(|this| {
                let mut d = this;
                d.style().refine(&user_style);
                d
            })
            .child(
                div()
                    .flex_1()
                    .min_h(px(200.0))
                    .relative()
                    .child(
                        canvas(
                            move |_bounds, _window, _cx| paint_data,
                            move |bounds, paint_data, window, _cx| {
                                if bounds.size.width <= px(0.0) || bounds.size.height <= px(0.0) {
                                    return;
                                }

                                let data_range = range_from_series(
                                    &paint_data.series,
                                    paint_data.y_min,
                                    paint_data.y_max,
                                );

                                let chart_left = bounds.left() + px(paint_data.padding_left);
                                let chart_right = bounds.right() - px(paint_data.padding_right);
                                let chart_top = bounds.top() + px(paint_data.padding_top);
                                let chart_bottom = bounds.bottom() - px(paint_data.padding_bottom);
                                let chart_width = chart_right - chart_left;
                                let chart_height = chart_bottom - chart_top;

                                if chart_width <= px(0.0) || chart_height <= px(0.0) {
                                    return;
                                }

                                if paint_data.show_grid {
                                    let grid_lines = 5;
                                    paint_horizontal_grid_lines(
                                        window,
                                        chart_left,
                                        chart_right,
                                        chart_top,
                                        chart_height,
                                        grid_lines,
                                        paint_data.grid_color.opacity(0.3),
                                    );
                                    paint_vertical_grid_lines(
                                        window,
                                        chart_top,
                                        chart_bottom,
                                        chart_left,
                                        chart_width,
                                        grid_lines,
                                        paint_data.grid_color.opacity(0.3),
                                    );
                                }

                                for (series_index, s) in paint_data.series.iter().enumerate() {
                                    if s.points.is_empty() {
                                        continue;
                                    }

                                    let color =
                                        s.color.unwrap_or_else(|| default_color(series_index));

                                    let screen_points: Vec<Point<Pixels>> = s
                                        .points
                                        .iter()
                                        .filter(|point| is_finite_data_point(point.x, point.y))
                                        .map(|p| {
                                            let norm_x = data_range.normalize_x(p.x);
                                            let norm_y = data_range.normalize_y(p.y);
                                            let screen_x = chart_left + chart_width * norm_x;
                                            let screen_y = chart_bottom - chart_height * norm_y;
                                            point(screen_x, screen_y)
                                        })
                                        .collect();

                                    if s.fill_area && screen_points.len() >= 2 {
                                        paint_area_under_line(
                                            window,
                                            &screen_points,
                                            chart_bottom,
                                            color.opacity(0.15),
                                        );
                                    }

                                    paint_series_line(
                                        window,
                                        &screen_points,
                                        paint_data.smooth,
                                        px(2.0),
                                        color,
                                    );

                                    if s.show_points {
                                        let point_radius = px(4.0);
                                        for pt in &screen_points {
                                            window.paint_quad(fill(
                                                Bounds::centered_at(
                                                    *pt,
                                                    size(point_radius * 2.0, point_radius * 2.0),
                                                ),
                                                color,
                                            ));
                                        }
                                    }
                                }
                            },
                        )
                        .size_full(),
                    )
                    .when(show_y_axis, |this| {
                        this.children(y_labels.iter().enumerate().map(|(i, label)| {
                            let top_percent = (i as f32 / 5.0) * 100.0;
                            div()
                                .absolute()
                                .left(px(4.0))
                                .top(relative(top_percent / 100.0))
                                .mt(px(padding_top - 6.0))
                                .text_size(px(11.0))
                                .text_color(text_color)
                                .child(label.clone())
                        }))
                    })
                    .when(show_x_axis && !x_axis_labels.is_empty(), |this| {
                        let num_labels = x_axis_labels.len();
                        this.children(x_axis_labels.iter().enumerate().map(|(i, label)| {
                            let left_percent = if num_labels == 1 {
                                50.0
                            } else {
                                (i as f32 / (num_labels - 1) as f32) * 100.0
                            };
                            let chart_width_percent =
                                (100.0 - padding_left / 4.0 - padding_right / 4.0) / 100.0;
                            let adjusted_left =
                                padding_left / 4.0 + left_percent * chart_width_percent;
                            div()
                                .absolute()
                                .bottom(px(8.0))
                                .left(relative(adjusted_left / 100.0))
                                .ml(px(-15.0))
                                .text_size(px(11.0))
                                .text_color(text_color)
                                .child(label.clone())
                        }))
                    }),
            )
            .when(self.show_legend && series_for_legend.len() > 1, |this| {
                this.child(
                    chart_legend_row(
                        series_for_legend
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                (s.name.clone(), s.color.unwrap_or_else(|| default_color(i)))
                            })
                            .collect(),
                        text_color,
                        LegendText::Sm,
                        Some(padding_left),
                        Some(8.0),
                    )
                    .justify_start(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{range_from_series, LineChartPoint, LineChartSeries};

    fn series(points: &[(f64, f64)]) -> Vec<LineChartSeries> {
        vec![LineChartSeries::new(
            "series",
            points
                .iter()
                .map(|&(x, y)| LineChartPoint::new(x, y))
                .collect(),
        )]
    }

    #[test]
    fn range_ignores_non_finite_points_and_overrides() {
        let range = range_from_series(
            &series(&[
                (0.0, 1.0),
                (f64::NAN, 2.0),
                (1.0, f64::INFINITY),
                (2.0, 3.0),
            ]),
            Some(f64::NEG_INFINITY),
            Some(f64::NAN),
        );

        assert_eq!(
            (range.x_min, range.x_max, range.y_min, range.y_max),
            (0.0, 2.0, 1.0, 3.0)
        );
        assert!(range.normalize_x(f64::NAN).is_finite());
        assert!(range.normalize_y(f64::INFINITY).is_finite());
    }
}

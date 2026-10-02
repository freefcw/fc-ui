use crate::capabilities::data::palette::default_color;
use crate::capabilities::data::pie_chart::PieChartSegment;
use crate::capabilities::foundation::theme::Theme;
use gpui::{div, hsla, prelude::FluentBuilder as _, px, Div, Hsla, ParentElement, Styled};

const START_ANGLE: f32 = -std::f32::consts::FRAC_PI_2;

#[derive(Clone, Copy)]
pub(crate) struct SegmentGeometry {
    pub(crate) start_angle: f32,
    pub(crate) sweep_angle: f32,
    pub(crate) color: Hsla,
}

pub(crate) fn positive_finite_total(segments: &[PieChartSegment]) -> f64 {
    segments
        .iter()
        .map(|segment| segment.value)
        .filter(|value| value.is_finite() && *value > 0.0)
        .sum()
}

pub(crate) fn segment_geometry(segments: &[PieChartSegment], total: f64) -> Vec<SegmentGeometry> {
    if !total.is_finite() || total <= 0.0 {
        return Vec::new();
    }

    let mut current_angle = START_ANGLE;
    let mut geometry = Vec::new();

    for (index, segment) in segments.iter().enumerate() {
        if !segment.value.is_finite() || segment.value <= 0.0 {
            continue;
        }

        let sweep_angle = (segment.value / total) as f32 * std::f32::consts::TAU;
        let color = segment.color.unwrap_or_else(|| default_color(index));
        geometry.push(SegmentGeometry {
            start_angle: current_angle,
            sweep_angle,
            color,
        });
        current_angle += sweep_angle;
    }

    geometry
}

pub(crate) fn color_at_angle(angle: f32, geometry: &[SegmentGeometry]) -> Hsla {
    let normalized_angle = if angle < START_ANGLE {
        angle + std::f32::consts::TAU
    } else {
        angle
    };

    for segment in geometry {
        let end_angle = segment.start_angle + segment.sweep_angle;
        if normalized_angle >= segment.start_angle && normalized_angle < end_angle {
            return segment.color;
        }
    }

    geometry
        .last()
        .map(|segment| segment.color)
        .unwrap_or(hsla(0.0, 0.0, 0.5, 1.0))
}

pub(crate) fn render_legend(
    segments: &[PieChartSegment],
    total: f64,
    show_percentages: bool,
    theme: &Theme,
) -> Div {
    let visible_count = segments
        .iter()
        .filter(|s| s.value.is_finite() && s.value > 0.0)
        .count();
    let mut displayed_total = 0u32;
    let mut visible_index = 0usize;

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .children(segments.iter().enumerate().filter_map(|(index, segment)| {
            if !segment.value.is_finite() || segment.value <= 0.0 {
                return None;
            }

            visible_index += 1;
            let color = segment.color.unwrap_or_else(|| default_color(index));
            let percentage = if total > 0.0 {
                if visible_index == visible_count {
                    100u32.saturating_sub(displayed_total)
                } else {
                    let pct = (segment.value / total * 100.0) as u32;
                    displayed_total += pct;
                    pct
                }
            } else {
                0
            };

            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().size(px(12.0)).rounded(px(2.0)).bg(color))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .justify_between()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.tokens.foreground)
                                    .child(segment.label.clone()),
                            )
                            .when(show_percentages, |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(format!("{}%", percentage)),
                                )
                            }),
                    ),
            )
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(label: &str, value: f64) -> PieChartSegment {
        PieChartSegment {
            label: label.into(),
            value,
            color: None,
        }
    }

    #[test]
    fn geometry_uses_only_positive_finite_segments() {
        let segments = vec![
            segment("first", 2.0),
            segment("invalid", f64::NAN),
            segment("zero", 0.0),
            segment("second", 1.0),
        ];
        let total = positive_finite_total(&segments);
        let geometry = segment_geometry(&segments, total);

        assert_eq!(total, 3.0);
        assert_eq!(geometry.len(), 2);
        assert_eq!(geometry[0].start_angle, START_ANGLE);
        assert!((geometry[0].sweep_angle - std::f32::consts::TAU * 2.0 / 3.0).abs() < f32::EPSILON);
        assert!(
            (geometry[1].start_angle - (START_ANGLE + geometry[0].sweep_angle)).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn color_lookup_wraps_and_uses_the_last_segment_as_a_boundary_fallback() {
        let geometry = segment_geometry(&[segment("first", 1.0), segment("second", 1.0)], 2.0);

        assert_eq!(
            color_at_angle(START_ANGLE + std::f32::consts::TAU + 0.1, &geometry),
            geometry[1].color
        );
        assert_eq!(
            color_at_angle(START_ANGLE - 0.1, &geometry),
            geometry[1].color
        );
    }
}

/// Maps byte offsets between an input's canonical value and display value.
///
/// A formatter does not expose an inverse mapping. The mapping therefore
/// anchors unchanged characters and falls back to monotonic proportional
/// boundaries when the formatter expands, removes, or reorders characters.
pub(crate) struct DisplayMapping;

impl DisplayMapping {
    /// Return display byte offsets for each raw character boundary.
    fn formatted_display_boundaries(raw: &str, display: &str) -> Option<Vec<usize>> {
        let raw_chars: Vec<char> = raw.chars().collect();
        let display_chars: Vec<char> = display.chars().collect();
        if raw_chars.is_empty() {
            return Some(vec![0]);
        }

        let proportional_boundaries = || {
            let raw_len = raw_chars.len() as u128;
            let display_len = display_chars.len() as u128;
            (0..=raw_chars.len())
                .map(|boundary| {
                    let boundary = boundary as u128;
                    ((boundary * display_len) + raw_len / 2) / raw_len
                })
                .map(|char_index| {
                    display_chars
                        .iter()
                        .take(char_index.min(display_chars.len() as u128) as usize)
                        .map(|ch| ch.len_utf8())
                        .sum()
                })
                .collect::<Vec<usize>>()
        };

        let mut anchors = Vec::new();
        let mut display_start = 0;
        for (raw_index, raw_char) in raw_chars.iter().enumerate() {
            let Some(relative) = display_chars[display_start..]
                .iter()
                .position(|display_char| display_char == raw_char)
            else {
                continue;
            };
            let display_index = display_start + relative;
            anchors.push((raw_index, display_index));
            display_start = display_index + 1;
        }

        // One surviving anchor is not enough to infer the direction of a
        // multi-character formatter. Use the monotonic proportional mapping
        // in that case. For a single raw character, the anchor still
        // correctly preserves a prefix or suffix affix.
        if anchors.is_empty() || (raw_chars.len() > 1 && anchors.len() == 1) {
            return Some(proportional_boundaries());
        }

        let mut boundaries = vec![0; raw_chars.len() + 1];
        for (boundary, boundary_value) in boundaries.iter_mut().enumerate().take(raw_chars.len()) {
            if let Some((_, display_index)) =
                anchors.iter().find(|(raw_index, _)| *raw_index == boundary)
            {
                *boundary_value = *display_index;
                continue;
            }
            let previous = anchors
                .iter()
                .rev()
                .find(|(raw_index, _)| *raw_index < boundary);
            let next = anchors.iter().find(|(raw_index, _)| *raw_index > boundary);
            *boundary_value = match (previous, next) {
                (Some((raw_start, display_start)), Some((raw_end, display_end))) => {
                    let raw_span = raw_end - raw_start;
                    display_start
                        + ((boundary - raw_start) * (display_end - display_start) + raw_span / 2)
                            / raw_span
                }
                (None, Some((raw_end, display_end))) => boundary * display_end / (*raw_end).max(1),
                (Some((raw_start, display_start)), None) => {
                    let raw_span = raw_chars.len() - raw_start;
                    display_start
                        + ((boundary - raw_start) * (display_chars.len() - display_start)
                            + raw_span / 2)
                            / raw_span
                }
                (None, None) => 0,
            };
        }
        boundaries[raw_chars.len()] = display_chars.len();
        for index in 1..boundaries.len() {
            boundaries[index] = boundaries[index].max(boundaries[index - 1]);
        }

        Some(
            boundaries
                .into_iter()
                .map(|char_index| {
                    display_chars
                        .iter()
                        .take(char_index.min(display_chars.len()))
                        .map(|ch| ch.len_utf8())
                        .sum()
                })
                .collect(),
        )
    }

    pub(crate) fn raw_to_display_index(raw: &str, display: &str, raw_index: usize) -> usize {
        if raw.is_empty() {
            return 0;
        }

        let mut raw_index = raw_index.min(raw.len());
        while raw_index > 0 && !raw.is_char_boundary(raw_index) {
            raw_index -= 1;
        }
        let raw_char_index = raw[..raw_index].chars().count();
        if let Some(boundaries) = Self::formatted_display_boundaries(raw, display) {
            return boundaries[raw_char_index.min(boundaries.len() - 1)];
        }
        raw_index.min(display.len())
    }

    pub(crate) fn display_to_raw_index(raw: &str, display: &str, display_index: usize) -> usize {
        if raw.is_empty() {
            return 0;
        }

        let mut display_index = display_index.min(display.len());
        while display_index > 0 && !display.is_char_boundary(display_index) {
            display_index -= 1;
        }
        if let Some(boundaries) = Self::formatted_display_boundaries(raw, display) {
            let mut raw_char_index = boundaries.len() - 1;
            for index in 0..boundaries.len() - 1 {
                if display_index <= boundaries[index] {
                    raw_char_index = index;
                    break;
                }
                if display_index <= boundaries[index + 1] {
                    let left = display_index - boundaries[index];
                    let right = boundaries[index + 1] - display_index;
                    raw_char_index = if left > right { index + 1 } else { index };
                    break;
                }
            }
            let raw_char_boundaries: Vec<usize> = raw
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(raw.len()))
                .collect();
            return raw_char_boundaries[raw_char_index.min(raw_char_boundaries.len() - 1)];
        }

        let raw_chars: Vec<char> = raw.chars().collect();
        let display_char_index = display[..display_index].chars().count();
        let raw_char_index = display_char_index * raw_chars.len() / display.chars().count().max(1);
        raw_chars
            .iter()
            .take(raw_char_index.min(raw_chars.len()))
            .map(|ch| ch.len_utf8())
            .sum()
    }
}

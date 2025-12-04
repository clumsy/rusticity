use crate::common::render_vertical_scrollbar;
use ratatui::prelude::*;
use ratatui::widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType, Paragraph};

/// Trait for states that support monitoring with metrics loading
pub trait MonitoringState {
    fn is_metrics_loading(&self) -> bool;
    fn set_metrics_loading(&mut self, loading: bool);
    fn monitoring_scroll(&self) -> usize;
    fn set_monitoring_scroll(&mut self, scroll: usize);
    fn clear_metrics(&mut self);
}

pub struct MetricChart<'a> {
    pub title: &'a str,
    pub data: &'a [(i64, f64)],
    pub y_axis_label: &'a str,
    pub x_axis_label: Option<String>,
}

pub struct MultiDatasetChart<'a> {
    pub title: &'a str,
    pub datasets: Vec<(&'a str, &'a [(i64, f64)])>,
    pub y_axis_label: &'a str,
    pub y_axis_step: u32,
    pub x_axis_label: Option<String>,
}

pub struct DualAxisChart<'a> {
    pub title: &'a str,
    pub left_dataset: (&'a str, &'a [(i64, f64)]),
    pub right_dataset: (&'a str, &'a [(i64, f64)]),
    pub left_y_label: &'a str,
    pub right_y_label: &'a str,
    pub x_axis_label: Option<String>,
}

pub fn render_monitoring_tab(
    frame: &mut Frame,
    area: Rect,
    single_charts: &[MetricChart],
    multi_charts: &[MultiDatasetChart],
    dual_charts: &[DualAxisChart],
    trailing_single_charts: &[MetricChart],
    scroll_position: usize,
) {
    let available_height = area.height as usize;
    let total_charts =
        single_charts.len() + multi_charts.len() + dual_charts.len() + trailing_single_charts.len();

    let mut y_offset = 0;
    let mut chart_idx = 0;

    for chart in single_charts.iter() {
        if chart_idx < scroll_position {
            chart_idx += 1;
            continue;
        }
        if y_offset + 20 > available_height {
            break;
        }
        let chart_height = 20.min((available_height - y_offset) as u16);
        let chart_rect = Rect {
            x: area.x,
            y: area.y + y_offset as u16,
            width: area.width.saturating_sub(1),
            height: chart_height,
        };
        render_chart(frame, chart, chart_rect);
        y_offset += 20;
        chart_idx += 1;
    }

    for chart in multi_charts.iter() {
        if chart_idx < scroll_position {
            chart_idx += 1;
            continue;
        }
        if y_offset + 20 > available_height {
            break;
        }
        let chart_height = 20.min((available_height - y_offset) as u16);
        let chart_rect = Rect {
            x: area.x,
            y: area.y + y_offset as u16,
            width: area.width.saturating_sub(1),
            height: chart_height,
        };
        render_multi_dataset_chart(frame, chart, chart_rect);
        y_offset += 20;
        chart_idx += 1;
    }

    for chart in dual_charts.iter() {
        if chart_idx < scroll_position {
            chart_idx += 1;
            continue;
        }
        if y_offset + 20 > available_height {
            break;
        }
        let chart_height = 20.min((available_height - y_offset) as u16);
        let chart_rect = Rect {
            x: area.x,
            y: area.y + y_offset as u16,
            width: area.width.saturating_sub(1),
            height: chart_height,
        };
        render_dual_axis_chart(frame, chart, chart_rect);
        y_offset += 20;
        chart_idx += 1;
    }

    for chart in trailing_single_charts.iter() {
        if chart_idx < scroll_position {
            chart_idx += 1;
            continue;
        }
        if y_offset + 20 > available_height {
            break;
        }
        let chart_height = 20.min((available_height - y_offset) as u16);
        let chart_rect = Rect {
            x: area.x,
            y: area.y + y_offset as u16,
            width: area.width.saturating_sub(1),
            height: chart_height,
        };
        render_chart(frame, chart, chart_rect);
        y_offset += 20;
        chart_idx += 1;
    }

    let total_height = total_charts * 20;
    let scroll_offset = scroll_position * 20;
    render_vertical_scrollbar(frame, area, total_height, scroll_offset);
}

fn render_chart(frame: &mut Frame, chart: &MetricChart, area: Rect) {
    let block = Block::default()
        .title(format!(" {} ", chart.title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Gray));

    if chart.data.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let paragraph = Paragraph::new("--");
        frame.render_widget(paragraph, inner);
        return;
    }

    let data: Vec<(f64, f64)> = chart
        .data
        .iter()
        .map(|(timestamp, value)| (*timestamp as f64, *value))
        .collect();

    let min_x = data.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = data
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = data
        .iter()
        .map(|(_, y)| *y)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let dataset = Dataset::default()
        .name(chart.title)
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data);

    let x_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let step = 1800;
        let mut current = (min_x as i64 / step) * step;
        while current <= max_x as i64 {
            let time = chrono::DateTime::from_timestamp(current, 0)
                .unwrap_or_default()
                .format("%H:%M")
                .to_string();
            labels.push(Span::raw(time));
            current += step;
        }
        labels
    };

    let mut x_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([min_x, max_x])
        .labels(x_labels);

    if let Some(label) = &chart.x_axis_label {
        x_axis = x_axis.title(label.as_str());
    }

    let y_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let mut current = 0.0;
        let max = max_y * 1.1;
        let step = if max <= 10.0 {
            0.5
        } else {
            (max / 10.0).ceil()
        };
        while current <= max {
            labels.push(Span::raw(format!("{:.1}", current)));
            current += step;
        }
        labels
    };

    let y_axis = Axis::default()
        .title(chart.y_axis_label)
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, max_y * 1.1])
        .labels(y_labels);

    let chart_widget = Chart::new(vec![dataset])
        .block(block)
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart_widget, area);
}

fn render_multi_dataset_chart(frame: &mut Frame, chart: &MultiDatasetChart, area: Rect) {
    let block = Block::default()
        .title(format!(" {} ", chart.title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Gray));

    let all_empty = chart.datasets.iter().all(|(_, data)| data.is_empty());
    if all_empty {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let paragraph = Paragraph::new("--");
        frame.render_widget(paragraph, inner);
        return;
    }

    let colors = [Color::Cyan, Color::Yellow, Color::Magenta];
    let mut converted_data: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = 0.0_f64;

    for (_, data) in chart.datasets.iter() {
        if data.is_empty() {
            converted_data.push(Vec::new());
            continue;
        }
        let converted: Vec<(f64, f64)> = data
            .iter()
            .map(|(timestamp, value)| (*timestamp as f64, *value))
            .collect();

        min_x = min_x.min(
            converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::INFINITY, f64::min),
        );
        max_x = max_x.max(
            converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::NEG_INFINITY, f64::max),
        );
        max_y = max_y.max(converted.iter().map(|(_, y)| *y).fold(0.0_f64, f64::max));

        converted_data.push(converted);
    }

    let mut datasets_vec = Vec::new();
    for (idx, ((name, _), data)) in chart.datasets.iter().zip(converted_data.iter()).enumerate() {
        if data.is_empty() {
            continue;
        }
        let dataset = Dataset::default()
            .name(*name)
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(colors[idx % colors.len()]))
            .data(data);

        datasets_vec.push(dataset);
    }

    max_y = max_y.max(1.0);

    let x_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let step = 1800;
        let mut current = (min_x as i64 / step) * step;
        while current <= max_x as i64 {
            let time = chrono::DateTime::from_timestamp(current, 0)
                .unwrap_or_default()
                .format("%H:%M")
                .to_string();
            labels.push(Span::raw(time));
            current += step;
        }
        labels
    };

    let mut x_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([min_x, max_x])
        .labels(x_labels);

    if let Some(label) = &chart.x_axis_label {
        x_axis = x_axis.title(label.as_str());
    }

    let y_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let mut current = 0.0;
        let max = max_y * 1.1;
        let step = chart.y_axis_step as f64;
        while current <= max {
            let label = if step >= 1000.0 {
                format!("{}K", (current / 1000.0) as u32)
            } else {
                format!("{:.0}", current)
            };
            labels.push(Span::raw(label));
            current += step;
        }
        labels
    };

    let y_axis = Axis::default()
        .title(chart.y_axis_label)
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, max_y * 1.1])
        .labels(y_labels);

    let chart_widget = Chart::new(datasets_vec)
        .block(block)
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart_widget, area);
}

fn render_dual_axis_chart(frame: &mut Frame, chart: &DualAxisChart, area: Rect) {
    let block = Block::default()
        .title(format!(" {} ", chart.title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Gray));

    let (left_name, left_data) = chart.left_dataset;
    let (right_name, right_data) = chart.right_dataset;

    if left_data.is_empty() && right_data.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let paragraph = Paragraph::new("--");
        frame.render_widget(paragraph, inner);
        return;
    }

    let left_converted: Vec<(f64, f64)> = left_data
        .iter()
        .map(|(timestamp, value)| (*timestamp as f64, *value))
        .collect();

    let right_converted: Vec<(f64, f64)> = right_data
        .iter()
        .map(|(timestamp, value)| (*timestamp as f64, *value))
        .collect();

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_left_y = 0.0_f64;
    let max_right_y = 100.0;

    if !left_converted.is_empty() {
        min_x = min_x.min(
            left_converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::INFINITY, f64::min),
        );
        max_x = max_x.max(
            left_converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::NEG_INFINITY, f64::max),
        );
        max_left_y = left_converted
            .iter()
            .map(|(_, y)| *y)
            .fold(0.0_f64, f64::max);
    }

    if !right_converted.is_empty() {
        min_x = min_x.min(
            right_converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::INFINITY, f64::min),
        );
        max_x = max_x.max(
            right_converted
                .iter()
                .map(|(x, _)| *x)
                .fold(f64::NEG_INFINITY, f64::max),
        );
    }

    max_left_y = max_left_y.max(1.0);

    let normalized_right: Vec<(f64, f64)> = right_converted
        .iter()
        .map(|(x, y)| (*x, y * max_left_y / max_right_y))
        .collect();

    let left_dataset = Dataset::default()
        .name(left_name)
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Red))
        .data(&left_converted);

    let right_dataset = Dataset::default()
        .name(right_name)
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&normalized_right);

    let x_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let step = 1800;
        let mut current = (min_x as i64 / step) * step;
        while current <= max_x as i64 {
            let time = chrono::DateTime::from_timestamp(current, 0)
                .unwrap_or_default()
                .format("%H:%M")
                .to_string();
            labels.push(Span::raw(time));
            current += step;
        }
        labels
    };

    let mut x_axis = Axis::default()
        .style(Style::default().fg(Color::Gray))
        .bounds([min_x, max_x])
        .labels(x_labels);

    if let Some(label) = &chart.x_axis_label {
        x_axis = x_axis.title(label.as_str());
    }

    let y_labels: Vec<Span> = {
        let mut labels = Vec::new();
        let mut current = 0.0;
        let max = max_left_y * 1.1;
        let step = if max <= 10.0 {
            0.5
        } else {
            (max / 10.0).ceil()
        };
        while current <= max {
            labels.push(Span::raw(format!("{:.0}", current)));
            current += step;
        }
        labels
    };

    let y_axis = Axis::default()
        .title(chart.left_y_label)
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, max_left_y * 1.1])
        .labels(y_labels);

    let chart_widget = Chart::new(vec![left_dataset, right_dataset])
        .block(block)
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart_widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_chart_creation() {
        let data = vec![(1700000000, 5.0), (1700000060, 10.0)];
        let chart = MetricChart {
            title: "Test Metric",
            data: &data,
            y_axis_label: "Count",
            x_axis_label: None,
        };
        assert_eq!(chart.title, "Test Metric");
        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.y_axis_label, "Count");
        assert_eq!(chart.x_axis_label, None);
    }

    #[test]
    fn test_empty_chart_data() {
        let data: Vec<(i64, f64)> = vec![];
        let chart = MetricChart {
            title: "Empty Chart",
            data: &data,
            y_axis_label: "Value",
            x_axis_label: None,
        };
        assert!(chart.data.is_empty());
    }

    #[test]
    fn test_metric_chart_with_x_axis_label() {
        let data = vec![(1700000000, 5.0), (1700000060, 10.0)];
        let chart = MetricChart {
            title: "Invocations",
            data: &data,
            y_axis_label: "Count",
            x_axis_label: Some("Invocations [sum: 15]".to_string()),
        };
        assert_eq!(
            chart.x_axis_label,
            Some("Invocations [sum: 15]".to_string())
        );
    }

    #[test]
    fn test_multi_dataset_chart_creation() {
        let min_data = vec![(1700000000, 100.0), (1700000060, 150.0)];
        let avg_data = vec![(1700000000, 200.0), (1700000060, 250.0)];
        let max_data = vec![(1700000000, 300.0), (1700000060, 350.0)];

        let chart = MultiDatasetChart {
            title: "Duration",
            datasets: vec![
                ("Minimum", &min_data),
                ("Average", &avg_data),
                ("Maximum", &max_data),
            ],
            y_axis_label: "Milliseconds",
            y_axis_step: 1000,
            x_axis_label: Some("Minimum [100], Average [200], Maximum [300]".to_string()),
        };

        assert_eq!(chart.title, "Duration");
        assert_eq!(chart.datasets.len(), 3);
        assert_eq!(chart.y_axis_label, "Milliseconds");
        assert_eq!(chart.y_axis_step, 1000);
    }

    #[test]
    fn test_multi_dataset_chart_empty() {
        let empty: Vec<(i64, f64)> = vec![];
        let chart = MultiDatasetChart {
            title: "Empty Duration",
            datasets: vec![
                ("Minimum", &empty),
                ("Average", &empty),
                ("Maximum", &empty),
            ],
            y_axis_label: "Milliseconds",
            y_axis_step: 1000,
            x_axis_label: None,
        };

        assert!(chart.datasets.iter().all(|(_, data)| data.is_empty()));
    }

    #[test]
    fn test_duration_label_format() {
        let min = 100.0;
        let avg = 200.5;
        let max = 350.0;
        let label = format!(
            "Minimum [{:.0}], Average [{:.0}], Maximum [{:.0}]",
            min, avg, max
        );
        assert_eq!(label, "Minimum [100], Average [200], Maximum [350]");
    }

    #[test]
    fn test_y_axis_label_formatting_1k() {
        let value = 1000.0;
        let label = format!("{}K", (value / 1000.0) as u32);
        assert_eq!(label, "1K");
    }

    #[test]
    fn test_y_axis_label_formatting_5k() {
        let value = 5000.0;
        let label = format!("{}K", (value / 1000.0) as u32);
        assert_eq!(label, "5K");
    }

    #[test]
    fn test_y_axis_step_1000() {
        let step = 1000;
        assert_eq!(step, 1000);
        let values = [0, 1000, 2000, 3000, 4000, 5000];
        for (i, val) in values.iter().enumerate() {
            assert_eq!(*val, i * step);
        }
    }

    #[test]
    fn test_duration_min_calculation() {
        let data = [(1700000000, 100.0), (1700000060, 50.0), (1700000120, 75.0)];
        let min: f64 = data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        assert_eq!(min, 50.0);
    }

    #[test]
    fn test_duration_avg_calculation() {
        let data = [
            (1700000000, 100.0),
            (1700000060, 200.0),
            (1700000120, 300.0),
        ];
        let avg: f64 = data.iter().map(|(_, v)| v).sum::<f64>() / data.len() as f64;
        assert_eq!(avg, 200.0);
    }

    #[test]
    fn test_duration_max_calculation() {
        let data = [
            (1700000000, 100.0),
            (1700000060, 350.0),
            (1700000120, 200.0),
        ];
        let max: f64 = data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        assert_eq!(max, 350.0);
    }

    #[test]
    fn test_duration_empty_data_min() {
        let data: Vec<(i64, f64)> = vec![];
        let min: f64 = data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        assert!(min.is_infinite() && min.is_sign_positive());
    }

    #[test]
    fn test_duration_empty_data_avg() {
        let data: Vec<(i64, f64)> = vec![];
        let avg: f64 = if !data.is_empty() {
            data.iter().map(|(_, v)| v).sum::<f64>() / data.len() as f64
        } else {
            0.0
        };
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_dual_axis_chart_creation() {
        let errors = vec![(1700000000, 5.0), (1700000060, 10.0)];
        let success_rate = vec![(1700000000, 95.0), (1700000060, 90.0)];

        let chart = DualAxisChart {
            title: "Error count and success rate",
            left_dataset: ("Errors", &errors),
            right_dataset: ("Success rate", &success_rate),
            left_y_label: "Count",
            right_y_label: "%",
            x_axis_label: Some("Errors [max: 10] and Success rate [min: 90%]".to_string()),
        };

        assert_eq!(chart.title, "Error count and success rate");
        assert_eq!(chart.left_y_label, "Count");
        assert_eq!(chart.right_y_label, "%");
    }

    #[test]
    fn test_dual_axis_normalization() {
        let max_left_y = 10.0;
        let max_right_y = 100.0;
        let right_value = 95.0;
        let normalized = right_value * max_left_y / max_right_y;
        assert_eq!(normalized, 9.5);
    }
}

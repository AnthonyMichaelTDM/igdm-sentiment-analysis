use std::{collections::HashMap, fmt::Display, path::PathBuf};

use anyhow::Result;
use chrono::TimeDelta;
use plotters::prelude::*;

use crate::{
    analyzer::{AnalyzedConversation, Score},
    parser::Participant,
};

const SHOW_SMOOTHED: bool = true;
const SHOW_LSQR: bool = true;
const REMOVE_OUTLIERS: bool = true;

#[derive(Debug, Default, Clone, Copy)]
pub enum PlotType {
    Positive,
    Negative,
    Neutral,
    #[default]
    Compound,
}

impl Display for PlotType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Positive => "positive",
                Self::Negative => "negative",
                Self::Neutral => "neutral",
                Self::Compound => "compound",
            }
        )
    }
}

impl AnalyzedConversation {
    pub fn plot(&self, plot_type: PlotType, output_file: &PathBuf) -> Result<()> {
        // first, we need to extract the data we want to plot
        // data should be HashMap<Participant, Vec<(timestamp, score)>>
        let data = extract_data(self, plot_type);

        let min_time: usize = *data
            .values()
            .flat_map(|v| v.iter().map(|(t, _)| t))
            .min()
            .unwrap();
        let max_time: usize = *data
            .values()
            .flat_map(|v| v.iter().map(|(t, _)| t))
            .max()
            .unwrap();
        let (min_score, max_score) = match plot_type {
            PlotType::Neutral | PlotType::Positive | PlotType::Negative => (0.0, 1.0),
            PlotType::Compound => (-1.0, 1.0),
        };

        // plot the data with the plotters crate
        let root = BitMapBackend::new(&output_file, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        let root = root.margin(10, 10, 10, 10);
        // construct the chart context
        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("Sentiment Analysis ({plot_type})"),
                ("sans-serif", 30).into_font(),
            )
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(40)
            .build_cartesian_2d(min_time..max_time, min_score..max_score)?; //min_score..=max_score)?;

        // draw the mesh
        chart
            .configure_mesh()
            // customize the x labels
            .x_desc("Time")
            // display the x labels as datetimes, currently is a timestamp in milliseconds
            .x_label_formatter(&|t| {
                format!(
                    "{}",
                    chrono::DateTime::from_timestamp(*t as i64 / 1000, 0).unwrap()
                )
            })
            // customize the y labels
            .y_desc("Score")
            // display the y labels as percentages
            .y_label_formatter(&&|s: &f64| format!("{:.0}%", s * 100.0))
            .draw()?;

        // draw the data, give each participant a different color
        for (i, (participant, scores)) in data.iter().enumerate() {
            // pick a color from the palette, and use it for the line
            let mut style = Palette99::pick(i).to_rgba();
            style.3 = 0.3; // set the alpha channel to 0.5 to make the line transparent

            // draw the data points
            chart
                .draw_series(PointSeries::of_element(
                    scores.iter().map(|(t, s)| (*t, *s)),
                    1,
                    style.filled(),
                    &|c, s, st| {
                        EmptyElement::at(c)    // We want to put the point at the position of (x, y)
                    + Circle::new((0,0),s,st.filled()) // And a circle of (2*radius, color)
                    },
                ))?
                .label(participant.name.clone())
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], style));
        }

        // add a legend to the plot
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft)
            .draw()?;

        // now, with a thicker line, let's draw a smoothed version of the data on top of the original data
        if SHOW_SMOOTHED {
            for (i, (_, scores)) in data.iter().enumerate() {
                // pick a color from the palette, and use it for the line
                let mut color = Palette99::pick(i).to_rgba();
                color.3 = 0.8; // set the alpha channel to 0.8 to make the line more visible

                // draw the smoothed line
                chart.draw_series(DashedLineSeries::new(
                    smoothen_wrt_time(
                        scores,
                        TimeDelta::milliseconds((max_time as i64 - min_time as i64) / 100),
                        // TimeDelta::days(3),
                    ),
                    6,
                    2,
                    color.stroke_width(2),
                ))?;
            }
        }

        // now, with an even thicker line, let's draw a least squares linear regression of the data on top of the original data
        if SHOW_LSQR {
            for (i, (_, scores)) in data.iter().enumerate() {
                // pick a color from the palette, and use it for the line
                let color = Palette99::pick(i).to_rgba();

                // draw the least squares linear regression line
                chart.draw_series(LineSeries::new(
                    least_squares_linear_regression(scores),
                    color.stroke_width(2),
                ))?;
            }
        }
        // save the plot to the output file
        root.present()?;

        Ok(())
    }
}

fn extract_data(
    analysis: &AnalyzedConversation,
    plot_type: PlotType,
) -> HashMap<Participant, Vec<(usize, f64)>> {
    analysis
        .analysis
        .iter()
        .map(|(participant, messages)| {
            // remove outliers from the data
            let filter = if REMOVE_OUTLIERS {
                |score| {
                    score
                        != Score {
                            pos: 0.0,
                            neu: 1.0,
                            neg: 0.0,
                            compound: 0.0,
                        }
                }
            } else {
                |_| true
            };

            (
                participant.clone(),
                messages
                    .iter()
                    .filter(|(_, score)| filter(*score))
                    .map(|(message, score)| {
                        (
                            message.timestamp_ms,
                            match plot_type {
                                PlotType::Positive => score.pos,
                                PlotType::Negative => score.neg,
                                PlotType::Neutral => score.neu,
                                PlotType::Compound => score.compound,
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

/// Smoothens the given data (timestamp, score) by averaging scores within a window of `window_size`,
/// data is assumed to be sorted by timestamp in ascending order.
fn smoothen_wrt_time(data: &[(usize, f64)], window_size: TimeDelta) -> Vec<(usize, f64)> {
    let window_size = window_size.num_milliseconds() as usize;
    let mut smoothed_scores = Vec::new();
    let mut window_start = data[0].0;
    let mut window_sum = 0.0;
    let mut window_count = 0;
    for (time, score) in data {
        if window_count > 0 && time - window_start >= window_size {
            smoothed_scores.push((window_start, window_sum / window_count as f64));
            window_start = *time;
            window_sum = 0.0;
            window_count = 0;
        } else {
            window_sum += score;
            window_count += 1;
        }
    }
    if window_count > 0 {
        smoothed_scores.push((window_start, window_sum / window_count as f64));
    }
    smoothed_scores
}

/// Calculates the least squares linear regression of the given data (timestamp, score),
fn least_squares_linear_regression(data: &[(usize, f64)]) -> Vec<(usize, f64)> {
    let x = data.iter().map(|(t, _)| *t as f64);
    let y = data.iter().map(|(_, s)| *s);
    let n = x.len() as f64;

    let (sum_x, sum_y, sum_x_squared, sum_xy) = x.clone().zip(y).fold(
        (0.0, 0.0, 0.0, 0.0),
        |(sum_x, sum_y, sum_x_squared, sum_xy), (x, y)| {
            (
                sum_x + x,
                sum_y + y,
                x.mul_add(x, sum_x_squared),
                x.mul_add(y, sum_xy),
            )
        },
    );

    let m = n.mul_add(sum_xy, -(sum_x * sum_y)) / n.mul_add(sum_x_squared, -(sum_x * sum_x));
    let b = m.mul_add(-sum_x, sum_y) / n;
    x.map(|x| (x as usize, x.mul_add(m, b))).collect()
}

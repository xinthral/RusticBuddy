//! @file ui.rs
//! @brief Immutable-render layer: draws each ::crate::app::Screen from ::crate::app::App state.
//!
//! Every function here reads ::crate::app::App and writes to a ratatui
//! `Frame`; none of them mutate application state (list-selection state is
//! the sole exception, passed as `&mut` only because ratatui's stateful
//! widgets require it to track scroll position).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Screen};
use crate::models::{ExerciseType, Ingredient};

/// @brief Top-level draw entry point: lays out header/body/footer and dispatches the body to the handler for the current screen.
/// @param frame The ratatui frame to draw into for this tick.
/// @param app Current application state; mutated only via its `ListState` fields for scroll tracking.
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);

    match app.screen {
        Screen::Dashboard => render_dashboard(frame, app, chunks[1]),
        Screen::WorkoutList => render_workout_list(frame, app, chunks[1]),
        Screen::WorkoutDetail => render_workout_detail(frame, app, chunks[1]),
        Screen::AddWorkout
        | Screen::AddExercise
        | Screen::LogMetrics
        | Screen::AddToLibrary
        | Screen::AddRecipe
        | Screen::AddRecipeIngredient
        | Screen::AddIngredient
        | Screen::SetIngredientStock => {
            render_form(frame, app, chunks[1]);
        }
        Screen::ExerciseTypeSelect => render_type_select(frame, app, chunks[1]),
        Screen::MetricsHistory => render_metrics_history(frame, app, chunks[1]),
        Screen::Reports => render_reports(frame, app, chunks[1]),
        Screen::PickExercise => render_pick_exercise(frame, app, chunks[1]),
        Screen::RecipeList => render_recipe_list(frame, app, chunks[1]),
        Screen::RecipeDetail => render_recipe_detail(frame, app, chunks[1]),
        Screen::PickIngredient => render_pick_ingredient(frame, app, chunks[1]),
        Screen::IngredientLibrary => render_ingredient_library(frame, app, chunks[1]),
        Screen::MealHistory => render_meal_history(frame, app, chunks[1]),
        Screen::Recipebook => render_recipebook(frame, app, chunks[1]),
    }

    render_footer(frame, app, chunks[2]);
}

/// @brief Draws the title bar naming the current screen.
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.screen`).
/// @param area Rectangle allotted to the header.
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.screen {
        Screen::Dashboard => " Rustic Buddy  —  Dashboard ",
        Screen::WorkoutList => " Rustic Buddy  —  Workouts ",
        Screen::WorkoutDetail => " Rustic Buddy  —  Workout Detail ",
        Screen::AddWorkout => " Rustic Buddy  —  New Workout ",
        Screen::AddExercise => " Rustic Buddy  —  Add Exercise ",
        Screen::ExerciseTypeSelect => " Rustic Buddy  —  Exercise Type ",
        Screen::LogMetrics => " Rustic Buddy  —  Log Metrics ",
        Screen::MetricsHistory => " Rustic Buddy  —  Metrics History ",
        Screen::Reports => " Rustic Buddy  —  Reports ",
        Screen::PickExercise => " Rustic Buddy  —  Select Exercise ",
        Screen::AddToLibrary => " Rustic Buddy  —  Add to Library ",
        Screen::RecipeList => " Rustic Buddy  —  Recipes ",
        Screen::RecipeDetail => " Rustic Buddy  —  Recipe Detail ",
        Screen::AddRecipe => " Rustic Buddy  —  New Recipe ",
        Screen::PickIngredient => " Rustic Buddy  —  Select Ingredient ",
        Screen::AddRecipeIngredient => " Rustic Buddy  —  Ingredient Amount ",
        Screen::IngredientLibrary => " Rustic Buddy  —  Ingredients ",
        Screen::AddIngredient => " Rustic Buddy  —  Add Ingredient ",
        Screen::SetIngredientStock => " Rustic Buddy  —  Set Stock ",
        Screen::MealHistory => " Rustic Buddy  —  Meal Log ",
        Screen::Recipebook => " Rustic Buddy  —  Recipebook ",
    };
    let para = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(para, area);
}

/// @brief Draws the keybinding hint bar appropriate to the current screen.
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.screen`).
/// @param area Rectangle allotted to the footer.
fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints = match app.screen {
        Screen::Dashboard => "[↑↓] Navigate  [Enter/→] Open  [1-5] Jump  [q] Quit",
        Screen::Recipebook => "[↑↓] Navigate  [Enter/→] Open  [1-3] Jump  [←/Esc/q] Back",
        Screen::WorkoutList => "[↑↓] Navigate  [Enter/→] Open  [a] Set Active  [n] New  [d] Delete  [←/Esc/q] Back",
        Screen::WorkoutDetail => "[↑↓] Navigate  [n] Add  [l] Log session  [d] Delete  [←/Esc/q] Back",
        Screen::AddWorkout
        | Screen::AddExercise
        | Screen::LogMetrics
        | Screen::AddToLibrary
        | Screen::AddRecipe
        | Screen::AddRecipeIngredient
        | Screen::AddIngredient
        | Screen::SetIngredientStock => {
            "[Tab/Enter] Next Field  [Shift+Tab] Prev  [Enter] Submit on last field  [Esc] Cancel"
        }
        Screen::ExerciseTypeSelect => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::PickExercise => "[↑↓] Navigate  [Enter/→] Select  [n] Add to library  [←/Esc/q] Back",
        Screen::MetricsHistory => "[↑↓] Navigate  [d] Delete  [←/Esc/q] Back",
        Screen::Reports => "[←/Esc/q] Back",
        Screen::RecipeList => "[↑↓] Navigate  [Enter/→] Open  [n] New  [d] Delete  [←/Esc/q] Back",
        Screen::RecipeDetail => "[↑↓] Navigate  [n] Add Ingredient  [l] Log Meal  [d] Remove Ingredient  [←/Esc/q] Back",
        Screen::PickIngredient => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::IngredientLibrary => "[↑↓] Navigate  [n] New  [s] Set Stock  [d] Delete  [←/Esc/q] Back",
        Screen::MealHistory => "[↑↓] Navigate  [d] Delete  [←/Esc/q] Back",
    };
    let para = Paragraph::new(hints)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}

/// @brief Draws the dashboard: the main menu on the left, active workout plan in the middle, latest personal metrics on the right.
/// @param frame Frame to draw into.
/// @param app Current application state; `dashboard_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the dashboard body.
fn render_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(39), Constraint::Percentage(39)])
        .split(area);

    let menu_items: Vec<ListItem> = ["Workouts", "Log Metrics", "History", "Reports", "Recipebook"]
        .iter()
        .map(|label| ListItem::new(format!("  {label}")))
        .collect();
    let menu = List::new(menu_items)
        .block(Block::default().borders(Borders::ALL).title(" Menu "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(menu, columns[0], &mut app.dashboard_list_state);

    let halves = [columns[1], columns[2]];

    // Middle — active workout
    let workout_lines = if let Some(plan) = app.active_plan() {
        let mut lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    plan.name.as_str(),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::raw(""),
        ];
        if app.active_exercises.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No exercises yet. Press [1/w] → [Enter] → [n]",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for ex in &app.active_exercises {
                let line = match ex.exercise_type {
                    ExerciseType::Strength => {
                        let weight = ex.weight.map(|w| format!(" @ {w} kg")).unwrap_or_default();
                        format!("  {}×{}  {}{}", ex.sets, ex.reps, ex.name, weight)
                    }
                    ExerciseType::Cardio => {
                        let dur = ex.duration_min
                            .map(|d| format!("{d} min"))
                            .unwrap_or_else(|| "—".into());
                        let dist = ex.distance_km
                            .map(|d| format!("  · {d:.2} km"))
                            .unwrap_or_default();
                        format!("  {}  {}{}", dur, ex.name, dist)
                    }
                };
                lines.push(Line::from(line));
            }
        }
        lines
    } else {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  No active workout set.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::raw("  Go to Workouts [1/w] and"),
            Line::raw("  press [a] to set one active."),
        ]
    };

    let workout_block = Block::default()
        .borders(Borders::ALL)
        .title(" Active Workout ")
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(
        Paragraph::new(workout_lines).block(workout_block).wrap(Wrap { trim: false }),
        halves[0],
    );

    // Right — latest metrics
    let metric_lines = if let Some(m) = app.metrics.first() {
        let weight = m.weight_kg.map(|v| format!("{v} kg")).unwrap_or_else(|| "—".into());
        let hr = m.heart_rate.map(|v| format!("{v} bpm")).unwrap_or_else(|| "—".into());
        let bp = match (m.bp_systolic, m.bp_diastolic) {
            (Some(s), Some(d)) => format!("{s}/{d} mmHg"),
            _ => "—".into(),
        };
        let date = m.recorded_at.get(..16).unwrap_or(&m.recorded_at);
        vec![
            Line::from(vec![
                Span::raw("  Recorded: "),
                Span::styled(date, Style::default().fg(Color::DarkGray)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Weight:         "),
                Span::styled(weight, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw("  Heart Rate:     "),
                Span::styled(hr, Style::default().fg(Color::Red)),
            ]),
            Line::from(vec![
                Span::raw("  Blood Pressure: "),
                Span::styled(bp, Style::default().fg(Color::Magenta)),
            ]),
        ]
    } else {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  No metrics logged yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::raw("  Press [2/m] to log your"),
            Line::raw("  first measurement."),
        ]
    };

    let metrics_block = Block::default()
        .borders(Borders::ALL)
        .title(" Latest Metrics ")
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(metric_lines).block(metrics_block).wrap(Wrap { trim: false }),
        halves[1],
    );
}

/// @brief Draws the Recipebook submenu: Recipes, Ingredients, and Meal Log.
/// @param frame Frame to draw into.
/// @param app Current application state; `recipebook_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the menu.
fn render_recipebook(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = ["Recipes", "Ingredients", "Meal Log"]
        .iter()
        .map(|label| ListItem::new(format!("  {label}")))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Recipebook "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.recipebook_list_state);
}

/// @brief Draws the list of workout plans, marking the active one with a star.
/// @param frame Frame to draw into.
/// @param app Current application state; `workout_list_state` is used (and its scroll position implicitly tracked) by the stateful list widget.
/// @param area Rectangle allotted to the list.
fn render_workout_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .workout_plans
        .iter()
        .map(|p| {
            let marker = if p.is_active { "★ " } else { "  " };
            let style = if p.is_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{}", marker, p.name)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Workout Plans  (★ = active) "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.workout_list_state);
}

/// @brief Draws the exercise list for the workout plan open in ::crate::app::App::detail_plan_id.
/// @param frame Frame to draw into.
/// @param app Current application state; `exercise_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the list.
fn render_workout_detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let plan_name = app
        .detail_plan_id
        .and_then(|id| app.workout_plans.iter().find(|p| p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Workout".into());

    let items: Vec<ListItem> = app
        .current_exercises
        .iter()
        .map(|e| {
            let display = match e.exercise_type {
                ExerciseType::Strength => {
                    let weight = e.weight.map(|w| format!(" @ {w} kg")).unwrap_or_default();
                    format!("  {}×{}  {}{}", e.sets, e.reps, e.name, weight)
                }
                ExerciseType::Cardio => {
                    let dur = e.duration_min
                        .map(|d| format!("{d} min"))
                        .unwrap_or_else(|| "—".into());
                    let dist = e.distance_km
                        .map(|d| format!("  · {d:.2} km"))
                        .unwrap_or_default();
                    format!("  {}  {}{}", dur, e.name, dist)
                }
            };
            ListItem::new(display)
        })
        .collect();

    let title = format!(" {} ", plan_name);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.exercise_list_state);
}

/// @brief Draws a generic labeled-text-field form, used for all of ::crate::app::Screen's Add*/Log* screens.
///
/// Field count and labels come from `app.input_fields`/`app.input_labels`;
/// the focused field (`app.input_index`) is highlighted with a cyan border
/// and a block cursor. Renders an error box below the fields if `app.status_msg` is set.
///
/// @param frame Frame to draw into.
/// @param app Current application state.
/// @param area Rectangle allotted to the form.
fn render_form(frame: &mut Frame, app: &App, area: Rect) {
    let n = app.input_fields.len();

    let block = Block::default().borders(Borders::ALL).title(" Fill in the fields ");
    frame.render_widget(block, area);

    let mut constraints: Vec<Constraint> = vec![Constraint::Length(1)];
    for _ in 0..n {
        constraints.push(Constraint::Length(3));
    }
    if app.status_msg.is_some() {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(0));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    for (i, (field, label)) in app
        .input_fields
        .iter()
        .zip(app.input_labels.iter())
        .enumerate()
    {
        let focused = i == app.input_index;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cursor = if focused { "█" } else { "" };
        let para = Paragraph::new(format!("{}{}", field, cursor))
            .style(Style::default().fg(if focused { Color::White } else { Color::Gray }))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(*label)
                    .border_style(border_style),
            );
        frame.render_widget(para, inner[i + 1]);
    }

    if let Some(msg) = &app.status_msg {
        let err = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" Error "));
        frame.render_widget(err, inner[n + 1]);
    }
}

/// @brief Draws the Strength/Cardio picker shown between choosing an exercise name and filling in its form.
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.type_select_index`).
/// @param area Rectangle allotted to the picker.
fn render_type_select(frame: &mut Frame, app: &App, area: Rect) {
    let options = [
        "  Strength  (sets · reps · weight)",
        "  Cardio    (duration · distance)",
    ];

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = if i == app.type_select_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(*label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Choose Exercise Type "));
    frame.render_widget(list, area);
}

/// @brief Draws the exercise-library picker, appending a "Custom name..." entry after the library items.
/// @param frame Frame to draw into.
/// @param app Current application state; `library_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the picker.
fn render_pick_exercise(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = app
        .library_exercises
        .iter()
        .map(|name| ListItem::new(format!("  {}", name)))
        .collect();

    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  Library is empty — press [n] to add your first exercise",
            Style::default().fg(Color::DarkGray),
        )));
    }

    items.push(ListItem::new(Span::styled(
        "  [ Custom name... ]",
        Style::default().fg(Color::DarkGray),
    )));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Exercise Library "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.library_list_state);
}

/// @brief Draws the two-pane Reports screen: biometric trends on the left, exercise trends on the right.
/// @param frame Frame to draw into.
/// @param app Current application state.
/// @param area Rectangle allotted to the reports body.
fn render_reports(frame: &mut Frame, app: &App, area: Rect) {
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_biometric_trends(frame, app, halves[0]);
    render_exercise_trends(frame, app, halves[1]);
}

/// @brief Draws weight/heart-rate/blood-pressure deltas between the oldest and newest logged ::crate::models::PersonalMetric.
///
/// Shows a placeholder message if there are fewer than 2 readings; a metric
/// missing from either the oldest or newest reading is shown as "—" rather
/// than computing a partial delta.
///
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.metrics`).
/// @param area Rectangle allotted to this panel.
fn render_biometric_trends(frame: &mut Frame, app: &App, area: Rect) {
    let metrics = &app.metrics;

    let lines = if metrics.is_empty() {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  No metrics logged yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::raw("  Press [2/m] from the dashboard to log your first entry."),
        ]
    } else if metrics.len() < 2 {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Need at least 2 entries to show trends.",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        let newest = &metrics[0];
        let oldest = &metrics[metrics.len() - 1];
        let count = metrics.len();
        let date_new = newest.recorded_at.get(..10).unwrap_or(&newest.recorded_at);
        let date_old = oldest.recorded_at.get(..10).unwrap_or(&oldest.recorded_at);

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                format!("  {} entries  ({} → {})", count, date_old, date_new),
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
        ];

        // Weight
        match (oldest.weight_kg, newest.weight_kg) {
            (Some(old), Some(new)) => {
                let delta = new - old;
                let (arrow, color) = float_trend(delta);
                lines.push(Line::from(vec![
                    Span::raw("  Weight      "),
                    Span::styled(format!("{:.1}", old), Style::default().fg(Color::DarkGray)),
                    Span::raw(" → "),
                    Span::styled(format!("{:.1} kg", new), Style::default().fg(Color::Yellow)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("               "),
                    Span::styled(format!("{} {:+.1} kg", arrow, delta), Style::default().fg(color)),
                ]));
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "  Weight      —",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        lines.push(Line::raw(""));

        // Heart rate
        match (oldest.heart_rate, newest.heart_rate) {
            (Some(old), Some(new)) => {
                let delta = new - old;
                let (arrow, color) = int_trend(delta);
                lines.push(Line::from(vec![
                    Span::raw("  Heart Rate  "),
                    Span::styled(format!("{}", old), Style::default().fg(Color::DarkGray)),
                    Span::raw(" → "),
                    Span::styled(format!("{} bpm", new), Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("               "),
                    Span::styled(format!("{} {:+} bpm", arrow, delta), Style::default().fg(color)),
                ]));
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "  Heart Rate  —",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        lines.push(Line::raw(""));

        // Blood pressure
        match (oldest.bp_systolic, oldest.bp_diastolic, newest.bp_systolic, newest.bp_diastolic) {
            (Some(os), Some(od), Some(ns), Some(nd)) => {
                let ds = ns - os;
                let dd = nd - od;
                let (arrow, color) = int_trend(ds);
                lines.push(Line::from(vec![
                    Span::raw("  Blood Press "),
                    Span::styled(format!("{}/{}", os, od), Style::default().fg(Color::DarkGray)),
                    Span::raw(" → "),
                    Span::styled(format!("{}/{} mmHg", ns, nd), Style::default().fg(Color::Magenta)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("               "),
                    Span::styled(
                        format!("{} {:+}/{:+}", arrow, ds, dd),
                        Style::default().fg(color),
                    ),
                ]));
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "  Blood Press —",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        lines
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Biometric Trends ")
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

/// @brief Picks a directional arrow glyph and highlight color for a floating-point delta, with a small dead zone around zero.
/// @param delta Change in value (new − old).
/// @return `("↑", Color::Cyan)` if `delta > 0.05`, `("↓", Color::Cyan)` if `delta < -0.05`, otherwise `("→", Color::DarkGray)`.
fn float_trend(delta: f64) -> (&'static str, Color) {
    if delta > 0.05 { ("↑", Color::Cyan) }
    else if delta < -0.05 { ("↓", Color::Cyan) }
    else { ("→", Color::DarkGray) }
}

/// @brief Picks a directional arrow glyph and highlight color for an integer delta.
/// @param delta Change in value (new − old).
/// @return `("↑", Color::Cyan)` if positive, `("↓", Color::Cyan)` if negative, otherwise `("→", Color::DarkGray)`.
fn int_trend(delta: i32) -> (&'static str, Color) {
    if delta > 0 { ("↑", Color::Cyan) }
    else if delta < 0 { ("↓", Color::Cyan) }
    else { ("→", Color::DarkGray) }
}

/// @brief Draws per-exercise trend summaries, grouping consecutive log entries by exercise name.
///
/// For each exercise, shows the session count and the change between its
/// first and most recent logged session (sets/reps/weight for strength,
/// duration/distance for cardio). Relies on `app.exercise_logs` already
/// being sorted by `(exercise_name, logged_at)` so entries for the same
/// exercise are contiguous.
///
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.exercise_logs`).
/// @param area Rectangle allotted to this panel.
fn render_exercise_trends(frame: &mut Frame, app: &App, area: Rect) {
    let logs = &app.exercise_logs;

    let lines = if logs.is_empty() {
        vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  No sessions logged yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::raw("  Open a workout, then press"),
            Line::raw("  [l] to log a session."),
        ]
    } else {
        let mut lines: Vec<Line> = vec![];
        let mut i = 0;
        while i < logs.len() {
            let name = &logs[i].exercise_name;
            let first = &logs[i];
            let mut last = first;
            let mut count = 0usize;
            while i < logs.len() && &logs[i].exercise_name == name {
                last = &logs[i];
                count += 1;
                i += 1;
            }

            let session_label = if count == 1 { "1 session".to_string() } else { format!("{} sessions", count) };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}", name),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", session_label), Style::default().fg(Color::DarkGray)),
            ]));

            match first.exercise_type {
                ExerciseType::Strength => {
                    // Sets
                    let sets_delta = last.sets - first.sets;
                    let (sets_arrow, _) = int_trend(sets_delta);
                    if count > 1 && sets_delta != 0 {
                        lines.push(Line::from(vec![
                            Span::raw("  Sets   "),
                            Span::styled(first.sets.to_string(), Style::default().fg(Color::DarkGray)),
                            Span::raw(" → "),
                            Span::styled(format!("{} {}", last.sets, sets_arrow), Style::default().fg(Color::Green)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  Sets   "),
                            Span::styled(last.sets.to_string(), Style::default().fg(Color::Green)),
                        ]));
                    }

                    // Reps
                    let reps_delta = last.reps - first.reps;
                    let (reps_arrow, _) = int_trend(reps_delta);
                    if count > 1 && reps_delta != 0 {
                        lines.push(Line::from(vec![
                            Span::raw("  Reps   "),
                            Span::styled(first.reps.to_string(), Style::default().fg(Color::DarkGray)),
                            Span::raw(" → "),
                            Span::styled(format!("{} {}", last.reps, reps_arrow), Style::default().fg(Color::Green)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  Reps   "),
                            Span::styled(last.reps.to_string(), Style::default().fg(Color::Green)),
                        ]));
                    }

                    // Weight
                    match (first.weight, last.weight) {
                        (Some(fw), Some(lw)) if count > 1 => {
                            let delta = lw - fw;
                            let (arrow, color) = float_trend(delta);
                            lines.push(Line::from(vec![
                                Span::raw("  Weight "),
                                Span::styled(format!("{:.1}", fw), Style::default().fg(Color::DarkGray)),
                                Span::raw(" → "),
                                Span::styled(format!("{:.1} kg", lw), Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(format!("{} {:+.1}", arrow, delta), Style::default().fg(color)),
                            ]));
                        }
                        (Some(w), _) => {
                            lines.push(Line::from(vec![
                                Span::raw("  Weight "),
                                Span::styled(format!("{:.1} kg", w), Style::default().fg(Color::Cyan)),
                            ]));
                        }
                        _ => {}
                    }
                }

                ExerciseType::Cardio => {
                    // Duration
                    match (first.duration_min, last.duration_min) {
                        (Some(fd), Some(ld)) if count > 1 => {
                            let delta = ld - fd;
                            let (arrow, color) = float_trend(delta);
                            lines.push(Line::from(vec![
                                Span::raw("  Duration "),
                                Span::styled(format!("{:.1}", fd), Style::default().fg(Color::DarkGray)),
                                Span::raw(" → "),
                                Span::styled(format!("{:.1} min", ld), Style::default().fg(Color::Green)),
                                Span::raw("  "),
                                Span::styled(format!("{} {:+.1}", arrow, delta), Style::default().fg(color)),
                            ]));
                        }
                        (Some(d), _) => {
                            lines.push(Line::from(vec![
                                Span::raw("  Duration "),
                                Span::styled(format!("{:.1} min", d), Style::default().fg(Color::Green)),
                            ]));
                        }
                        _ => {}
                    }

                    // Distance
                    match (first.distance_km, last.distance_km) {
                        (Some(fd), Some(ld)) if count > 1 => {
                            let delta = ld - fd;
                            let (arrow, color) = float_trend(delta);
                            lines.push(Line::from(vec![
                                Span::raw("  Distance "),
                                Span::styled(format!("{:.2}", fd), Style::default().fg(Color::DarkGray)),
                                Span::raw(" → "),
                                Span::styled(format!("{:.2} km", ld), Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(format!("{} {:+.2}", arrow, delta), Style::default().fg(color)),
                            ]));
                        }
                        (Some(d), _) => {
                            lines.push(Line::from(vec![
                                Span::raw("  Distance "),
                                Span::styled(format!("{:.2} km", d), Style::default().fg(Color::Cyan)),
                            ]));
                        }
                        _ => {}
                    }
                }
            }

            lines.push(Line::raw(""));
        }
        lines
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Exercise Trends ")
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

/// @brief Draws the full personal-metrics history as a scrollable, newest-first list.
/// @param frame Frame to draw into.
/// @param app Current application state; `metrics_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the list.
fn render_metrics_history(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .metrics
        .iter()
        .map(|m| {
            let weight = m.weight_kg.map(|v| format!("{v:.1} kg")).unwrap_or_else(|| "—".into());
            let hr = m.heart_rate.map(|v| format!("{v} bpm")).unwrap_or_else(|| "—".into());
            let bp = match (m.bp_systolic, m.bp_diastolic) {
                (Some(s), Some(d)) => format!("{s}/{d}"),
                _ => "—".into(),
            };
            let date = m.recorded_at.get(..16).unwrap_or(&m.recorded_at);
            ListItem::new(format!(
                "  {}   Weight: {:>10}   HR: {:>8}   BP: {}",
                date, weight, hr, bp
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Metrics History (newest first) "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.metrics_list_state);
}

/// @brief Draws the list of recipes, showing each one's category.
/// @param frame Frame to draw into.
/// @param app Current application state; `recipe_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the list.
fn render_recipe_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .recipes
        .iter()
        .map(|r| ListItem::new(format!("  {}  ({})", r.name, r.category)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Recipes "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.recipe_list_state);
}

/// @brief Draws a single recipe's metadata/instructions alongside its attached ingredients.
/// @param frame Frame to draw into.
/// @param app Current application state; `recipe_ingredient_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the recipe-detail body.
fn render_recipe_detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let recipe = app
        .detail_recipe_id
        .and_then(|id| app.recipes.iter().find(|r| r.id == id));

    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let meta_lines = if let Some(recipe) = recipe {
        let mut lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    recipe.name.as_str(),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(format!("  {}", recipe.category), Style::default().fg(Color::DarkGray))),
            Line::raw(""),
        ];
        if recipe.prep_min > 0 || recipe.cook_min > 0 || recipe.cook_temp > 0 {
            if recipe.prep_min > 0 {
                lines.push(Line::raw(format!("  Prep: {} min", recipe.prep_min)));
            }
            if recipe.cook_min > 0 {
                lines.push(Line::raw(format!("  Cook: {} min", recipe.cook_min)));
            }
            if recipe.cook_temp > 0 {
                lines.push(Line::raw(format!("  Temp: {}°", recipe.cook_temp)));
            }
            lines.push(Line::raw(""));
        }
        if recipe.instructions.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No instructions yet.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled("  Instructions:", Style::default().add_modifier(Modifier::BOLD))));
            lines.push(Line::raw(format!("  {}", recipe.instructions)));
        }
        lines
    } else {
        vec![Line::raw(""), Line::from(Span::styled("  Recipe not found.", Style::default().fg(Color::DarkGray)))]
    };

    let meta_block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(
        Paragraph::new(meta_lines).block(meta_block).wrap(Wrap { trim: false }),
        halves[0],
    );

    let mut items: Vec<ListItem> = app
        .current_recipe_ingredients
        .iter()
        .map(|ri| {
            let qty = match (ri.amount, ri.unit.is_empty()) {
                (Some(a), true) => format!("{a} "),
                (Some(a), false) => format!("{a} {} ", ri.unit),
                (None, true) => String::new(),
                (None, false) => format!("{} ", ri.unit),
            };
            ListItem::new(format!("  {}{}", qty, ri.name))
        })
        .collect();
    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  No ingredients yet — press [n] to attach one",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Ingredients "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, halves[1], &mut app.recipe_ingredient_list_state);
}

/// @brief Draws the ingredient-library picker, appending a "Custom ingredient..." entry after the library items.
/// @param frame Frame to draw into.
/// @param app Current application state; `ingredient_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the picker.
fn render_pick_ingredient(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = app
        .ingredients
        .iter()
        .map(|i| ListItem::new(format!("  {}  ({})  —  {}", i.name, i.category, stock_label(i))))
        .collect();

    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  Library is empty — press [Enter] to add your first ingredient",
            Style::default().fg(Color::DarkGray),
        )));
    }

    items.push(ListItem::new(Span::styled(
        "  [ Custom ingredient... ]",
        Style::default().fg(Color::DarkGray),
    )));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Ingredient Library "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.ingredient_list_state);
}

/// @brief Draws the full ingredient library as a browsable, alphabetical list.
/// @param frame Frame to draw into.
/// @param app Current application state; `ingredient_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the list.
fn render_ingredient_library(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .ingredients
        .iter()
        .map(|i| ListItem::new(format!("  {}  ({})  —  {}", i.name, i.category, stock_label(i))))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Ingredient Library "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.ingredient_list_state);
}

/// @brief Formats an ingredient's on-hand stock for display, or a placeholder if untracked.
/// @param ingredient The ingredient whose `stock_amount`/`stock_unit` to format.
/// @return e.g. `"2 lb on hand"`, or `"not tracked"` if `stock_amount` is `None`.
fn stock_label(ingredient: &Ingredient) -> String {
    match ingredient.stock_amount {
        Some(amount) if ingredient.stock_unit.is_empty() => format!("{amount} on hand"),
        Some(amount) => format!("{amount} {} on hand", ingredient.stock_unit),
        None => "not tracked".to_string(),
    }
}

/// @brief Draws the full meal history as a scrollable, newest-first list.
/// @param frame Frame to draw into.
/// @param app Current application state; `meal_log_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the list.
fn render_meal_history(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .meal_log
        .iter()
        .map(|m| {
            let date = m.logged_at.get(..16).unwrap_or(&m.logged_at);
            ListItem::new(format!(
                "  {}   {}  ({})  ×{}",
                date, m.meal_name, m.category, m.servings
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Meal Log (newest first) "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.meal_log_list_state);
}

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

use crate::app::{App, FavoritesFocus, RecipeDetailFocus, Screen, SearchResults};
use crate::models::{ExerciseType, Ingredient, IngredientType};

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
        | Screen::AddRecipeComponent
        | Screen::AddIngredient
        | Screen::SetIngredientStock
        | Screen::SetIngredientNutrition
        | Screen::SearchByName => {
            render_form(frame, app, chunks[1]);
        }
        Screen::ExerciseTypeSelect => render_type_select(frame, app, chunks[1]),
        Screen::MetricsHistory => render_metrics_history(frame, app, chunks[1]),
        Screen::Reports => render_reports(frame, app, chunks[1]),
        Screen::PickExercise => render_pick_exercise(frame, app, chunks[1]),
        Screen::RecipeList => render_recipe_list(frame, app, chunks[1]),
        Screen::RecipeDetail => render_recipe_detail(frame, app, chunks[1]),
        Screen::PickIngredient => render_pick_ingredient(frame, app, chunks[1]),
        Screen::PickSubRecipe => render_pick_sub_recipe(frame, app, chunks[1]),
        Screen::IngredientLibrary => render_ingredient_library(frame, app, chunks[1]),
        Screen::MealHistory => render_meal_history(frame, app, chunks[1]),
        Screen::Recipebook => render_recipebook(frame, app, chunks[1]),
        Screen::Search => render_search(frame, app, chunks[1]),
        Screen::SearchPickCategory => render_search_pick_category(frame, app, chunks[1]),
        Screen::SearchResults => render_search_results(frame, app, chunks[1]),
        Screen::Favorites => render_favorites(frame, app, chunks[1]),
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
        Screen::PickSubRecipe => " Rustic Buddy  —  Select Sub-Recipe ",
        Screen::AddRecipeComponent => " Rustic Buddy  —  Sub-Recipe Amount ",
        Screen::IngredientLibrary => " Rustic Buddy  —  Ingredients ",
        Screen::AddIngredient => " Rustic Buddy  —  Add Ingredient ",
        Screen::SetIngredientStock => " Rustic Buddy  —  Set Stock ",
        Screen::SetIngredientNutrition => " Rustic Buddy  —  Set Nutrition ",
        Screen::MealHistory => " Rustic Buddy  —  Meal Log ",
        Screen::Recipebook => " Rustic Buddy  —  Recipebook ",
        Screen::Search => " Rustic Buddy  —  Search ",
        Screen::SearchByName => " Rustic Buddy  —  Search ",
        Screen::SearchPickCategory => " Rustic Buddy  —  Search by Category ",
        Screen::SearchResults => " Rustic Buddy  —  Search Results ",
        Screen::Favorites => " Rustic Buddy  —  Favorites ",
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
        Screen::Recipebook => "[↑↓] Navigate  [Enter/→] Open  [1-5] Jump  [←/Esc/q] Back",
        Screen::WorkoutList => "[↑↓] Navigate  [Enter/→] Open  [a] Set Active  [n] New  [d] Delete  [←/Esc/q] Back",
        Screen::WorkoutDetail => "[↑↓] Navigate  [n] Add  [l] Log session  [d] Delete  [←/Esc/q] Back",
        Screen::AddWorkout
        | Screen::AddExercise
        | Screen::LogMetrics
        | Screen::AddToLibrary
        | Screen::AddRecipe
        | Screen::AddRecipeIngredient
        | Screen::AddRecipeComponent
        | Screen::AddIngredient
        | Screen::SetIngredientStock
        | Screen::SetIngredientNutrition
        | Screen::SearchByName => {
            "[Tab/Enter] Next Field  [Shift+Tab] Prev  [Enter] Submit on last field  [Esc] Cancel"
        }
        Screen::ExerciseTypeSelect => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::PickExercise => "[↑↓] Navigate  [Enter/→] Select  [n] Add to library  [←/Esc/q] Back",
        Screen::MetricsHistory => "[↑↓] Navigate  [d] Delete  [←/Esc/q] Back",
        Screen::Reports => "[←/Esc/q] Back",
        Screen::RecipeList => "[↑↓] Navigate  [Enter/→] Open  [n] New  [f] Favorite  [d] Delete  [←/Esc/q] Back",
        Screen::RecipeDetail => {
            "[Tab] Switch Pane  [↑↓] Navigate  [n] Add  [Enter/→] Open Sub-Recipe  [l] Log Meal  [f] Favorite  [d] Remove  [←/Esc/q] Back"
        }
        Screen::PickIngredient => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::PickSubRecipe => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::IngredientLibrary => {
            "[↑↓] Navigate  [n] New  [s] Set Stock  [u] Set Nutrition  [f] Favorite  [d] Delete  [←/Esc/q] Back"
        }
        Screen::MealHistory => "[↑↓] Navigate  [d] Delete  [←/Esc/q] Back",
        Screen::Search => "[↑↓] Navigate  [Enter/→] Select  [1-6] Jump  [←/Esc/q] Back",
        Screen::SearchPickCategory => "[↑↓] Navigate  [Enter/→] Select  [←/Esc/q] Back",
        Screen::SearchResults => "[↑↓] Navigate  [Enter/→] Open  [←/Esc/q] Back to Search",
        Screen::Favorites => "[Tab] Switch Pane  [↑↓] Navigate  [Enter/→] Open  [f] Unfavorite  [←/Esc/q] Back",
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

/// @brief Draws the Recipebook submenu: Recipes, Ingredients, Meal Log, Search, and Favorites.
/// @param frame Frame to draw into.
/// @param app Current application state; `recipebook_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the menu.
fn render_recipebook(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = ["Recipes", "Ingredients", "Meal Log", "Search", "Favorites"]
        .iter()
        .map(|label| ListItem::new(format!("  {label}")))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Recipebook "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.recipebook_list_state);
}

/// @brief Draws the Search submenu offering the five search modes.
/// @param frame Frame to draw into.
/// @param app Current application state; `search_menu_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the menu.
fn render_search(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = [
        "Ingredients by Type",
        "Ingredients by Name",
        "Recipes by Name",
        "Recipes containing Ingredient (by Name)",
        "Recipes containing Ingredient (by Type)",
        "Recipes containing Sub-Recipe (by Name)",
    ]
    .iter()
    .map(|label| ListItem::new(format!("  {label}")))
    .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Search "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.search_menu_list_state);
}

/// @brief Draws a picker over every ::IngredientType variant, for the two category-based search modes.
/// @param frame Frame to draw into.
/// @param app Current application state; `search_category_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the picker.
fn render_search_pick_category(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = IngredientType::ALL
        .iter()
        .map(|c| ListItem::new(format!("  {c}")))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Select a Category "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.search_category_list_state);
}

/// @brief Draws the matches from the most recently run search (::App::search_results), titled with ::App::search_result_label.
/// @param frame Frame to draw into.
/// @param app Current application state; `search_results_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the results list.
fn render_search_results(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = match &app.search_results {
        SearchResults::Ingredients(ingredients) => ingredients
            .iter()
            .map(|i| ListItem::new(format!("  {}{}  ({})  —  {}", favorite_mark(i.is_favorite), i.name, i.category, stock_label(i))))
            .collect(),
        SearchResults::Recipes(recipes) => recipes
            .iter()
            .map(|r| ListItem::new(format!("  {}{}  ({})", favorite_mark(r.is_favorite), r.name, r.category)))
            .collect(),
    };
    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  No matches.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let title = format!(" {} ", app.search_result_label);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.search_results_list_state);
}

/// @brief Draws the Favorites screen: a Recipes pane and an Ingredients pane, side by side.
/// @param frame Frame to draw into.
/// @param app Current application state; `favorite_recipes_list_state`/`favorite_ingredients_list_state`
/// track selection/scroll, `favorites_focus` picks which pane is highlighted.
/// @param area Rectangle allotted to the screen.
fn render_favorites(frame: &mut Frame, app: &mut App, area: Rect) {
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(area);

    let focused_style = Style::default().fg(Color::Yellow);
    let unfocused_style = Style::default();

    let mut recipe_items: Vec<ListItem> = app
        .favorite_recipes
        .iter()
        .map(|r| ListItem::new(format!("  {}{}  ({})", favorite_mark(r.is_favorite), r.name, r.category)))
        .collect();
    if recipe_items.is_empty() {
        recipe_items.push(ListItem::new(Span::styled(
            "  No favorite recipes yet — press [f] on a recipe to favorite it",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let recipe_border = if app.favorites_focus == FavoritesFocus::Recipes { focused_style } else { unfocused_style };
    let recipe_list = List::new(recipe_items)
        .block(Block::default().borders(Borders::ALL).title(" Favorite Recipes ").border_style(recipe_border))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(recipe_list, halves[0], &mut app.favorite_recipes_list_state);

    let mut ingredient_items: Vec<ListItem> = app
        .favorite_ingredients
        .iter()
        .map(|i| ListItem::new(format!("  {}{}  ({})  —  {}", favorite_mark(i.is_favorite), i.name, i.category, stock_label(i))))
        .collect();
    if ingredient_items.is_empty() {
        ingredient_items.push(ListItem::new(Span::styled(
            "  No favorite ingredients yet — press [f] on an ingredient to favorite it",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let ingredient_border = if app.favorites_focus == FavoritesFocus::Ingredients { focused_style } else { unfocused_style };
    let ingredient_list = List::new(ingredient_items)
        .block(Block::default().borders(Borders::ALL).title(" Favorite Ingredients ").border_style(ingredient_border))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(ingredient_list, halves[1], &mut app.favorite_ingredients_list_state);
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

/// @brief Draws the three-pane Reports screen: biometric trends, exercise trends, and daily nutrition objectives.
/// @param frame Frame to draw into.
/// @param app Current application state.
/// @param area Rectangle allotted to the reports body.
fn render_reports(frame: &mut Frame, app: &App, area: Rect) {
    let thirds = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(area);

    render_biometric_trends(frame, app, thirds[0]);
    render_exercise_trends(frame, app, thirds[1]);
    render_daily_objectives(frame, app, thirds[2]);
}

/// @brief Draws target daily nutrition intake computed from the most recently logged body weight.
///
/// Targets are derived from bodyweight-based rules of thumb (not stored, purely computed):
/// calories = weight(lbs) × 10, protein = weight(lbs) × 0.8g, fat = weight(lbs) × 0.3g,
/// fiber = (calories / 1000) × 14g, and carbs = whatever calories remain after fat
/// (9 kcal/g) and protein (4 kcal/g) are accounted for, converted to grams at 4 kcal/g.
///
/// Scans for the most recent ::crate::models::PersonalMetric with a weight recorded (a
/// reading may log only heart rate/BP), rather than assuming the latest entry has one.
///
/// @param frame Frame to draw into.
/// @param app Current application state (used only for `app.metrics`).
/// @param area Rectangle allotted to this panel.
fn render_daily_objectives(frame: &mut Frame, app: &App, area: Rect) {
    let lines = match app.metrics.iter().find_map(|m| m.weight_kg) {
        None => vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  No weight logged yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::raw("  Press [2/m] from the dashboard to log your weight."),
        ],
        Some(weight_kg) => {
            const KG_TO_LB: f64 = 2.20462;
            let weight_lbs = weight_kg * KG_TO_LB;

            let calories = weight_lbs * 10.0;
            let protein_g = weight_lbs * 0.8;
            let fat_g = weight_lbs * 0.3;
            let fiber_g = (calories / 1000.0) * 14.0;
            let carbs_g = (calories - fat_g * 9.0 - protein_g * 4.0) / 4.0;

            vec![
                Line::from(Span::styled(
                    format!("  Based on {:.1} kg ({:.1} lb)", weight_kg, weight_lbs),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  Calories  "),
                    Span::styled(format!("{:.0} kcal", calories), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  Protein   "),
                    Span::styled(format!("{:.0} g", protein_g), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  Fat       "),
                    Span::styled(format!("{:.0} g", fat_g), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  Carbs     "),
                    Span::styled(format!("{:.0} g", carbs_g), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  Fiber     "),
                    Span::styled(format!("{:.0} g", fiber_g), Style::default().fg(Color::Green)),
                ]),
            ]
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Daily Objectives ")
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
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
        .map(|r| ListItem::new(format!("  {}{}  ({})", favorite_mark(r.is_favorite), r.name, r.category)))
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
                Span::raw(favorite_mark(recipe.is_favorite)),
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
        if app.current_recipe_nutrition.total_grams > 0.0 {
            lines.push(Line::from(Span::styled("  Nutrition (per gram):", Style::default().add_modifier(Modifier::BOLD))));
            lines.push(Line::raw(format!("  Total weight: {:.0} g", app.current_recipe_nutrition.total_grams)));
            lines.push(Line::raw(format!("  {}", macro_label("Calories", app.current_recipe_nutrition.calories_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Protein", app.current_recipe_nutrition.protein_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Fat", app.current_recipe_nutrition.fat_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Fiber", app.current_recipe_nutrition.fiber_per_g))));
            lines.push(Line::raw(""));
        } else {
            lines.push(Line::from(Span::styled(
                "  No nutrition data yet — attach ingredients with nutrition set, or sub-recipes with gram-convertible amounts.",
                Style::default().fg(Color::DarkGray),
            )));
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

    let panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(halves[1]);

    let focused_style = Style::default().fg(Color::Yellow);
    let unfocused_style = Style::default();

    let mut ingredient_items: Vec<ListItem> = app
        .current_recipe_ingredients
        .iter()
        .map(|ri| ListItem::new(format!("  {}{}", qty_label(ri.amount, &ri.unit, ri.grams), ri.name)))
        .collect();
    if ingredient_items.is_empty() {
        ingredient_items.push(ListItem::new(Span::styled(
            "  No ingredients yet — press [n] to attach one",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let ingredient_border = if app.recipe_detail_focus == RecipeDetailFocus::Ingredients {
        focused_style
    } else {
        unfocused_style
    };
    let ingredient_list = List::new(ingredient_items)
        .block(Block::default().borders(Borders::ALL).title(" Ingredients ").border_style(ingredient_border))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(ingredient_list, panes[0], &mut app.recipe_ingredient_list_state);

    let mut component_items: Vec<ListItem> = app
        .current_recipe_components
        .iter()
        .map(|rc| ListItem::new(format!("  {}{}", qty_label(rc.amount, &rc.unit, rc.grams), rc.name)))
        .collect();
    if component_items.is_empty() {
        component_items.push(ListItem::new(Span::styled(
            "  No sub-recipes yet — press [n] to attach one",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let component_border = if app.recipe_detail_focus == RecipeDetailFocus::Components {
        focused_style
    } else {
        unfocused_style
    };
    let component_list = List::new(component_items)
        .block(Block::default().borders(Borders::ALL).title(" Sub-Recipes ").border_style(component_border))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(component_list, panes[1], &mut app.recipe_component_list_state);
}

/// @brief Formats one line of ::RecipeNutrition's per-gram output, e.g. `"Calories: 2.10 / g"`.
/// @param label Macro name (e.g. `"Calories"`).
/// @param per_g The computed per-gram value, or `None` if no contributor had this macro's data.
/// @return e.g. `"Calories: 2.10 / g"`, or `"Calories: unknown"` when `per_g` is `None`.
fn macro_label(label: &str, per_g: Option<f64>) -> String {
    match per_g {
        Some(v) => format!("{label}: {v:.2} / g"),
        None => format!("{label}: unknown"),
    }
}

/// @brief Formats an optional amount/unit pair as a leading quantity label, appending the
/// auto-converted gram amount when known and not already redundant with a bare grams unit.
/// @param amount Quantity, if specified.
/// @param unit Unit of measure; may be empty even when `amount` is `Some`.
/// @param grams The amount/unit converted to grams (see `crate::models::grams_for`), if known.
/// @return e.g. `"2 cups (≈240g) "`, `"1 lb (≈454g) "`, `"2 "`, `"cups "`, or `""`.
fn qty_label(amount: Option<f64>, unit: &str, grams: Option<f64>) -> String {
    let base = match (amount, unit.is_empty()) {
        (Some(a), true) => format!("{a} "),
        (Some(a), false) => format!("{a} {unit} "),
        (None, true) => String::new(),
        (None, false) => format!("{unit} "),
    };
    let is_bare_grams = matches!(unit.trim().to_ascii_lowercase().as_str(), "g" | "gram" | "grams");
    match grams {
        Some(g) if !is_bare_grams => format!("{base}(≈{g:.0}g) "),
        _ => base,
    }
}

/// @brief Draws the sub-recipe picker: every existing recipe eligible to be attached as a component
/// of the recipe open in ::Screen::RecipeDetail (self and cycle-causing recipes already filtered out).
/// @param frame Frame to draw into.
/// @param app Current application state; `pick_sub_recipe_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the picker.
fn render_pick_sub_recipe(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = app
        .sub_recipe_candidates
        .iter()
        .map(|r| ListItem::new(format!("  {}  ({})", r.name, r.category)))
        .collect();

    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  No eligible recipes — create another recipe first",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Select a Sub-Recipe "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.pick_sub_recipe_list_state);
}

/// @brief Draws the ingredient-library picker, appending a "Custom ingredient..." entry after the library items.
/// @param frame Frame to draw into.
/// @param app Current application state; `ingredient_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the picker.
fn render_pick_ingredient(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = app
        .ingredients
        .iter()
        .map(|i| ListItem::new(format!("  {}{}  ({})  —  {}", favorite_mark(i.is_favorite), i.name, i.category, stock_label(i))))
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

/// @brief Draws the ingredient library as a two-pane view: a browsable, alphabetical list on the
/// left, and the nutrient facts for whichever ingredient is selected on the right.
/// @param frame Frame to draw into.
/// @param app Current application state; `ingredient_list_state` tracks selection/scroll.
/// @param area Rectangle allotted to the view.
fn render_ingredient_library(frame: &mut Frame, app: &mut App, area: Rect) {
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let items: Vec<ListItem> = app
        .ingredients
        .iter()
        .map(|i| ListItem::new(format!("  {}{}  ({})  —  {}", favorite_mark(i.is_favorite), i.name, i.category, stock_label(i))))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Ingredient Library "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, halves[0], &mut app.ingredient_list_state);

    let selected = app
        .ingredient_list_state
        .selected()
        .and_then(|i| app.ingredients.get(i));

    let facts_lines = if let Some(ingredient) = selected {
        let mut lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::raw(favorite_mark(ingredient.is_favorite)),
                Span::styled(
                    ingredient.name.as_str(),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(format!("  {}", ingredient.category), Style::default().fg(Color::DarkGray))),
            Line::raw(""),
            Line::raw(format!("  Stock: {}", stock_label(ingredient))),
            Line::raw(""),
        ];
        if ingredient.calories_per_g.is_none()
            && ingredient.protein_per_g.is_none()
            && ingredient.fat_per_g.is_none()
            && ingredient.fiber_per_g.is_none()
        {
            lines.push(Line::from(Span::styled(
                "  No nutrition data yet — press [u] to set it.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled("  Nutrition (per gram):", Style::default().add_modifier(Modifier::BOLD))));
            lines.push(Line::raw(format!("  {}", macro_label("Calories", ingredient.calories_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Protein", ingredient.protein_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Fat", ingredient.fat_per_g))));
            lines.push(Line::raw(format!("  {}", macro_label("Fiber", ingredient.fiber_per_g))));
        }
        lines.push(Line::raw(""));
        match ingredient.density_g_per_ml {
            Some(d) => lines.push(Line::raw(format!("  Density: {d:.3} g/mL"))),
            None => lines.push(Line::from(Span::styled(
                "  Density: unknown (volume-unit amounts of this ingredient won't convert to grams)",
                Style::default().fg(Color::DarkGray),
            ))),
        }
        lines
    } else {
        vec![Line::raw(""), Line::from(Span::styled("  No ingredients yet — press [n] to add one.", Style::default().fg(Color::DarkGray)))]
    };

    let facts_block = Block::default()
        .borders(Borders::ALL)
        .title(" Nutrient Facts ")
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(
        Paragraph::new(facts_lines).block(facts_block).wrap(Wrap { trim: false }),
        halves[1],
    );
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

/// @brief Returns a leading star marker for a favorited item, padded to the same width otherwise.
/// @param is_favorite Whether the item is favorited.
/// @return `"★ "` if favorited, else `"  "`, so list columns stay aligned either way.
fn favorite_mark(is_favorite: bool) -> &'static str {
    if is_favorite { "★ " } else { "  " }
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

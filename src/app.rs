//! @file app.rs
//! @brief Application state, screen/navigation model, and input handling.
//!
//! ::App owns the database connection plus all in-memory view state, and is
//! mutated in response to key events (see ::App::handle_event). ::crate::ui
//! reads this state to render each frame but never mutates it.

use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::ListState;
use rusqlite::Connection;

use crate::{database, models::{
    Exercise, ExerciseLogEntry, ExerciseType, Ingredient, IngredientType, MealLogEntry,
    PersonalMetric, Recipe, RecipeIngredient, RecipeType, WorkoutPlan,
}};

/// @brief Identifies which screen of the TUI is currently displayed.
///
/// Drives both ::crate::ui::render's dispatch and ::App::handle_nav_key's
/// dispatch; the two must stay in sync for every variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    WorkoutList,
    WorkoutDetail,
    AddWorkout,
    AddExercise,
    ExerciseTypeSelect,
    LogMetrics,
    MetricsHistory,
    Reports,
    PickExercise,
    AddToLibrary,

    RecipeList,
    RecipeDetail,
    AddRecipe,
    PickIngredient,
    AddRecipeIngredient,
    IngredientLibrary,
    AddIngredient,
    SetIngredientStock,
    MealHistory,
    Recipebook,
}

/// @brief All mutable state for the running TUI: current screen, DB handle, cached query results, and form state.
pub struct App {
    pub screen: Screen,
    pub db: Connection,
    pub should_quit: bool,

    // Top-level menus
    pub dashboard_list_state: ListState,
    pub recipebook_list_state: ListState,

    // Workout list
    pub workout_plans: Vec<WorkoutPlan>,
    pub workout_list_state: ListState,

    // Active plan exercises shown on dashboard
    pub active_exercises: Vec<Exercise>,

    // Workout detail
    pub detail_plan_id: Option<i64>,
    pub current_exercises: Vec<Exercise>,
    pub exercise_list_state: ListState,

    // Metrics
    pub metrics: Vec<PersonalMetric>,
    pub metrics_list_state: ListState,

    // Plan exercise counts (plan_id → count)
    pub plan_exercise_counts: HashMap<i64, usize>,

    // Exercise library
    pub library_exercises: Vec<String>,
    pub library_list_state: ListState,

    // Exercise history (for Reports trends)
    pub exercise_logs: Vec<ExerciseLogEntry>,

    // Exercise type selection (between PickExercise and AddExercise)
    pub pending_exercise_name: Option<String>,
    pub type_select_index: usize,
    pub current_exercise_type: ExerciseType,

    // Form state
    pub input_fields: Vec<String>,
    pub input_labels: Vec<&'static str>,
    pub input_index: usize,
    pub input_mode: bool,
    pub status_msg: Option<String>,

    // Cookbook: recipes
    pub recipes: Vec<Recipe>,
    pub recipe_list_state: ListState,
    pub detail_recipe_id: Option<i64>,
    pub current_recipe_ingredients: Vec<RecipeIngredient>,
    pub recipe_ingredient_list_state: ListState,

    // Cookbook: ingredient library (shared by IngredientLibrary and PickIngredient)
    pub ingredients: Vec<Ingredient>,
    pub ingredient_list_state: ListState,
    pub pending_ingredient_id: Option<i64>,
    pub adding_ingredient_for_recipe: bool,
    pub editing_ingredient_id: Option<i64>,

    // Cookbook: meal log
    pub meal_log: Vec<MealLogEntry>,
    pub meal_log_list_state: ListState,
}

impl App {
    /// @brief Constructs the initial application state and performs the first load of all data from the database.
    /// @param db An open database connection (see ::crate::database::open); ::App takes ownership of it.
    /// @return A new ::App on the ::Screen::Dashboard with plans, metrics, library, and logs already populated.
    pub fn new(db: Connection) -> Self {
        let mut app = Self {
            screen: Screen::Dashboard,
            db,
            should_quit: false,
            dashboard_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            recipebook_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            workout_plans: vec![],
            workout_list_state: ListState::default(),
            active_exercises: vec![],
            detail_plan_id: None,
            current_exercises: vec![],
            exercise_list_state: ListState::default(),
            metrics: vec![],
            metrics_list_state: ListState::default(),
            plan_exercise_counts: HashMap::new(),
            library_exercises: vec![],
            library_list_state: ListState::default(),
            exercise_logs: vec![],
            pending_exercise_name: None,
            type_select_index: 0,
            current_exercise_type: ExerciseType::Strength,
            input_fields: vec![],
            input_labels: vec![],
            input_index: 0,
            input_mode: false,
            status_msg: None,
            recipes: vec![],
            recipe_list_state: ListState::default(),
            detail_recipe_id: None,
            current_recipe_ingredients: vec![],
            recipe_ingredient_list_state: ListState::default(),
            ingredients: vec![],
            ingredient_list_state: ListState::default(),
            pending_ingredient_id: None,
            adding_ingredient_for_recipe: false,
            editing_ingredient_id: None,
            meal_log: vec![],
            meal_log_list_state: ListState::default(),
        };
        app.refresh_plans();
        app.refresh_metrics();
        app.refresh_library();
        app.refresh_exercise_logs();
        app.refresh_recipes();
        app.refresh_ingredients();
        app.refresh_meal_log();
        app
    }

    /// @brief Reloads all workout plans, the active plan's exercises, and per-plan exercise counts from the database.
    ///
    /// Any query failure is silently treated as an empty result (via
    /// `unwrap_or_default`) rather than propagated, since the UI has no
    /// error-display path for background refreshes.
    pub fn refresh_plans(&mut self) {
        self.workout_plans = database::get_workout_plans(&self.db).unwrap_or_default();
        self.active_exercises = self
            .workout_plans
            .iter()
            .find(|p| p.is_active)
            .and_then(|p| database::get_exercises(&self.db, p.id).ok())
            .unwrap_or_default();
        self.plan_exercise_counts = database::get_exercise_counts(&self.db).unwrap_or_default();
    }

    /// @brief Reloads the exercise list for whichever plan is open in the workout-detail screen (::App::detail_plan_id).
    ///
    /// No-op if `detail_plan_id` is `None`.
    pub fn refresh_current_exercises(&mut self) {
        if let Some(id) = self.detail_plan_id {
            self.current_exercises = database::get_exercises(&self.db, id).unwrap_or_default();
        }
    }

    /// @brief Reloads the reusable exercise-name library from the database.
    pub fn refresh_library(&mut self) {
        self.library_exercises = database::get_library_exercises(&self.db).unwrap_or_default();
    }

    /// @brief Reloads the full exercise session history used by the Reports screen.
    pub fn refresh_exercise_logs(&mut self) {
        self.exercise_logs = database::get_exercise_logs(&self.db).unwrap_or_default();
    }

    /// @brief Reloads the 30 most recent personal-metrics readings.
    pub fn refresh_metrics(&mut self) {
        self.metrics = database::get_metrics(&self.db, 30).unwrap_or_default();
    }

    /// @brief Reloads the full recipe list from the database.
    pub fn refresh_recipes(&mut self) {
        self.recipes = database::get_recipes(&self.db).unwrap_or_default();
    }

    /// @brief Reloads the ingredient list for whichever recipe is open in the recipe-detail screen (::App::detail_recipe_id).
    ///
    /// No-op if `detail_recipe_id` is `None`.
    pub fn refresh_current_recipe_ingredients(&mut self) {
        if let Some(id) = self.detail_recipe_id {
            self.current_recipe_ingredients =
                database::get_recipe_ingredients(&self.db, id).unwrap_or_default();
        }
    }

    /// @brief Reloads the ingredient library from the database.
    pub fn refresh_ingredients(&mut self) {
        self.ingredients = database::get_ingredients(&self.db).unwrap_or_default();
    }

    /// @brief Reloads the full meal history from the database.
    pub fn refresh_meal_log(&mut self) {
        self.meal_log = database::get_meal_log(&self.db).unwrap_or_default();
    }

    /// @brief Returns the currently active workout plan, if any.
    /// @return A reference to the ::WorkoutPlan with `is_active == true`, or `None` if no plan is active.
    pub fn active_plan(&self) -> Option<&WorkoutPlan> {
        self.workout_plans.iter().find(|p| p.is_active)
    }

    /// @brief Polls for and processes a single terminal input event, if one is available within 250ms.
    ///
    /// Routes key presses to ::App::handle_input_key when a form is open
    /// (::App::input_mode) or to ::App::handle_nav_key otherwise. Non-key
    /// events and key-release/repeat events are ignored.
    ///
    /// @return `Ok(())` unless reading from the terminal fails.
    pub fn handle_event(&mut self) -> std::io::Result<()> {
        if !event::poll(Duration::from_millis(250))? {
            return Ok(());
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            if self.input_mode {
                self.handle_input_key(key.code);
            } else {
                self.handle_nav_key(key.code);
            }
        }
        Ok(())
    }

    /// @brief Handles a key press while a text-entry form is active (::App::input_mode == true).
    ///
    /// Esc cancels the form, Tab/Shift+Tab and Enter move between fields
    /// (Enter on the last field submits via ::App::submit_form),
    /// Backspace/character keys edit the focused field's text.
    ///
    /// @param code The key that was pressed.
    fn handle_input_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.cancel_form(),
            KeyCode::Tab => {
                self.input_index = (self.input_index + 1) % self.input_fields.len();
            }
            KeyCode::BackTab => {
                if self.input_index == 0 {
                    self.input_index = self.input_fields.len() - 1;
                } else {
                    self.input_index -= 1;
                }
            }
            KeyCode::Enter => {
                if self.input_index < self.input_fields.len() - 1 {
                    self.input_index += 1;
                } else {
                    self.submit_form();
                }
            }
            KeyCode::Backspace => {
                self.input_fields[self.input_index].pop();
            }
            KeyCode::Char(c) => {
                self.input_fields[self.input_index].push(c);
            }
            _ => {}
        }
    }

    /// @brief Dispatches a navigation key press to the handler for the current ::Screen.
    ///
    /// Screens without list navigation or actions (e.g. form screens, which
    /// are handled by ::App::handle_input_key instead) fall through to a no-op.
    ///
    /// @param code The key that was pressed.
    fn handle_nav_key(&mut self, code: KeyCode) {
        match self.screen.clone() {
            Screen::Dashboard => self.handle_dashboard_key(code),
            Screen::WorkoutList => self.handle_workout_list_key(code),
            Screen::WorkoutDetail => self.handle_workout_detail_key(code),
            Screen::MetricsHistory => self.handle_metrics_history_key(code),
            Screen::Reports => self.handle_reports_key(code),
            Screen::PickExercise => self.handle_pick_exercise_key(code),
            Screen::ExerciseTypeSelect => self.handle_type_select_key(code),
            Screen::RecipeList => self.handle_recipe_list_key(code),
            Screen::RecipeDetail => self.handle_recipe_detail_key(code),
            Screen::PickIngredient => self.handle_pick_ingredient_key(code),
            Screen::IngredientLibrary => self.handle_ingredient_library_key(code),
            Screen::MealHistory => self.handle_meal_history_key(code),
            Screen::Recipebook => self.handle_recipebook_key(code),
            _ => {}
        }
    }

    /// @brief Number of selectable entries in the ::Screen::Dashboard menu (see ::App::activate_dashboard_item).
    const DASHBOARD_MENU_LEN: usize = 5;

    /// @brief Handles key presses on ::Screen::Dashboard: quit, arrow/Enter navigation of the main menu, and its number/letter shortcuts.
    /// @param code The key that was pressed.
    fn handle_dashboard_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => {
                let i = self.dashboard_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.dashboard_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.dashboard_list_state.selected().unwrap_or(0);
                if i + 1 < Self::DASHBOARD_MENU_LEN {
                    self.dashboard_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => self.activate_dashboard_item(),
            KeyCode::Char('1') | KeyCode::Char('w') => {
                self.dashboard_list_state.select(Some(0));
                self.activate_dashboard_item();
            }
            KeyCode::Char('2') | KeyCode::Char('m') => {
                self.dashboard_list_state.select(Some(1));
                self.activate_dashboard_item();
            }
            KeyCode::Char('3') | KeyCode::Char('h') => {
                self.dashboard_list_state.select(Some(2));
                self.activate_dashboard_item();
            }
            KeyCode::Char('4') | KeyCode::Char('r') => {
                self.dashboard_list_state.select(Some(3));
                self.activate_dashboard_item();
            }
            KeyCode::Char('5') | KeyCode::Char('c') => {
                self.dashboard_list_state.select(Some(4));
                self.activate_dashboard_item();
            }
            _ => {}
        }
    }

    /// @brief Opens whichever top-level screen is selected in the ::Screen::Dashboard menu (::App::dashboard_list_state).
    fn activate_dashboard_item(&mut self) {
        match self.dashboard_list_state.selected().unwrap_or(0) {
            0 => {
                if self.workout_list_state.selected().is_none() && !self.workout_plans.is_empty() {
                    self.workout_list_state.select(Some(0));
                }
                self.screen = Screen::WorkoutList;
            }
            1 => self.open_log_metrics(),
            2 => self.screen = Screen::MetricsHistory,
            3 => self.screen = Screen::Reports,
            4 => self.screen = Screen::Recipebook,
            _ => {}
        }
    }

    /// @brief Number of selectable entries in the ::Screen::Recipebook menu (see ::App::activate_recipebook_item).
    const RECIPEBOOK_MENU_LEN: usize = 3;

    /// @brief Handles key presses on ::Screen::Recipebook: back to the dashboard, arrow/Enter navigation, and its number/letter shortcuts.
    /// @param code The key that was pressed.
    fn handle_recipebook_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            KeyCode::Up => {
                let i = self.recipebook_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.recipebook_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.recipebook_list_state.selected().unwrap_or(0);
                if i + 1 < Self::RECIPEBOOK_MENU_LEN {
                    self.recipebook_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => self.activate_recipebook_item(),
            KeyCode::Char('1') | KeyCode::Char('r') => {
                self.recipebook_list_state.select(Some(0));
                self.activate_recipebook_item();
            }
            KeyCode::Char('2') | KeyCode::Char('i') => {
                self.recipebook_list_state.select(Some(1));
                self.activate_recipebook_item();
            }
            KeyCode::Char('3') | KeyCode::Char('l') => {
                self.recipebook_list_state.select(Some(2));
                self.activate_recipebook_item();
            }
            _ => {}
        }
    }

    /// @brief Opens whichever cookbook screen is selected in the ::Screen::Recipebook menu (::App::recipebook_list_state).
    fn activate_recipebook_item(&mut self) {
        match self.recipebook_list_state.selected().unwrap_or(0) {
            0 => {
                self.refresh_recipes();
                if self.recipe_list_state.selected().is_none() && !self.recipes.is_empty() {
                    self.recipe_list_state.select(Some(0));
                }
                self.screen = Screen::RecipeList;
            }
            1 => {
                self.refresh_ingredients();
                if self.ingredient_list_state.selected().is_none() && !self.ingredients.is_empty() {
                    self.ingredient_list_state.select(Some(0));
                }
                self.screen = Screen::IngredientLibrary;
            }
            2 => {
                self.refresh_meal_log();
                if self.meal_log_list_state.selected().is_none() && !self.meal_log.is_empty() {
                    self.meal_log_list_state.select(Some(0));
                }
                self.screen = Screen::MealHistory;
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::WorkoutList: navigate, open, activate, create, and delete plans.
    /// @param code The key that was pressed.
    fn handle_workout_list_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            KeyCode::Up => {
                let i = self.workout_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.workout_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.workout_plans.len();
                let i = self.workout_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.workout_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if let Some(i) = self.workout_list_state.selected() {
                    if let Some(plan) = self.workout_plans.get(i) {
                        self.detail_plan_id = Some(plan.id);
                        self.current_exercises = vec![];
                        self.exercise_list_state = ListState::default();
                        self.refresh_current_exercises();
                        if !self.current_exercises.is_empty() {
                            self.exercise_list_state.select(Some(0));
                        }
                        self.screen = Screen::WorkoutDetail;
                    }
                }
            }
            KeyCode::Char('a') => {
                if let Some(i) = self.workout_list_state.selected() {
                    if let Some(plan) = self.workout_plans.get(i) {
                        let id = plan.id;
                        let _ = database::set_active_plan(&self.db, id);
                        self.refresh_plans();
                    }
                }
            }
            KeyCode::Char('n') => self.open_add_workout(),
            KeyCode::Char('d') => {
                if let Some(i) = self.workout_list_state.selected() {
                    if let Some(plan) = self.workout_plans.get(i) {
                        let id = plan.id;
                        let _ = database::delete_workout_plan(&self.db, id);
                        self.refresh_plans();
                        let new_i = i.saturating_sub(1);
                        if self.workout_plans.is_empty() {
                            self.workout_list_state.select(None);
                        } else {
                            self.workout_list_state
                                .select(Some(new_i.min(self.workout_plans.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::WorkoutDetail: navigate exercises, add, log a session for all exercises, and delete.
    /// @param code The key that was pressed.
    fn handle_workout_detail_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::WorkoutList;
            }
            KeyCode::Up => {
                let i = self.exercise_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.exercise_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.current_exercises.len();
                let i = self.exercise_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.exercise_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('n') => {
                self.open_pick_exercise();
            }
            KeyCode::Char('l') => {
                if !self.current_exercises.is_empty() {
                    let exercises = self.current_exercises.clone();
                    for ex in &exercises {
                        let _ = database::log_exercise_session(
                            &self.db,
                            &ex.name,
                            &ex.exercise_type,
                            ex.sets,
                            ex.reps,
                            ex.weight,
                            ex.duration_min,
                            ex.distance_km,
                        );
                    }
                    self.refresh_exercise_logs();
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.exercise_list_state.selected() {
                    if let Some(ex) = self.current_exercises.get(i) {
                        let id = ex.id;
                        let _ = database::delete_exercise(&self.db, id);
                        self.refresh_current_exercises();
                        self.refresh_plans();
                        let new_i = i.saturating_sub(1);
                        if self.current_exercises.is_empty() {
                            self.exercise_list_state.select(None);
                        } else {
                            self.exercise_list_state
                                .select(Some(new_i.min(self.current_exercises.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::PickExercise: navigate the library list (plus a trailing "Custom" entry) and select or add.
    /// @param code The key that was pressed.
    fn handle_pick_exercise_key(&mut self, code: KeyCode) {
        let total = self.library_exercises.len() + 1; // +1 for "Custom"
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::WorkoutDetail;
            }
            KeyCode::Up => {
                let i = self.library_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.library_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.library_list_state.selected().unwrap_or(0);
                if i + 1 < total {
                    self.library_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let i = self.library_list_state.selected().unwrap_or(0);
                if self.detail_plan_id.is_some() {
                    self.pending_exercise_name = if i < self.library_exercises.len() {
                        Some(self.library_exercises[i].clone())
                    } else {
                        None
                    };
                    self.type_select_index = 0;
                    self.screen = Screen::ExerciseTypeSelect;
                }
            }
            KeyCode::Char('n') => {
                self.open_add_to_library();
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::ExerciseTypeSelect: toggle between Strength/Cardio and confirm into ::App::open_add_exercise.
    /// @param code The key that was pressed.
    fn handle_type_select_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::PickExercise;
            }
            KeyCode::Up => {
                if self.type_select_index > 0 {
                    self.type_select_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.type_select_index < 1 {
                    self.type_select_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let exercise_type = if self.type_select_index == 0 {
                    ExerciseType::Strength
                } else {
                    ExerciseType::Cardio
                };
                if let Some(plan_id) = self.detail_plan_id {
                    let name = self.pending_exercise_name.take();
                    self.open_add_exercise(plan_id, name, exercise_type);
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::Reports: only exiting back to the dashboard.
    /// @param code The key that was pressed.
    fn handle_reports_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::MetricsHistory: navigate readings and delete the selected one.
    /// @param code The key that was pressed.
    fn handle_metrics_history_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            KeyCode::Up => {
                let i = self.metrics_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.metrics_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.metrics.len();
                let i = self.metrics_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.metrics_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.metrics_list_state.selected() {
                    if let Some(m) = self.metrics.get(i) {
                        let id = m.id;
                        let _ = database::delete_metric(&self.db, id);
                        self.refresh_metrics();
                        let new_i = i.saturating_sub(1);
                        if self.metrics.is_empty() {
                            self.metrics_list_state.select(None);
                        } else {
                            self.metrics_list_state
                                .select(Some(new_i.min(self.metrics.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::RecipeList: navigate, open, create, and delete recipes.
    /// @param code The key that was pressed.
    fn handle_recipe_list_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Recipebook,
            KeyCode::Up => {
                let i = self.recipe_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.recipe_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.recipes.len();
                let i = self.recipe_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.recipe_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if let Some(i) = self.recipe_list_state.selected() {
                    if let Some(recipe) = self.recipes.get(i) {
                        self.detail_recipe_id = Some(recipe.id);
                        self.current_recipe_ingredients = vec![];
                        self.recipe_ingredient_list_state = ListState::default();
                        self.refresh_current_recipe_ingredients();
                        if !self.current_recipe_ingredients.is_empty() {
                            self.recipe_ingredient_list_state.select(Some(0));
                        }
                        self.screen = Screen::RecipeDetail;
                    }
                }
            }
            KeyCode::Char('n') => self.open_add_recipe(),
            KeyCode::Char('d') => {
                if let Some(i) = self.recipe_list_state.selected() {
                    if let Some(recipe) = self.recipes.get(i) {
                        let id = recipe.id;
                        let _ = database::delete_recipe(&self.db, id);
                        self.refresh_recipes();
                        let new_i = i.saturating_sub(1);
                        if self.recipes.is_empty() {
                            self.recipe_list_state.select(None);
                        } else {
                            self.recipe_list_state
                                .select(Some(new_i.min(self.recipes.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::RecipeDetail: navigate attached ingredients, attach, and detach.
    /// @param code The key that was pressed.
    fn handle_recipe_detail_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::RecipeList;
            }
            KeyCode::Up => {
                let i = self.recipe_ingredient_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.recipe_ingredient_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.current_recipe_ingredients.len();
                let i = self.recipe_ingredient_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.recipe_ingredient_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('n') => {
                self.open_pick_ingredient();
            }
            KeyCode::Char('l') => {
                self.log_meal_from_recipe();
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.recipe_ingredient_list_state.selected() {
                    if let Some(ri) = self.current_recipe_ingredients.get(i) {
                        let id = ri.id;
                        let _ = database::remove_recipe_ingredient(&self.db, id);
                        self.refresh_current_recipe_ingredients();
                        let new_i = i.saturating_sub(1);
                        if self.current_recipe_ingredients.is_empty() {
                            self.recipe_ingredient_list_state.select(None);
                        } else {
                            self.recipe_ingredient_list_state
                                .select(Some(new_i.min(self.current_recipe_ingredients.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::PickIngredient: navigate the library list (plus a trailing "Custom" entry) and select or add.
    /// @param code The key that was pressed.
    fn handle_pick_ingredient_key(&mut self, code: KeyCode) {
        let total = self.ingredients.len() + 1; // +1 for "Custom"
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::RecipeDetail;
            }
            KeyCode::Up => {
                let i = self.ingredient_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.ingredient_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.ingredient_list_state.selected().unwrap_or(0);
                if i + 1 < total {
                    self.ingredient_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let i = self.ingredient_list_state.selected().unwrap_or(0);
                if i < self.ingredients.len() {
                    self.pending_ingredient_id = Some(self.ingredients[i].id);
                    self.open_add_recipe_ingredient();
                } else {
                    self.adding_ingredient_for_recipe = true;
                    self.open_add_ingredient();
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::IngredientLibrary: navigate, add, and delete library ingredients.
    /// @param code The key that was pressed.
    fn handle_ingredient_library_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Recipebook,
            KeyCode::Up => {
                let i = self.ingredient_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.ingredient_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.ingredients.len();
                let i = self.ingredient_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.ingredient_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('n') => {
                self.adding_ingredient_for_recipe = false;
                self.open_add_ingredient();
            }
            KeyCode::Char('s') => {
                self.open_set_ingredient_stock();
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.ingredient_list_state.selected() {
                    if let Some(ingredient) = self.ingredients.get(i) {
                        let id = ingredient.id;
                        let _ = database::delete_ingredient(&self.db, id);
                        self.refresh_ingredients();
                        let new_i = i.saturating_sub(1);
                        if self.ingredients.is_empty() {
                            self.ingredient_list_state.select(None);
                        } else {
                            self.ingredient_list_state
                                .select(Some(new_i.min(self.ingredients.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::MealHistory: navigate readings and delete the selected one.
    /// @param code The key that was pressed.
    fn handle_meal_history_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Recipebook,
            KeyCode::Up => {
                let i = self.meal_log_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.meal_log_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.meal_log.len();
                let i = self.meal_log_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.meal_log_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.meal_log_list_state.selected() {
                    if let Some(entry) = self.meal_log.get(i) {
                        let id = entry.id;
                        let _ = database::delete_meal_log_entry(&self.db, id);
                        self.refresh_meal_log();
                        let new_i = i.saturating_sub(1);
                        if self.meal_log.is_empty() {
                            self.meal_log_list_state.select(None);
                        } else {
                            self.meal_log_list_state
                                .select(Some(new_i.min(self.meal_log.len() - 1)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Opens the "new workout plan" form (::Screen::AddWorkout) with a single blank name field.
    fn open_add_workout(&mut self) {
        self.input_fields = vec![String::new()];
        self.input_labels = vec!["Workout Name"];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddWorkout;
    }

    /// @brief Refreshes the library and opens ::Screen::PickExercise with the first item selected.
    fn open_pick_exercise(&mut self) {
        self.refresh_library();
        self.library_list_state.select(Some(0));
        self.screen = Screen::PickExercise;
    }

    /// @brief Opens the "add to library" form (::Screen::AddToLibrary) with a single blank name field.
    fn open_add_to_library(&mut self) {
        self.input_fields = vec![String::new()];
        self.input_labels = vec!["Exercise Name"];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddToLibrary;
    }

    /// @brief Logs one serving of the open recipe as eaten and depletes matching pantry stock.
    ///
    /// No-op if `detail_recipe_id` is `None` or no longer resolves to a recipe. For each attached
    /// ingredient with a specified `amount`, the matching library ingredient's `stock_amount` is
    /// decremented by that amount (floored at 0) — but only if that ingredient's stock is already
    /// being tracked (`Some`); untracked ingredients are left untracked rather than seeded from zero.
    fn log_meal_from_recipe(&mut self) {
        let Some(recipe) = self
            .detail_recipe_id
            .and_then(|id| self.recipes.iter().find(|r| r.id == id))
            .cloned()
        else {
            return;
        };

        let _ = database::log_meal(&self.db, &recipe.name, &recipe.category, 1.0);

        self.refresh_ingredients();
        for ri in &self.current_recipe_ingredients {
            let Some(amount) = ri.amount else { continue };
            if let Some(ingredient) = self.ingredients.iter().find(|i| i.id == ri.ingredient_id) {
                if let Some(stock) = ingredient.stock_amount {
                    let new_stock = (stock - amount).max(0.0);
                    let _ = database::set_ingredient_stock(
                        &self.db, ingredient.id, Some(new_stock), &ingredient.stock_unit,
                    );
                }
            }
        }
        self.refresh_ingredients();
        self.refresh_meal_log();
    }

    /// @brief Opens the "set stock" form (::Screen::SetIngredientStock) for the selected library ingredient, prefilled with its current amount/unit.
    fn open_set_ingredient_stock(&mut self) {
        let Some(i) = self.ingredient_list_state.selected() else { return };
        let Some(ingredient) = self.ingredients.get(i) else { return };

        self.editing_ingredient_id = Some(ingredient.id);
        self.input_fields = vec![
            ingredient.stock_amount.map(|a| a.to_string()).unwrap_or_default(),
            ingredient.stock_unit.clone(),
        ];
        self.input_labels = vec!["On Hand (optional)", "Unit (e.g. lb, cups)"];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::SetIngredientStock;
    }

    /// @brief Opens the "new recipe" form (::Screen::AddRecipe) with name/category/timing/instructions fields.
    fn open_add_recipe(&mut self) {
        self.input_fields = vec![
            String::new(),
            String::new(),
            String::from("0"),
            String::from("0"),
            String::from("0"),
            String::new(),
        ];
        self.input_labels = vec![
            "Recipe Name",
            "Category (Baked/Boiled/Brewed/Crafted/Fried/Grilled/Mixed/Roasted/Sauteed/Steamed/Simmered)",
            "Prep Time (min)",
            "Cook Time (min)",
            "Cook Temp",
            "Instructions",
        ];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddRecipe;
    }

    /// @brief Refreshes the ingredient library and opens ::Screen::PickIngredient with the first item selected.
    fn open_pick_ingredient(&mut self) {
        self.refresh_ingredients();
        self.ingredient_list_state.select(Some(0));
        self.screen = Screen::PickIngredient;
    }

    /// @brief Opens the "attach ingredient" form (::Screen::AddRecipeIngredient) with blank amount/unit fields.
    fn open_add_recipe_ingredient(&mut self) {
        self.input_fields = vec![String::new(), String::new()];
        self.input_labels = vec!["Amount (optional)", "Unit (optional)"];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddRecipeIngredient;
    }

    /// @brief Opens the "add ingredient" form (::Screen::AddIngredient) with blank name/category fields.
    ///
    /// Where this returns to on submit/cancel depends on ::App::adding_ingredient_for_recipe,
    /// set by the caller before opening this form.
    fn open_add_ingredient(&mut self) {
        self.input_fields = vec![String::new(), String::new()];
        self.input_labels = vec![
            "Ingredient Name",
            "Category (Condiment/Craft/Culinary/Dairy/Fat/Fish/Fruit/Herb/Grain/Meat/Nuts/Seasoning/Sweetener/Vegetable)",
        ];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddIngredient;
    }

    /// @brief Opens the "add exercise" form (::Screen::AddExercise) with fields tailored to the given exercise type.
    ///
    /// Strength exercises get Name/Sets/Reps/Weight fields (Sets and Reps
    /// preset to "3" and "10"); cardio exercises get Name/Duration/Distance
    /// fields. If `prefill_name` is `Some`, the name field is pre-filled and
    /// focus starts on the second field instead of the name field.
    ///
    /// @param plan_id Plan the new exercise will belong to (stored in ::App::detail_plan_id).
    /// @param prefill_name Exercise name carried over from ::Screen::PickExercise, if one was chosen from the library.
    /// @param exercise_type Strength or cardio, chosen on ::Screen::ExerciseTypeSelect.
    fn open_add_exercise(&mut self, plan_id: i64, prefill_name: Option<String>, exercise_type: ExerciseType) {
        let has_name = prefill_name.is_some();
        self.detail_plan_id = Some(plan_id);
        self.current_exercise_type = exercise_type;

        match self.current_exercise_type {
            ExerciseType::Strength => {
                self.input_fields = vec![
                    prefill_name.unwrap_or_default(),
                    String::from("3"),
                    String::from("10"),
                    String::new(),
                ];
                self.input_labels = vec!["Exercise Name", "Sets", "Reps", "Weight kg (optional)"];
            }
            ExerciseType::Cardio => {
                self.input_fields = vec![
                    prefill_name.unwrap_or_default(),
                    String::new(),
                    String::new(),
                ];
                self.input_labels = vec!["Exercise Name", "Duration (min)", "Distance km (optional)"];
            }
        }
        self.input_index = if has_name { 1 } else { 0 };
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddExercise;
    }

    /// @brief Opens the "log metrics" form (::Screen::LogMetrics) with four blank fields (weight, heart rate, systolic, diastolic).
    fn open_log_metrics(&mut self) {
        self.input_fields = vec![
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        self.input_labels = vec![
            "Weight (kg)",
            "Heart Rate (bpm)",
            "Blood Pressure Systolic",
            "Blood Pressure Diastolic",
        ];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::LogMetrics;
    }

    /// @brief Exits the current form without saving and returns to the screen it was opened from.
    fn cancel_form(&mut self) {
        self.input_mode = false;
        self.status_msg = None;
        self.screen = match self.screen {
            Screen::AddWorkout => Screen::WorkoutList,
            Screen::AddExercise => Screen::ExerciseTypeSelect,
            Screen::LogMetrics => Screen::Dashboard,
            Screen::AddToLibrary => Screen::PickExercise,
            Screen::AddRecipe => Screen::RecipeList,
            Screen::AddRecipeIngredient => Screen::RecipeDetail,
            Screen::AddIngredient => {
                if self.adding_ingredient_for_recipe {
                    Screen::PickIngredient
                } else {
                    Screen::IngredientLibrary
                }
            }
            Screen::SetIngredientStock => Screen::IngredientLibrary,
            _ => Screen::Dashboard,
        };
    }

    /// @brief Dispatches Enter-on-last-field submission to the handler matching the current form screen.
    fn submit_form(&mut self) {
        match self.screen.clone() {
            Screen::AddWorkout => self.submit_add_workout(),
            Screen::AddExercise => self.submit_add_exercise(),
            Screen::LogMetrics => self.submit_log_metrics(),
            Screen::AddToLibrary => self.submit_add_to_library(),
            Screen::AddRecipe => self.submit_add_recipe(),
            Screen::AddRecipeIngredient => self.submit_add_recipe_ingredient(),
            Screen::AddIngredient => self.submit_add_ingredient(),
            Screen::SetIngredientStock => self.submit_set_ingredient_stock(),
            _ => {}
        }
    }

    /// @brief Validates and saves the "add to library" form, then returns to ::Screen::PickExercise with the new entry selected.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg
    /// and leaving the form open.
    fn submit_add_to_library(&mut self) {
        let name = self.input_fields[0].trim().to_string();
        if name.is_empty() {
            self.status_msg = Some("Name cannot be empty.".into());
            return;
        }
        if let Err(e) = database::add_to_library(&self.db, &name) {
            self.status_msg = Some(format!("Error: {e}"));
            return;
        }
        self.refresh_library();
        let idx = self.library_exercises.iter().position(|n| n == &name).unwrap_or(0);
        self.library_list_state.select(Some(idx));
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::PickExercise;
    }

    /// @brief Validates and saves the "new workout plan" form, then returns to ::Screen::WorkoutList.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg
    /// and leaving the form open.
    fn submit_add_workout(&mut self) {
        let name = self.input_fields[0].trim().to_string();
        if name.is_empty() {
            self.status_msg = Some("Workout name cannot be empty.".into());
            return;
        }
        if let Err(e) = database::create_workout_plan(&self.db, &name) {
            self.status_msg = Some(format!("Error: {e}"));
            return;
        }
        self.refresh_plans();
        if self.workout_list_state.selected().is_none() {
            self.workout_list_state.select(Some(0));
        }
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::WorkoutList;
    }

    /// @brief Validates and saves the "add exercise" form, parsing numeric fields according to ::App::current_exercise_type, then returns to ::Screen::WorkoutDetail.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg
    /// and leaving the form open. Numeric fields that fail to parse fall
    /// back to defaults (sets=3, reps=10) or `None` rather than blocking submission.
    fn submit_add_exercise(&mut self) {
        let name = self.input_fields[0].trim().to_string();
        if name.is_empty() {
            self.status_msg = Some("Exercise name cannot be empty.".into());
            return;
        }

        let (sets, reps, weight, duration_min, distance_km) = match self.current_exercise_type {
            ExerciseType::Strength => {
                let sets: i32 = self.input_fields[1].trim().parse().unwrap_or(3);
                let reps: i32 = self.input_fields[2].trim().parse().unwrap_or(10);
                let weight: Option<f64> = self.input_fields[3].trim().parse().ok();
                (sets, reps, weight, None, None)
            }
            ExerciseType::Cardio => {
                let duration_min: Option<f64> = self.input_fields[1].trim().parse().ok();
                let distance_km: Option<f64> = self.input_fields[2].trim().parse().ok();
                (0, 0, None, duration_min, distance_km)
            }
        };

        if let Some(plan_id) = self.detail_plan_id {
            if let Err(e) = database::add_exercise(
                &self.db, plan_id, &name, &self.current_exercise_type,
                sets, reps, weight, duration_min, distance_km,
            ) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        }
        self.refresh_current_exercises();
        self.refresh_plans();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::WorkoutDetail;
    }

    /// @brief Saves the "log metrics" form and returns to ::Screen::Dashboard.
    ///
    /// All fields are optional: any field that fails to parse (including
    /// being left blank) is stored as `None` rather than blocking submission.
    fn submit_log_metrics(&mut self) {
        let weight_kg: Option<f64> = self.input_fields[0].trim().parse().ok();
        let heart_rate: Option<i32> = self.input_fields[1].trim().parse().ok();
        let bp_systolic: Option<i32> = self.input_fields[2].trim().parse().ok();
        let bp_diastolic: Option<i32> = self.input_fields[3].trim().parse().ok();

        if let Err(e) = database::log_metric(&self.db, weight_kg, heart_rate, bp_systolic, bp_diastolic) {
            self.status_msg = Some(format!("Error: {e}"));
            return;
        }
        self.refresh_metrics();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::Dashboard;
    }

    /// @brief Validates and saves the "new recipe" form, then opens the new recipe in ::Screen::RecipeDetail.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg
    /// and leaving the form open. The category field is parsed case-insensitively,
    /// falling back to Pending; numeric fields fall back to 0.
    fn submit_add_recipe(&mut self) {
        let name = self.input_fields[0].trim().to_string();
        if name.is_empty() {
            self.status_msg = Some("Recipe name cannot be empty.".into());
            return;
        }
        let category: RecipeType = self.input_fields[1].trim().parse().unwrap_or(RecipeType::Pending);
        let prep_min: i32 = self.input_fields[2].trim().parse().unwrap_or(0);
        let cook_min: i32 = self.input_fields[3].trim().parse().unwrap_or(0);
        let cook_temp: i32 = self.input_fields[4].trim().parse().unwrap_or(0);
        let instructions = self.input_fields[5].trim().to_string();

        let id = match database::create_recipe(
            &self.db, &name, &category, prep_min, cook_min, cook_temp, &instructions,
        ) {
            Ok(id) => id,
            Err(e) => {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        };
        self.refresh_recipes();
        self.detail_recipe_id = Some(id);
        self.current_recipe_ingredients = vec![];
        self.recipe_ingredient_list_state = ListState::default();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::RecipeDetail;
    }

    /// @brief Saves the "attach ingredient" form, linking ::App::pending_ingredient_id to the open recipe, then returns to ::Screen::RecipeDetail.
    ///
    /// Both fields are optional: an unparsable or blank amount is stored as `None`.
    fn submit_add_recipe_ingredient(&mut self) {
        let amount: Option<f64> = self.input_fields[0].trim().parse().ok();
        let unit = self.input_fields[1].trim().to_string();

        if let (Some(recipe_id), Some(ingredient_id)) = (self.detail_recipe_id, self.pending_ingredient_id) {
            if let Err(e) = database::add_recipe_ingredient(&self.db, recipe_id, ingredient_id, amount, &unit) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        }
        self.pending_ingredient_id = None;
        self.refresh_current_recipe_ingredients();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::RecipeDetail;
    }

    /// @brief Validates and saves the "add ingredient" form, then returns to ::Screen::PickIngredient or ::Screen::IngredientLibrary
    /// depending on ::App::adding_ingredient_for_recipe.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg
    /// and leaving the form open. If the ingredient was being added mid-recipe-attach,
    /// it is immediately selected as the pending ingredient and the amount/unit form is opened.
    fn submit_add_ingredient(&mut self) {
        let name = self.input_fields[0].trim().to_string();
        if name.is_empty() {
            self.status_msg = Some("Ingredient name cannot be empty.".into());
            return;
        }
        let category: IngredientType = self.input_fields[1].trim().parse().unwrap_or(IngredientType::Pending);

        if let Err(e) = database::create_ingredient(&self.db, &name, &category) {
            self.status_msg = Some(format!("Error: {e}"));
            return;
        }
        self.refresh_ingredients();
        self.status_msg = None;

        if self.adding_ingredient_for_recipe {
            self.adding_ingredient_for_recipe = false;
            if let Some(ingredient) = self.ingredients.iter().find(|i| i.name == name) {
                self.pending_ingredient_id = Some(ingredient.id);
            }
            self.open_add_recipe_ingredient();
        } else {
            let idx = self.ingredients.iter().position(|i| i.name == name).unwrap_or(0);
            self.ingredient_list_state.select(Some(idx));
            self.input_mode = false;
            self.screen = Screen::IngredientLibrary;
        }
    }

    /// @brief Saves the "set stock" form for ::App::editing_ingredient_id, then returns to ::Screen::IngredientLibrary.
    ///
    /// A blank or unparsable amount clears the tracked stock (`None`) rather than blocking submission.
    fn submit_set_ingredient_stock(&mut self) {
        let amount: Option<f64> = self.input_fields[0].trim().parse().ok();
        let unit = self.input_fields[1].trim().to_string();

        if let Some(id) = self.editing_ingredient_id {
            if let Err(e) = database::set_ingredient_stock(&self.db, id, amount, &unit) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        }
        self.editing_ingredient_id = None;
        self.refresh_ingredients();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::IngredientLibrary;
    }
}

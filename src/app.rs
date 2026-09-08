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
    self, Exercise, ExerciseLogEntry, ExercisePerformance, ExerciseType, Ingredient, IngredientType,
    MealLogEntry, PersonalMetric, Recipe, RecipeComponent, RecipeIngredient, RecipeNutrition,
    RecipeType, WorkoutPlan,
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
    WorkoutGraph,
    ExerciseGraph,
    AddWorkout,
    AddExercise,
    UpdateExercise,
    ExerciseTypeSelect,
    LogMetrics,
    MetricsHistory,
    MetricsGraph,
    Reports,
    ExerciseHistory,
    ExerciseSessionLog,
    EditExerciseLog,
    PickExercise,
    AddToLibrary,

    RecipeList,
    RecipeDetail,
    AddRecipe,
    PickIngredient,
    AddRecipeIngredient,
    PickSubRecipe,
    AddRecipeComponent,
    IngredientLibrary,
    AddIngredient,
    SetIngredientStock,
    SetIngredientNutrition,
    MealHistory,
    Recipebook,

    Search,
    SearchByName,
    SearchPickCategory,
    SearchResults,

    Favorites,
}

/// @brief Which pane of ::Screen::Favorites has navigation focus.
///
/// Drives which list `Up`/`Down`/`f` (unfavorite) act on (see ::App::handle_favorites_key).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FavoritesFocus {
    Recipes,
    Ingredients,
}

/// @brief Which pane of ::Screen::RecipeDetail has navigation focus.
///
/// Drives which list `Up`/`Down`/`n`/`d` act on (see ::App::handle_recipe_detail_key).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecipeDetailFocus {
    Ingredients,
    Components,
}

/// @brief Which of the five search modes is being performed.
///
/// Drives which database query ::App::submit_search_by_name (name-based
/// kinds) or the ::Screen::SearchPickCategory Enter handler (category-based
/// kinds) runs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchKind {
    IngredientByType,
    IngredientByName,
    RecipeByName,
    RecipeByIngredientName,
    RecipeByIngredientType,
    RecipeBySubRecipeName,
}

/// @brief The result set from the most recently run search, tagged by which kind of row it holds.
pub enum SearchResults {
    Ingredients(Vec<Ingredient>),
    Recipes(Vec<Recipe>),
}

impl SearchResults {
    /// @brief Number of matching rows, regardless of which variant this is.
    fn len(&self) -> usize {
        match self {
            SearchResults::Ingredients(v) => v.len(),
            SearchResults::Recipes(v) => v.len(),
        }
    }
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
    /// @brief Scroll/selection state for ::Screen::ExerciseHistory's full exercise-summary list.
    pub exercise_history_list_state: ListState,

    // Active plan exercises shown on dashboard
    pub active_exercises: Vec<Exercise>,

    // Workout detail
    pub detail_plan_id: Option<i64>,
    pub current_exercises: Vec<Exercise>,
    /// @brief Name of the exercise ::Screen::ExerciseGraph is currently charting, set when opened
    /// from ::Screen::WorkoutDetail via `[g]` on the highlighted exercise.
    pub graphed_exercise_name: String,
    /// @brief Primary key of the exercise ::Screen::UpdateExercise is currently updating, set
    /// when opened from ::Screen::WorkoutDetail via `[u]` on the highlighted exercise.
    pub updating_exercise_id: Option<i64>,
    /// @brief Exercise name ::Screen::ExerciseSessionLog is currently filtered to.
    pub session_log_exercise_name: String,
    /// @brief Scroll/selection state for ::Screen::ExerciseSessionLog's individual-session list.
    pub session_log_list_state: ListState,
    /// @brief Primary key of the `exercise_log` row ::Screen::EditExerciseLog is currently editing.
    pub editing_log_id: Option<i64>,
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

    // Cookbook: nested recipes (sub-recipe components)
    pub current_recipe_components: Vec<RecipeComponent>,
    pub recipe_component_list_state: ListState,
    pub recipe_detail_focus: RecipeDetailFocus,
    pub pending_component_child_id: Option<i64>,
    pub sub_recipe_candidates: Vec<Recipe>,
    pub pick_sub_recipe_list_state: ListState,

    // Cookbook: computed recipe nutrition (rolled up from ingredients/sub-recipes)
    pub current_recipe_nutrition: RecipeNutrition,

    // Cookbook: ingredient library (shared by IngredientLibrary and PickIngredient)
    pub ingredients: Vec<Ingredient>,
    pub ingredient_list_state: ListState,
    pub pending_ingredient_id: Option<i64>,
    pub adding_ingredient_for_recipe: bool,
    pub editing_ingredient_id: Option<i64>,

    // Cookbook: meal log
    pub meal_log: Vec<MealLogEntry>,
    pub meal_log_list_state: ListState,

    // Cookbook: search
    pub search_menu_list_state: ListState,
    pub search_category_list_state: ListState,
    pub search_results_list_state: ListState,
    pending_search_kind: Option<SearchKind>,
    pub search_results: SearchResults,
    pub search_result_label: String,

    // Cookbook: favorites
    pub favorite_recipes: Vec<Recipe>,
    pub favorite_recipes_list_state: ListState,
    pub favorite_ingredients: Vec<Ingredient>,
    pub favorite_ingredients_list_state: ListState,
    pub favorites_focus: FavoritesFocus,
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
            exercise_history_list_state: ListState::default(),
            active_exercises: vec![],
            detail_plan_id: None,
            current_exercises: vec![],
            graphed_exercise_name: String::new(),
            updating_exercise_id: None,
            session_log_exercise_name: String::new(),
            session_log_list_state: ListState::default(),
            editing_log_id: None,
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
            current_recipe_components: vec![],
            recipe_component_list_state: ListState::default(),
            recipe_detail_focus: RecipeDetailFocus::Ingredients,
            pending_component_child_id: None,
            sub_recipe_candidates: vec![],
            pick_sub_recipe_list_state: ListState::default(),
            current_recipe_nutrition: RecipeNutrition::default(),
            ingredients: vec![],
            ingredient_list_state: ListState::default(),
            pending_ingredient_id: None,
            adding_ingredient_for_recipe: false,
            editing_ingredient_id: None,
            meal_log: vec![],
            meal_log_list_state: ListState::default(),
            search_menu_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            search_category_list_state: ListState::default(),
            search_results_list_state: ListState::default(),
            pending_search_kind: None,
            search_results: SearchResults::Ingredients(vec![]),
            search_result_label: String::new(),
            favorite_recipes: vec![],
            favorite_recipes_list_state: ListState::default(),
            favorite_ingredients: vec![],
            favorite_ingredients_list_state: ListState::default(),
            favorites_focus: FavoritesFocus::Recipes,
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

    /// @brief Reloads the sub-recipe list for whichever recipe is open in the recipe-detail screen (::App::detail_recipe_id).
    ///
    /// No-op if `detail_recipe_id` is `None`.
    pub fn refresh_current_recipe_components(&mut self) {
        if let Some(id) = self.detail_recipe_id {
            self.current_recipe_components =
                database::get_recipe_components(&self.db, id).unwrap_or_default();
        }
    }

    /// @brief Recomputes the nutrition roll-up for whichever recipe is open in the recipe-detail screen (::App::detail_recipe_id).
    ///
    /// No-op if `detail_recipe_id` is `None`. Called wherever the recipe's ingredients or
    /// sub-recipes change, since either can affect the computed result.
    pub fn refresh_current_recipe_nutrition(&mut self) {
        if let Some(id) = self.detail_recipe_id {
            self.current_recipe_nutrition =
                database::compute_recipe_nutrition(&self.db, id).unwrap_or_default();
        }
    }

    /// @brief Opens ::Screen::RecipeDetail for the given recipe, loading its attached ingredients and sub-recipes.
    ///
    /// Shared by ::Screen::RecipeList's Enter handler, search results that
    /// resolve to a recipe, and drilling into a sub-recipe from the
    /// Components pane.
    /// @param recipe_id Primary key of the ::Recipe to open.
    fn open_recipe_detail(&mut self, recipe_id: i64) {
        self.detail_recipe_id = Some(recipe_id);
        self.current_recipe_ingredients = vec![];
        self.recipe_ingredient_list_state = ListState::default();
        self.refresh_current_recipe_ingredients();
        if !self.current_recipe_ingredients.is_empty() {
            self.recipe_ingredient_list_state.select(Some(0));
        }
        self.current_recipe_components = vec![];
        self.recipe_component_list_state = ListState::default();
        self.refresh_current_recipe_components();
        if !self.current_recipe_components.is_empty() {
            self.recipe_component_list_state.select(Some(0));
        }
        self.refresh_current_recipe_nutrition();
        self.recipe_detail_focus = RecipeDetailFocus::Ingredients;
        self.screen = Screen::RecipeDetail;
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
            Screen::WorkoutGraph => self.handle_workout_graph_key(code),
            Screen::ExerciseGraph => self.handle_exercise_graph_key(code),
            Screen::MetricsHistory => self.handle_metrics_history_key(code),
            Screen::MetricsGraph => self.handle_metrics_graph_key(code),
            Screen::Reports => self.handle_reports_key(code),
            Screen::ExerciseHistory => self.handle_exercise_history_key(code),
            Screen::ExerciseSessionLog => self.handle_exercise_session_log_key(code),
            Screen::PickExercise => self.handle_pick_exercise_key(code),
            Screen::ExerciseTypeSelect => self.handle_type_select_key(code),
            Screen::RecipeList => self.handle_recipe_list_key(code),
            Screen::RecipeDetail => self.handle_recipe_detail_key(code),
            Screen::PickIngredient => self.handle_pick_ingredient_key(code),
            Screen::PickSubRecipe => self.handle_pick_sub_recipe_key(code),
            Screen::IngredientLibrary => self.handle_ingredient_library_key(code),
            Screen::MealHistory => self.handle_meal_history_key(code),
            Screen::Recipebook => self.handle_recipebook_key(code),
            Screen::Search => self.handle_search_key(code),
            Screen::SearchPickCategory => self.handle_search_pick_category_key(code),
            Screen::SearchResults => self.handle_search_results_key(code),
            Screen::Favorites => self.handle_favorites_key(code),
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
    const RECIPEBOOK_MENU_LEN: usize = 5;

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
            KeyCode::Char('4') | KeyCode::Char('s') => {
                self.recipebook_list_state.select(Some(3));
                self.activate_recipebook_item();
            }
            KeyCode::Char('5') | KeyCode::Char('f') => {
                self.recipebook_list_state.select(Some(4));
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
            3 => {
                self.search_menu_list_state.select(Some(0));
                self.screen = Screen::Search;
            }
            4 => self.open_favorites(),
            _ => {}
        }
    }

    /// @brief Number of selectable entries in the ::Screen::Search menu (see ::App::activate_search_item).
    const SEARCH_MENU_LEN: usize = 6;

    /// @brief Handles key presses on ::Screen::Search: back to Recipebook, arrow/Enter navigation, and its number shortcuts.
    /// @param code The key that was pressed.
    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Recipebook,
            KeyCode::Up => {
                let i = self.search_menu_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.search_menu_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.search_menu_list_state.selected().unwrap_or(0);
                if i + 1 < Self::SEARCH_MENU_LEN {
                    self.search_menu_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => self.activate_search_item(),
            KeyCode::Char('1') => {
                self.search_menu_list_state.select(Some(0));
                self.activate_search_item();
            }
            KeyCode::Char('2') => {
                self.search_menu_list_state.select(Some(1));
                self.activate_search_item();
            }
            KeyCode::Char('3') => {
                self.search_menu_list_state.select(Some(2));
                self.activate_search_item();
            }
            KeyCode::Char('4') => {
                self.search_menu_list_state.select(Some(3));
                self.activate_search_item();
            }
            KeyCode::Char('5') => {
                self.search_menu_list_state.select(Some(4));
                self.activate_search_item();
            }
            KeyCode::Char('6') => {
                self.search_menu_list_state.select(Some(5));
                self.activate_search_item();
            }
            _ => {}
        }
    }

    /// @brief Opens the input screen matching whichever search mode is selected in the ::Screen::Search menu.
    ///
    /// Name-based modes open ::Screen::SearchByName (a text-entry form);
    /// category-based modes open ::Screen::SearchPickCategory (a list picker).
    fn activate_search_item(&mut self) {
        self.pending_search_kind = match self.search_menu_list_state.selected().unwrap_or(0) {
            0 => Some(SearchKind::IngredientByType),
            1 => Some(SearchKind::IngredientByName),
            2 => Some(SearchKind::RecipeByName),
            3 => Some(SearchKind::RecipeByIngredientName),
            4 => Some(SearchKind::RecipeByIngredientType),
            5 => Some(SearchKind::RecipeBySubRecipeName),
            _ => None,
        };
        match self.pending_search_kind {
            Some(SearchKind::IngredientByType) | Some(SearchKind::RecipeByIngredientType) => {
                self.open_search_pick_category();
            }
            Some(_) => self.open_search_by_name(),
            None => {}
        }
    }

    /// @brief Opens the "search by name" form (::Screen::SearchByName) with a single blank field, labeled per ::App::pending_search_kind.
    fn open_search_by_name(&mut self) {
        let label = match self.pending_search_kind {
            Some(SearchKind::RecipeByName) => "Recipe name contains",
            Some(SearchKind::RecipeByIngredientName) => "Ingredient name contains",
            Some(SearchKind::RecipeBySubRecipeName) => "Sub-recipe name contains",
            _ => "Ingredient name contains",
        };
        self.input_fields = vec![String::new()];
        self.input_labels = vec![label];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::SearchByName;
    }

    /// @brief Validates and runs a name-based search, then jumps to ::Screen::SearchResults.
    ///
    /// Rejects an empty (after trimming) search term by setting
    /// ::App::status_msg and leaving the form open.
    fn submit_search_by_name(&mut self) {
        let query = self.input_fields[0].trim().to_string();
        if query.is_empty() {
            self.status_msg = Some("Search term cannot be empty.".into());
            return;
        }
        match self.pending_search_kind {
            Some(SearchKind::IngredientByName) => {
                self.search_results = SearchResults::Ingredients(
                    database::search_ingredients_by_name(&self.db, &query).unwrap_or_default(),
                );
                self.search_result_label = format!("Ingredients — name contains \"{query}\"");
            }
            Some(SearchKind::RecipeByName) => {
                self.search_results = SearchResults::Recipes(
                    database::search_recipes_by_name(&self.db, &query).unwrap_or_default(),
                );
                self.search_result_label = format!("Recipes — name contains \"{query}\"");
            }
            Some(SearchKind::RecipeByIngredientName) => {
                self.search_results = SearchResults::Recipes(
                    database::search_recipes_by_ingredient_name(&self.db, &query).unwrap_or_default(),
                );
                self.search_result_label = format!("Recipes containing ingredient — name contains \"{query}\"");
            }
            Some(SearchKind::RecipeBySubRecipeName) => {
                self.search_results = SearchResults::Recipes(
                    database::search_recipes_by_subrecipe_name(&self.db, &query).unwrap_or_default(),
                );
                self.search_result_label = format!("Recipes containing sub-recipe — name contains \"{query}\"");
            }
            _ => return,
        }
        self.search_results_list_state = ListState::default();
        if self.search_results.len() > 0 {
            self.search_results_list_state.select(Some(0));
        }
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::SearchResults;
    }

    /// @brief Refreshes the ingredient library and opens ::Screen::SearchPickCategory with the first category selected.
    fn open_search_pick_category(&mut self) {
        self.search_category_list_state = ListState::default();
        self.search_category_list_state.select(Some(0));
        self.screen = Screen::SearchPickCategory;
    }

    /// @brief Handles key presses on ::Screen::SearchPickCategory: navigate ::IngredientType::ALL and run the matching category search.
    /// @param code The key that was pressed.
    fn handle_search_pick_category_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Search,
            KeyCode::Up => {
                let i = self.search_category_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.search_category_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let i = self.search_category_list_state.selected().unwrap_or(0);
                if i + 1 < IngredientType::ALL.len() {
                    self.search_category_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let i = self.search_category_list_state.selected().unwrap_or(0);
                let Some(category) = IngredientType::ALL.get(i) else { return };
                match self.pending_search_kind {
                    Some(SearchKind::IngredientByType) => {
                        self.search_results = SearchResults::Ingredients(
                            database::search_ingredients_by_category(&self.db, category).unwrap_or_default(),
                        );
                        self.search_result_label = format!("Ingredients — category \"{category}\"");
                    }
                    Some(SearchKind::RecipeByIngredientType) => {
                        self.search_results = SearchResults::Recipes(
                            database::search_recipes_by_ingredient_category(&self.db, category).unwrap_or_default(),
                        );
                        self.search_result_label =
                            format!("Recipes containing ingredient — category \"{category}\"");
                    }
                    _ => return,
                }
                self.search_results_list_state = ListState::default();
                if self.search_results.len() > 0 {
                    self.search_results_list_state.select(Some(0));
                }
                self.screen = Screen::SearchResults;
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::SearchResults: navigate and open the selected match.
    ///
    /// A matched recipe opens ::Screen::RecipeDetail (via ::App::open_recipe_detail);
    /// a matched ingredient jumps to ::Screen::IngredientLibrary with that
    /// entry selected, since ingredients have no standalone detail screen.
    /// @param code The key that was pressed.
    fn handle_search_results_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Search,
            KeyCode::Up => {
                let i = self.search_results_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.search_results_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.search_results.len();
                let i = self.search_results_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.search_results_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let Some(i) = self.search_results_list_state.selected() else { return };
                match &self.search_results {
                    SearchResults::Recipes(recipes) => {
                        if let Some(id) = recipes.get(i).map(|r| r.id) {
                            self.open_recipe_detail(id);
                        }
                    }
                    SearchResults::Ingredients(ingredients) => {
                        if let Some(id) = ingredients.get(i).map(|ing| ing.id) {
                            self.refresh_ingredients();
                            if let Some(idx) = self.ingredients.iter().position(|ing| ing.id == id) {
                                self.ingredient_list_state.select(Some(idx));
                            }
                            self.screen = Screen::IngredientLibrary;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// @brief Refreshes recipes/ingredients, filters both down to favorited items, and opens ::Screen::Favorites.
    fn open_favorites(&mut self) {
        self.refresh_recipes();
        self.refresh_ingredients();
        self.favorite_recipes = self.recipes.iter().filter(|r| r.is_favorite).cloned().collect();
        self.favorite_ingredients = self.ingredients.iter().filter(|i| i.is_favorite).cloned().collect();

        self.favorite_recipes_list_state = ListState::default();
        if !self.favorite_recipes.is_empty() {
            self.favorite_recipes_list_state.select(Some(0));
        }
        self.favorite_ingredients_list_state = ListState::default();
        if !self.favorite_ingredients.is_empty() {
            self.favorite_ingredients_list_state.select(Some(0));
        }
        self.favorites_focus = FavoritesFocus::Recipes;
        self.screen = Screen::Favorites;
    }

    /// @brief Handles key presses on ::Screen::Favorites: switch focus between the Recipes and
    /// Ingredients panes, navigate/open/unfavorite on whichever is focused.
    /// @param code The key that was pressed.
    fn handle_favorites_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Recipebook,
            KeyCode::Tab => {
                self.favorites_focus = match self.favorites_focus {
                    FavoritesFocus::Recipes => FavoritesFocus::Ingredients,
                    FavoritesFocus::Ingredients => FavoritesFocus::Recipes,
                };
            }
            KeyCode::Up => match self.favorites_focus {
                FavoritesFocus::Recipes => {
                    let i = self.favorite_recipes_list_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.favorite_recipes_list_state.select(Some(i - 1));
                    }
                }
                FavoritesFocus::Ingredients => {
                    let i = self.favorite_ingredients_list_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.favorite_ingredients_list_state.select(Some(i - 1));
                    }
                }
            },
            KeyCode::Down => match self.favorites_focus {
                FavoritesFocus::Recipes => {
                    let len = self.favorite_recipes.len();
                    let i = self.favorite_recipes_list_state.selected().unwrap_or(0);
                    if i + 1 < len {
                        self.favorite_recipes_list_state.select(Some(i + 1));
                    }
                }
                FavoritesFocus::Ingredients => {
                    let len = self.favorite_ingredients.len();
                    let i = self.favorite_ingredients_list_state.selected().unwrap_or(0);
                    if i + 1 < len {
                        self.favorite_ingredients_list_state.select(Some(i + 1));
                    }
                }
            },
            KeyCode::Char('f') => match self.favorites_focus {
                FavoritesFocus::Recipes => {
                    let Some(i) = self.favorite_recipes_list_state.selected() else { return };
                    let Some(id) = self.favorite_recipes.get(i).map(|r| r.id) else { return };
                    let _ = database::toggle_recipe_favorite(&self.db, id);
                    self.favorite_recipes.remove(i);
                    let new_i = i.saturating_sub(1);
                    if self.favorite_recipes.is_empty() {
                        self.favorite_recipes_list_state.select(None);
                    } else {
                        self.favorite_recipes_list_state.select(Some(new_i.min(self.favorite_recipes.len() - 1)));
                    }
                    self.refresh_recipes();
                }
                FavoritesFocus::Ingredients => {
                    let Some(i) = self.favorite_ingredients_list_state.selected() else { return };
                    let Some(id) = self.favorite_ingredients.get(i).map(|ing| ing.id) else { return };
                    let _ = database::toggle_ingredient_favorite(&self.db, id);
                    self.favorite_ingredients.remove(i);
                    let new_i = i.saturating_sub(1);
                    if self.favorite_ingredients.is_empty() {
                        self.favorite_ingredients_list_state.select(None);
                    } else {
                        self.favorite_ingredients_list_state
                            .select(Some(new_i.min(self.favorite_ingredients.len() - 1)));
                    }
                    self.refresh_ingredients();
                }
            },
            KeyCode::Enter | KeyCode::Right => match self.favorites_focus {
                FavoritesFocus::Recipes => {
                    let Some(i) = self.favorite_recipes_list_state.selected() else { return };
                    let Some(id) = self.favorite_recipes.get(i).map(|r| r.id) else { return };
                    self.open_recipe_detail(id);
                }
                FavoritesFocus::Ingredients => {
                    let Some(i) = self.favorite_ingredients_list_state.selected() else { return };
                    let Some(id) = self.favorite_ingredients.get(i).map(|ing| ing.id) else { return };
                    self.refresh_ingredients();
                    if let Some(idx) = self.ingredients.iter().position(|ing| ing.id == id) {
                        self.ingredient_list_state.select(Some(idx));
                    }
                    self.screen = Screen::IngredientLibrary;
                }
            },
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
                if let Some(i) = self.workout_list_state.selected()
                    && let Some(plan) = self.workout_plans.get(i) {
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
            KeyCode::Char('a') => {
                if let Some(i) = self.workout_list_state.selected()
                    && let Some(plan) = self.workout_plans.get(i) {
                        let id = plan.id;
                        let _ = database::set_active_plan(&self.db, id);
                        self.refresh_plans();
                    }
            }
            KeyCode::Char('n') => self.open_add_workout(),
            KeyCode::Char('d') => {
                if let Some(i) = self.workout_list_state.selected()
                    && let Some(plan) = self.workout_plans.get(i) {
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
            KeyCode::Char('g') => self.screen = Screen::WorkoutGraph,
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::WorkoutGraph: only exiting back to the workout list.
    /// @param code The key that was pressed.
    fn handle_workout_graph_key(&mut self, code: KeyCode) {
        if let KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left = code {
            self.screen = Screen::WorkoutList;
        }
    }

    /// @brief Handles key presses on ::Screen::ExerciseGraph: only exiting back to the workout detail screen.
    /// @param code The key that was pressed.
    fn handle_exercise_graph_key(&mut self, code: KeyCode) {
        if let KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left = code {
            self.screen = Screen::WorkoutDetail;
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
            KeyCode::Char('g') => {
                if let Some(i) = self.exercise_list_state.selected()
                    && let Some(ex) = self.current_exercises.get(i) {
                        self.graphed_exercise_name = ex.name.clone();
                        self.screen = Screen::ExerciseGraph;
                    }
            }
            KeyCode::Char('u') => {
                if let Some(i) = self.exercise_list_state.selected()
                    && let Some(ex) = self.current_exercises.get(i).cloned() {
                        self.open_update_exercise(&ex);
                    }
            }
            KeyCode::Char('l') => {
                if !self.current_exercises.is_empty() {
                    let exercises = self.current_exercises.clone();
                    for ex in &exercises {
                        let perf = ExercisePerformance {
                            exercise_type: ex.exercise_type.clone(),
                            sets: ex.sets,
                            reps: ex.reps,
                            weight: ex.weight,
                            duration_min: ex.duration_min,
                            distance_km: ex.distance_km,
                        };
                        let _ = database::log_exercise_session(&self.db, &ex.name, &perf);
                    }
                    self.refresh_exercise_logs();
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.exercise_list_state.selected()
                    && let Some(ex) = self.current_exercises.get(i) {
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

    /// @brief Handles key presses on ::Screen::Reports: exiting back to the dashboard, or opening
    /// the full exercise history.
    /// @param code The key that was pressed.
    fn handle_reports_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            KeyCode::Char('e') => self.open_exercise_history(),
            _ => {}
        }
    }

    /// @brief Refreshes ::Screen::ExerciseHistory's selection and opens it, showing every logged
    /// exercise's summary (see ::crate::models::summarize_exercise_logs), most recent first.
    fn open_exercise_history(&mut self) {
        let count = models::summarize_exercise_logs(&self.exercise_logs).len();
        self.exercise_history_list_state = ListState::default();
        if count > 0 {
            self.exercise_history_list_state.select(Some(0));
        }
        self.screen = Screen::ExerciseHistory;
    }

    /// @brief Handles key presses on ::Screen::ExerciseHistory: navigate the full exercise-summary
    /// list, and drill into one exercise's individual logged sessions.
    /// @param code The key that was pressed.
    fn handle_exercise_history_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Reports,
            KeyCode::Up => {
                let i = self.exercise_history_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.exercise_history_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = models::summarize_exercise_logs(&self.exercise_logs).len();
                let i = self.exercise_history_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.exercise_history_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                if let Some(i) = self.exercise_history_list_state.selected()
                    && let Some(s) = models::summarize_exercise_logs(&self.exercise_logs).get(i) {
                        self.open_exercise_session_log(s.name.clone());
                    }
            }
            _ => {}
        }
    }

    /// @brief Returns `app.exercise_logs` filtered to ::App::session_log_exercise_name, sorted
    /// most-recently-logged first.
    pub fn session_log_entries(&self) -> Vec<ExerciseLogEntry> {
        let mut entries: Vec<ExerciseLogEntry> = self
            .exercise_logs
            .iter()
            .filter(|e| e.exercise_name == self.session_log_exercise_name)
            .cloned()
            .collect();
        entries.sort_by(|a, b| b.logged_at.cmp(&a.logged_at));
        entries
    }

    /// @brief Opens ::Screen::ExerciseSessionLog for one exercise's individual logged sessions,
    /// most recent first — lets a specific mis-logged session be found and corrected or removed,
    /// which ::App::submit_update_exercise (the plan-row updater) doesn't cover: it always logs a
    /// *new* entry rather than editing one already in history.
    /// @param name Exercise name to filter `app.exercise_logs` down to.
    fn open_exercise_session_log(&mut self, name: String) {
        self.session_log_exercise_name = name;
        self.session_log_list_state = ListState::default();
        if !self.session_log_entries().is_empty() {
            self.session_log_list_state.select(Some(0));
        }
        self.screen = Screen::ExerciseSessionLog;
    }

    /// @brief Handles key presses on ::Screen::ExerciseSessionLog: navigate, edit, and delete
    /// individual logged sessions for ::App::session_log_exercise_name.
    /// @param code The key that was pressed.
    fn handle_exercise_session_log_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::ExerciseHistory,
            KeyCode::Up => {
                let i = self.session_log_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.session_log_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.session_log_entries().len();
                let i = self.session_log_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.session_log_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('e') => {
                if let Some(i) = self.session_log_list_state.selected()
                    && let Some(entry) = self.session_log_entries().get(i).cloned() {
                        self.open_edit_exercise_log(&entry);
                    }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.session_log_list_state.selected()
                    && let Some(entry) = self.session_log_entries().get(i).cloned() {
                        let _ = database::delete_exercise_log_entry(&self.db, entry.id);
                        self.refresh_exercise_logs();
                        let new_len = self.session_log_entries().len();
                        if new_len == 0 {
                            self.session_log_list_state.select(None);
                        } else {
                            self.session_log_list_state.select(Some(i.min(new_len - 1)));
                        }
                    }
            }
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::MetricsHistory: navigate readings and delete the selected one.
    /// @param code The key that was pressed.
    fn handle_metrics_history_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::Dashboard,
            KeyCode::Char('g') => self.screen = Screen::MetricsGraph,
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
                if let Some(i) = self.metrics_list_state.selected()
                    && let Some(m) = self.metrics.get(i) {
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
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::MetricsGraph: only exiting back to the history list.
    /// @param code The key that was pressed.
    fn handle_metrics_graph_key(&mut self, code: KeyCode) {
        if let KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left = code {
            self.screen = Screen::MetricsHistory;
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
                if let Some(i) = self.recipe_list_state.selected()
                    && let Some(id) = self.recipes.get(i).map(|r| r.id) {
                        self.open_recipe_detail(id);
                    }
            }
            KeyCode::Char('n') => self.open_add_recipe(),
            KeyCode::Char('f') => {
                if let Some(i) = self.recipe_list_state.selected()
                    && let Some(recipe) = self.recipes.get(i) {
                        let id = recipe.id;
                        let _ = database::toggle_recipe_favorite(&self.db, id);
                        self.refresh_recipes();
                    }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.recipe_list_state.selected()
                    && let Some(recipe) = self.recipes.get(i) {
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
            _ => {}
        }
    }

    /// @brief Handles key presses on ::Screen::RecipeDetail: switch focus between the Ingredients
    /// and Sub-Recipes panes, navigate/attach/detach on whichever is focused, drill into a
    /// sub-recipe, and log the recipe as eaten.
    /// @param code The key that was pressed.
    fn handle_recipe_detail_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                self.screen = Screen::RecipeList;
            }
            KeyCode::Tab => {
                self.recipe_detail_focus = match self.recipe_detail_focus {
                    RecipeDetailFocus::Ingredients => RecipeDetailFocus::Components,
                    RecipeDetailFocus::Components => RecipeDetailFocus::Ingredients,
                };
            }
            KeyCode::Up => match self.recipe_detail_focus {
                RecipeDetailFocus::Ingredients => {
                    let i = self.recipe_ingredient_list_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.recipe_ingredient_list_state.select(Some(i - 1));
                    }
                }
                RecipeDetailFocus::Components => {
                    let i = self.recipe_component_list_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.recipe_component_list_state.select(Some(i - 1));
                    }
                }
            },
            KeyCode::Down => match self.recipe_detail_focus {
                RecipeDetailFocus::Ingredients => {
                    let len = self.current_recipe_ingredients.len();
                    let i = self.recipe_ingredient_list_state.selected().unwrap_or(0);
                    if i + 1 < len {
                        self.recipe_ingredient_list_state.select(Some(i + 1));
                    }
                }
                RecipeDetailFocus::Components => {
                    let len = self.current_recipe_components.len();
                    let i = self.recipe_component_list_state.selected().unwrap_or(0);
                    if i + 1 < len {
                        self.recipe_component_list_state.select(Some(i + 1));
                    }
                }
            },
            KeyCode::Char('n') => match self.recipe_detail_focus {
                RecipeDetailFocus::Ingredients => self.open_pick_ingredient(),
                RecipeDetailFocus::Components => self.open_pick_sub_recipe(),
            },
            KeyCode::Char('l') => {
                self.log_meal_from_recipe();
            }
            KeyCode::Char('f') => {
                if let Some(id) = self.detail_recipe_id {
                    let _ = database::toggle_recipe_favorite(&self.db, id);
                    self.refresh_recipes();
                }
            }
            KeyCode::Char('d') => match self.recipe_detail_focus {
                RecipeDetailFocus::Ingredients => {
                    if let Some(i) = self.recipe_ingredient_list_state.selected()
                        && let Some(ri) = self.current_recipe_ingredients.get(i) {
                            let id = ri.id;
                            let _ = database::remove_recipe_ingredient(&self.db, id);
                            self.refresh_current_recipe_ingredients();
                            self.refresh_current_recipe_nutrition();
                            let new_i = i.saturating_sub(1);
                            if self.current_recipe_ingredients.is_empty() {
                                self.recipe_ingredient_list_state.select(None);
                            } else {
                                self.recipe_ingredient_list_state
                                    .select(Some(new_i.min(self.current_recipe_ingredients.len() - 1)));
                            }
                        }
                }
                RecipeDetailFocus::Components => {
                    let Some(i) = self.recipe_component_list_state.selected() else { return };
                    let Some(rc) = self.current_recipe_components.get(i) else { return };
                    let id = rc.id;
                    let _ = database::remove_recipe_component(&self.db, id);
                    self.refresh_current_recipe_components();
                    self.refresh_current_recipe_nutrition();
                    let new_i = i.saturating_sub(1);
                    if self.current_recipe_components.is_empty() {
                        self.recipe_component_list_state.select(None);
                    } else {
                        self.recipe_component_list_state
                            .select(Some(new_i.min(self.current_recipe_components.len() - 1)));
                    }
                }
            },
            KeyCode::Enter | KeyCode::Right => {
                if self.recipe_detail_focus != RecipeDetailFocus::Components {
                    return;
                }
                let Some(i) = self.recipe_component_list_state.selected() else { return };
                let Some(child_id) = self.current_recipe_components.get(i).map(|c| c.child_recipe_id) else { return };
                self.open_recipe_detail(child_id);
            }
            _ => {}
        }
    }

    /// @brief Refreshes recipes and opens ::Screen::PickSubRecipe with candidates filtered to
    /// exclude the current recipe and anything that would create a cycle.
    ///
    /// A candidate `C` would create a cycle if `C` already (transitively) contains the current
    /// recipe (see ::database::recipe_contains_recipe) — attaching `C` as a sub-recipe would
    /// then close a loop back to the current recipe.
    fn open_pick_sub_recipe(&mut self) {
        self.refresh_recipes();
        let Some(current_id) = self.detail_recipe_id else { return };
        self.sub_recipe_candidates = self
            .recipes
            .iter()
            .filter(|r| r.id != current_id)
            .filter(|r| !database::recipe_contains_recipe(&self.db, r.id, current_id).unwrap_or(true))
            .cloned()
            .collect();
        self.pick_sub_recipe_list_state = ListState::default();
        if !self.sub_recipe_candidates.is_empty() {
            self.pick_sub_recipe_list_state.select(Some(0));
        }
        self.screen = Screen::PickSubRecipe;
    }

    /// @brief Handles key presses on ::Screen::PickSubRecipe: navigate ::App::sub_recipe_candidates and select.
    /// @param code The key that was pressed.
    fn handle_pick_sub_recipe_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => self.screen = Screen::RecipeDetail,
            KeyCode::Up => {
                let i = self.pick_sub_recipe_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.pick_sub_recipe_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Down => {
                let len = self.sub_recipe_candidates.len();
                let i = self.pick_sub_recipe_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    self.pick_sub_recipe_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                let Some(i) = self.pick_sub_recipe_list_state.selected() else { return };
                let Some(r) = self.sub_recipe_candidates.get(i) else { return };
                self.pending_component_child_id = Some(r.id);
                self.open_add_recipe_component();
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
            KeyCode::Char('u') => {
                self.open_set_ingredient_nutrition();
            }
            KeyCode::Char('f') => {
                if let Some(i) = self.ingredient_list_state.selected()
                    && let Some(ingredient) = self.ingredients.get(i) {
                        let id = ingredient.id;
                        let _ = database::toggle_ingredient_favorite(&self.db, id);
                        self.refresh_ingredients();
                    }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.ingredient_list_state.selected()
                    && let Some(ingredient) = self.ingredients.get(i) {
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
                if let Some(i) = self.meal_log_list_state.selected()
                    && let Some(entry) = self.meal_log.get(i) {
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
    /// No-op if `detail_recipe_id` is `None` or no longer resolves to a recipe. Depletes stock
    /// for every ingredient found anywhere in the recipe's sub-recipe tree (see
    /// ::database::get_recipe_ingredients_recursive), not just its direct links — a sub-recipe
    /// used more than once in the tree deducts its ingredients once per use. For each such
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
        let tree_ingredients =
            database::get_recipe_ingredients_recursive(&self.db, recipe.id).unwrap_or_default();

        // Sum by ingredient first, since the same ingredient can appear more than once across
        // the tree (e.g. directly and inside a sub-recipe) — depleting per-occurrence against a
        // stale snapshot would only ever apply the last occurrence's amount.
        let mut total_by_ingredient: HashMap<i64, f64> = HashMap::new();
        for ri in &tree_ingredients {
            if let Some(amount) = ri.amount {
                *total_by_ingredient.entry(ri.ingredient_id).or_insert(0.0) += amount;
            }
        }
        for (ingredient_id, total_amount) in total_by_ingredient {
            if let Some(ingredient) = self.ingredients.iter().find(|i| i.id == ingredient_id)
                && let Some(stock) = ingredient.stock_amount {
                    let new_stock = (stock - total_amount).max(0.0);
                    let _ = database::set_ingredient_stock(
                        &self.db, ingredient.id, Some(new_stock), &ingredient.stock_unit,
                    );
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

    /// @brief Opens the "set nutrition" form (::Screen::SetIngredientNutrition) for the selected library
    /// ingredient, prefilled with its current per-gram macros and density.
    fn open_set_ingredient_nutrition(&mut self) {
        let Some(i) = self.ingredient_list_state.selected() else { return };
        let Some(ingredient) = self.ingredients.get(i) else { return };

        self.editing_ingredient_id = Some(ingredient.id);
        self.input_fields = vec![
            ingredient.calories_per_g.map(|v| v.to_string()).unwrap_or_default(),
            ingredient.protein_per_g.map(|v| v.to_string()).unwrap_or_default(),
            ingredient.fat_per_g.map(|v| v.to_string()).unwrap_or_default(),
            ingredient.fiber_per_g.map(|v| v.to_string()).unwrap_or_default(),
            ingredient.density_g_per_ml.map(|v| v.to_string()).unwrap_or_default(),
        ];
        self.input_labels = vec![
            "Calories per gram (optional)",
            "Protein g per gram (optional)",
            "Fat g per gram (optional)",
            "Fiber g per gram (optional)",
            "Density g/mL (optional, for cup/tbsp/tsp/ml/l units)",
        ];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::SetIngredientNutrition;
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

    /// @brief Opens the "attach sub-recipe" form (::Screen::AddRecipeComponent) with blank amount/unit fields.
    fn open_add_recipe_component(&mut self) {
        self.input_fields = vec![String::new(), String::new()];
        self.input_labels = vec!["Amount (optional)", "Unit (optional)"];
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::AddRecipeComponent;
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
        self.updating_exercise_id = None;
        self.screen = Screen::AddExercise;
    }

    /// @brief Opens the "update exercise" form (::Screen::UpdateExercise) for the highlighted
    /// exercise, prefilled with its current values.
    ///
    /// Added because the only prior way to bump an exercise's working weight/reps was `[n]` Add,
    /// which always inserts a new plan row rather than updating the existing one — the root
    /// cause of duplicate same-name rows piling up in a workout plan. On submit, this both
    /// updates the plan row *and* logs a new history entry with the new values in one action
    /// (see ::App::submit_update_exercise), since a separate "now go press `[l]`" step is too easy
    /// to forget and would silently lose that session's data.
    ///
    /// @param ex The exercise to update (cloned by the caller before this is called, since the
    /// caller borrows it from `self.current_exercises`).
    fn open_update_exercise(&mut self, ex: &Exercise) {
        self.detail_plan_id = Some(ex.workout_plan_id);
        self.current_exercise_type = ex.exercise_type.clone();
        self.updating_exercise_id = Some(ex.id);

        match ex.exercise_type {
            ExerciseType::Strength => {
                self.input_fields = vec![
                    ex.name.clone(),
                    ex.sets.to_string(),
                    ex.reps.to_string(),
                    ex.weight.map(|w| w.to_string()).unwrap_or_default(),
                ];
                self.input_labels = vec!["Exercise Name", "Sets", "Reps", "Weight kg (optional)"];
            }
            ExerciseType::Cardio => {
                self.input_fields = vec![
                    ex.name.clone(),
                    ex.duration_min.map(|d| d.to_string()).unwrap_or_default(),
                    ex.distance_km.map(|d| d.to_string()).unwrap_or_default(),
                ];
                self.input_labels = vec!["Exercise Name", "Duration (min)", "Distance km (optional)"];
            }
        }
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::UpdateExercise;
    }

    /// @brief Opens the "edit session" form (::Screen::EditExerciseLog) for one logged session,
    /// prefilled with its current values, so a mis-logged entry can be corrected without losing
    /// its timestamp or place in history.
    /// @param entry The logged session to edit.
    fn open_edit_exercise_log(&mut self, entry: &ExerciseLogEntry) {
        self.current_exercise_type = entry.exercise_type.clone();
        self.editing_log_id = Some(entry.id);

        match entry.exercise_type {
            ExerciseType::Strength => {
                self.input_fields = vec![
                    entry.exercise_name.clone(),
                    entry.sets.to_string(),
                    entry.reps.to_string(),
                    entry.weight.map(|w| w.to_string()).unwrap_or_default(),
                ];
                self.input_labels = vec!["Exercise Name", "Sets", "Reps", "Weight kg (optional)"];
            }
            ExerciseType::Cardio => {
                self.input_fields = vec![
                    entry.exercise_name.clone(),
                    entry.duration_min.map(|d| d.to_string()).unwrap_or_default(),
                    entry.distance_km.map(|d| d.to_string()).unwrap_or_default(),
                ];
                self.input_labels = vec!["Exercise Name", "Duration (min)", "Distance km (optional)"];
            }
        }
        self.input_index = 0;
        self.input_mode = true;
        self.status_msg = None;
        self.screen = Screen::EditExerciseLog;
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
        self.updating_exercise_id = None;
        self.editing_log_id = None;
        self.screen = match self.screen {
            Screen::AddWorkout => Screen::WorkoutList,
            Screen::AddExercise => Screen::ExerciseTypeSelect,
            Screen::UpdateExercise => Screen::WorkoutDetail,
            Screen::EditExerciseLog => Screen::ExerciseSessionLog,
            Screen::LogMetrics => Screen::Dashboard,
            Screen::AddToLibrary => Screen::PickExercise,
            Screen::AddRecipe => Screen::RecipeList,
            Screen::AddRecipeIngredient => Screen::RecipeDetail,
            Screen::AddRecipeComponent => Screen::RecipeDetail,
            Screen::AddIngredient => {
                if self.adding_ingredient_for_recipe {
                    Screen::PickIngredient
                } else {
                    Screen::IngredientLibrary
                }
            }
            Screen::SetIngredientStock => Screen::IngredientLibrary,
            Screen::SetIngredientNutrition => Screen::IngredientLibrary,
            Screen::SearchByName => Screen::Search,
            _ => Screen::Dashboard,
        };
    }

    /// @brief Dispatches Enter-on-last-field submission to the handler matching the current form screen.
    fn submit_form(&mut self) {
        match self.screen.clone() {
            Screen::AddWorkout => self.submit_add_workout(),
            Screen::AddExercise => self.submit_add_exercise(),
            Screen::UpdateExercise => self.submit_update_exercise(),
            Screen::EditExerciseLog => self.submit_edit_exercise_log(),
            Screen::LogMetrics => self.submit_log_metrics(),
            Screen::AddToLibrary => self.submit_add_to_library(),
            Screen::AddRecipe => self.submit_add_recipe(),
            Screen::AddRecipeIngredient => self.submit_add_recipe_ingredient(),
            Screen::AddRecipeComponent => self.submit_add_recipe_component(),
            Screen::AddIngredient => self.submit_add_ingredient(),
            Screen::SetIngredientStock => self.submit_set_ingredient_stock(),
            Screen::SetIngredientNutrition => self.submit_set_ingredient_nutrition(),
            Screen::SearchByName => self.submit_search_by_name(),
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

        let perf = ExercisePerformance {
            exercise_type: self.current_exercise_type.clone(),
            sets,
            reps,
            weight,
            duration_min,
            distance_km,
        };
        if let Some(plan_id) = self.detail_plan_id
            && let Err(e) = database::add_exercise(&self.db, plan_id, &name, &perf) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.refresh_current_exercises();
        self.refresh_plans();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::WorkoutDetail;
    }

    /// @brief Validates and saves the "update exercise" form for ::App::updating_exercise_id,
    /// updating the existing plan row in place *and* logging the new values as a fresh session in
    /// history, then returns to ::Screen::WorkoutDetail.
    ///
    /// The plan-row update and the history log happen together in this one action deliberately:
    /// this used to be a plain "edit" that only touched the plan row, requiring a separate `[l]`
    /// Log Session press to actually record the change — a step easy to forget, silently losing
    /// that session's data. Logging here always *appends* a new `exercise_log` row; no existing
    /// history row is ever touched, so nothing is lost even if you update the same exercise
    /// several times in a row.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg and leaving the form
    /// open. Numeric fields that fail to parse fall back to defaults (sets=3, reps=10) or `None`
    /// rather than blocking submission, matching ::App::submit_add_exercise.
    fn submit_update_exercise(&mut self) {
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

        let perf = ExercisePerformance {
            exercise_type: self.current_exercise_type.clone(),
            sets,
            reps,
            weight,
            duration_min,
            distance_km,
        };
        if let Some(id) = self.updating_exercise_id
            && let Err(e) = database::update_exercise(&self.db, id, &name, &perf) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        if let Err(e) = database::log_exercise_session(&self.db, &name, &perf) {
            self.status_msg = Some(format!("Error: {e}"));
            return;
        }
        self.updating_exercise_id = None;
        self.refresh_current_exercises();
        self.refresh_plans();
        self.refresh_exercise_logs();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::WorkoutDetail;
    }

    /// @brief Validates and saves the "edit session" form for ::App::editing_log_id, updating
    /// that logged session in place (name/type/performance, but not its original timestamp), then
    /// returns to ::Screen::ExerciseSessionLog.
    ///
    /// Rejects an empty (after trimming) name by setting ::App::status_msg and leaving the form
    /// open. Numeric fields that fail to parse fall back to defaults (sets=3, reps=10) or `None`
    /// rather than blocking submission, matching ::App::submit_add_exercise.
    fn submit_edit_exercise_log(&mut self) {
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

        let perf = ExercisePerformance {
            exercise_type: self.current_exercise_type.clone(),
            sets,
            reps,
            weight,
            duration_min,
            distance_km,
        };
        if let Some(id) = self.editing_log_id
            && let Err(e) = database::update_exercise_log_entry(&self.db, id, &name, &perf) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.editing_log_id = None;
        self.session_log_exercise_name = name;
        self.refresh_exercise_logs();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::ExerciseSessionLog;
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

        if let (Some(recipe_id), Some(ingredient_id)) = (self.detail_recipe_id, self.pending_ingredient_id)
            && let Err(e) = database::add_recipe_ingredient(&self.db, recipe_id, ingredient_id, amount, &unit) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.pending_ingredient_id = None;
        self.refresh_current_recipe_ingredients();
        self.refresh_current_recipe_nutrition();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::RecipeDetail;
    }

    /// @brief Saves the "attach sub-recipe" form, linking ::App::pending_component_child_id to the open recipe, then returns to ::Screen::RecipeDetail.
    ///
    /// Both fields are optional: an unparsable or blank amount is stored as `None`.
    fn submit_add_recipe_component(&mut self) {
        let amount: Option<f64> = self.input_fields[0].trim().parse().ok();
        let unit = self.input_fields[1].trim().to_string();

        if let (Some(parent_id), Some(child_id)) = (self.detail_recipe_id, self.pending_component_child_id)
            && let Err(e) = database::add_recipe_component(&self.db, parent_id, child_id, amount, &unit) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.pending_component_child_id = None;
        self.refresh_current_recipe_components();
        self.refresh_current_recipe_nutrition();
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

        if let Some(id) = self.editing_ingredient_id
            && let Err(e) = database::set_ingredient_stock(&self.db, id, amount, &unit) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.editing_ingredient_id = None;
        self.refresh_ingredients();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::IngredientLibrary;
    }

    /// @brief Saves the "set nutrition" form for ::App::editing_ingredient_id, then returns to ::Screen::IngredientLibrary.
    ///
    /// Every field is optional: a blank or unparsable value clears that piece of data (`None`)
    /// rather than blocking submission.
    fn submit_set_ingredient_nutrition(&mut self) {
        let calories_per_g: Option<f64> = self.input_fields[0].trim().parse().ok();
        let protein_per_g: Option<f64> = self.input_fields[1].trim().parse().ok();
        let fat_per_g: Option<f64> = self.input_fields[2].trim().parse().ok();
        let fiber_per_g: Option<f64> = self.input_fields[3].trim().parse().ok();
        let density_g_per_ml: Option<f64> = self.input_fields[4].trim().parse().ok();

        if let Some(id) = self.editing_ingredient_id
            && let Err(e) = database::set_ingredient_nutrition(
                &self.db, id, calories_per_g, protein_per_g, fat_per_g, fiber_per_g, density_g_per_ml,
            ) {
                self.status_msg = Some(format!("Error: {e}"));
                return;
            }
        self.editing_ingredient_id = None;
        self.refresh_ingredients();
        self.input_mode = false;
        self.status_msg = None;
        self.screen = Screen::IngredientLibrary;
    }
}

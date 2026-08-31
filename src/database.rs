//! @file database.rs
//! @brief SQLite data-access layer: schema management and CRUD queries for
//!        workout plans, exercises, exercise history, the exercise library,
//!        and personal metrics.

use rusqlite::{params, Connection, Result};

use crate::models::{
    self, Exercise, ExerciseLogEntry, ExercisePerformance, ExerciseType, Ingredient,
    IngredientType, MealLogEntry, PersonalMetric, Recipe, RecipeComponent, RecipeIngredient,
    RecipeNutrition, RecipeType, WorkoutPlan,
};

/// @brief Opens (creating if necessary) the on-disk SQLite database and brings it up to date.
///
/// Opens `rusticbuddy.db` in the current working directory, enables
/// foreign-key enforcement, creates any missing tables via ::init_schema,
/// and applies any pending column additions via ::run_migrations.
///
/// @return An open ::Connection ready for use, or an `rusqlite` error if the
///         file cannot be opened or the schema statements fail.
pub fn open() -> Result<Connection> {
    let conn = Connection::open("rusticbuddy.db")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    init_schema(&conn)?;
    run_migrations(&conn)?;
    Ok(conn)
}

/// @brief Creates all application tables if they do not already exist.
/// @param conn Open database connection to operate on.
/// @return `Ok(())` on success, or an `rusqlite` error if any `CREATE TABLE` fails.
fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workout_plans (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            name      TEXT    NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS exercises (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            workout_plan_id INTEGER NOT NULL REFERENCES workout_plans(id) ON DELETE CASCADE,
            name            TEXT    NOT NULL,
            exercise_type   TEXT    NOT NULL DEFAULT 'strength',
            sets            INTEGER NOT NULL DEFAULT 3,
            reps            INTEGER NOT NULL DEFAULT 10,
            weight          REAL,
            duration_min    REAL,
            distance_km     REAL
        );
        CREATE TABLE IF NOT EXISTS exercise_log (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            exercise_name TEXT    NOT NULL,
            exercise_type TEXT    NOT NULL DEFAULT 'strength',
            sets          INTEGER NOT NULL,
            reps          INTEGER NOT NULL,
            weight        REAL,
            duration_min  REAL,
            distance_km   REAL,
            logged_at     TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS exercise_library (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT    NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS personal_metrics (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            weight_lbs   REAL,
            heart_rate   INTEGER,
            bp_systolic  INTEGER,
            bp_diastolic INTEGER
        );
        CREATE TABLE IF NOT EXISTS ingredients (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            name     TEXT    NOT NULL UNIQUE,
            category TEXT    NOT NULL DEFAULT 'Pending'
        );
        CREATE TABLE IF NOT EXISTS recipes (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL,
            category     TEXT    NOT NULL DEFAULT 'Pending',
            prep_min     INTEGER NOT NULL DEFAULT 0,
            cook_min     INTEGER NOT NULL DEFAULT 0,
            cook_temp    INTEGER NOT NULL DEFAULT 0,
            instructions TEXT    NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS recipe_ingredients (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            recipe_id     INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
            ingredient_id INTEGER NOT NULL REFERENCES ingredients(id) ON DELETE CASCADE,
            amount        REAL,
            unit          TEXT    NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS recipe_components (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
            child_recipe_id  INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
            amount           REAL,
            unit             TEXT    NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS meal_log (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            meal_name  TEXT    NOT NULL,
            category   TEXT    NOT NULL DEFAULT 'Pending',
            servings   REAL    NOT NULL DEFAULT 1,
            logged_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );",
    )
}

/// @brief Applies additive schema migrations for databases created before newer columns existed.
///
/// Each `ALTER TABLE ... ADD COLUMN` is attempted unconditionally; failures
/// (e.g. because the column already exists) are deliberately discarded so
/// this is safe to run on every startup, on both fresh and pre-existing
/// database files.
///
/// @param conn Open database connection to operate on.
/// @return Always `Ok(())`.
fn run_migrations(conn: &Connection) -> Result<()> {
    // Each ALTER TABLE is silently ignored if the column already exists (new databases).
    let _ = conn.execute_batch("ALTER TABLE exercises ADD COLUMN exercise_type TEXT NOT NULL DEFAULT 'strength';");
    let _ = conn.execute_batch("ALTER TABLE exercises ADD COLUMN duration_min REAL;");
    let _ = conn.execute_batch("ALTER TABLE exercises ADD COLUMN distance_km REAL;");
    let _ = conn.execute_batch("ALTER TABLE exercise_log ADD COLUMN exercise_type TEXT NOT NULL DEFAULT 'strength';");
    let _ = conn.execute_batch("ALTER TABLE exercise_log ADD COLUMN duration_min REAL;");
    let _ = conn.execute_batch("ALTER TABLE exercise_log ADD COLUMN distance_km REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN stock_amount REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN stock_unit TEXT NOT NULL DEFAULT '';");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN calories_per_g REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN protein_per_g REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN fat_per_g REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN fiber_per_g REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN density_g_per_ml REAL;");
    let _ = conn.execute_batch("ALTER TABLE recipe_ingredients ADD COLUMN grams REAL;");
    let _ = conn.execute_batch("ALTER TABLE recipe_components ADD COLUMN grams REAL;");
    let _ = conn.execute_batch("ALTER TABLE ingredients ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;");
    let _ = conn.execute_batch("ALTER TABLE recipes ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;");
    Ok(())
}

// --- Workout Plans ---

/// @brief Fetches all workout plans, alphabetically by name.
/// @param conn Open database connection.
/// @return All rows of `workout_plans`.
pub fn get_workout_plans(conn: &Connection) -> Result<Vec<WorkoutPlan>> {
    let mut stmt =
        conn.prepare("SELECT id, name, is_active FROM workout_plans ORDER BY name")?;
    stmt.query_map([], |row| {
        Ok(WorkoutPlan {
            id: row.get(0)?,
            name: row.get(1)?,
            is_active: row.get::<_, i32>(2)? != 0,
        })
    })?
    .collect()
}

/// @brief Inserts a new, inactive workout plan.
/// @param conn Open database connection.
/// @param name Display name for the new plan.
/// @return `Ok(())` on success.
pub fn create_workout_plan(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("INSERT INTO workout_plans (name) VALUES (?1)", params![name])?;
    Ok(())
}

/// @brief Deletes a workout plan and (via `ON DELETE CASCADE`) its exercises.
/// @param conn Open database connection.
/// @param id Primary key of the plan to delete.
/// @return `Ok(())` on success.
pub fn delete_workout_plan(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM workout_plans WHERE id = ?1", params![id])?;
    Ok(())
}

/// @brief Marks a single workout plan as active, deactivating all others.
/// @param conn Open database connection.
/// @param id Primary key of the plan to activate.
/// @return `Ok(())` on success.
pub fn set_active_plan(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE workout_plans SET is_active = 0", [])?;
    conn.execute(
        "UPDATE workout_plans SET is_active = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

// --- Exercises ---

/// @brief Fetches all exercises belonging to a given workout plan, in insertion order.
/// @param conn Open database connection.
/// @param plan_id Primary key of the owning ::WorkoutPlan.
/// @return Exercises for that plan; unrecognized `exercise_type` values fall back to Strength.
pub fn get_exercises(conn: &Connection, plan_id: i64) -> Result<Vec<Exercise>> {
    let mut stmt = conn.prepare(
        "SELECT id, workout_plan_id, name, exercise_type, sets, reps, weight, duration_min, distance_km
         FROM exercises WHERE workout_plan_id = ?1 ORDER BY id",
    )?;
    stmt.query_map(params![plan_id], |row| {
        let type_str: String = row.get(3)?;
        Ok(Exercise {
            id: row.get(0)?,
            workout_plan_id: row.get(1)?,
            name: row.get(2)?,
            exercise_type: type_str.parse().unwrap_or(ExerciseType::Strength),
            sets: row.get(4)?,
            reps: row.get(5)?,
            weight: row.get(6)?,
            duration_min: row.get(7)?,
            distance_km: row.get(8)?,
        })
    })?
    .collect()
}

/// @brief Inserts a new exercise into a workout plan.
/// @param conn Open database connection.
/// @param plan_id Primary key of the owning ::WorkoutPlan.
/// @param name Exercise name.
/// @param exercise_type Whether this is a strength or cardio exercise.
/// @param perf Strength/cardio performance fields (sets/reps/weight or duration/distance).
/// @return `Ok(())` on success.
pub fn add_exercise(
    conn: &Connection,
    plan_id: i64,
    name: &str,
    perf: &ExercisePerformance,
) -> Result<()> {
    conn.execute(
        "INSERT INTO exercises (workout_plan_id, name, exercise_type, sets, reps, weight, duration_min, distance_km)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            plan_id, name, perf.exercise_type.as_str(), perf.sets, perf.reps,
            perf.weight, perf.duration_min, perf.distance_km,
        ],
    )?;
    Ok(())
}

/// @brief Deletes a single exercise from its workout plan.
/// @param conn Open database connection.
/// @param id Primary key of the exercise to delete.
/// @return `Ok(())` on success.
pub fn delete_exercise(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM exercises WHERE id = ?1", params![id])?;
    Ok(())
}

/// @brief Records a completed exercise session in the history log.
///
/// Writes a denormalized snapshot into `exercise_log` (name and type are
/// copied by value) so history remains intact even if the source ::Exercise
/// is later edited or deleted.
///
/// @param conn Open database connection.
/// @param name Name of the exercise performed.
/// @param perf Strength/cardio performance fields (sets/reps/weight or duration/distance).
/// @return `Ok(())` on success.
pub fn log_exercise_session(
    conn: &Connection,
    name: &str,
    perf: &ExercisePerformance,
) -> Result<()> {
    conn.execute(
        "INSERT INTO exercise_log (exercise_name, exercise_type, sets, reps, weight, duration_min, distance_km)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            name, perf.exercise_type.as_str(), perf.sets, perf.reps,
            perf.weight, perf.duration_min, perf.distance_km,
        ],
    )?;
    Ok(())
}

/// @brief Fetches the full exercise history, grouped by exercise name and ordered chronologically within each group.
/// @param conn Open database connection.
/// @return All rows of `exercise_log`, ordered by `(exercise_name, logged_at)`.
pub fn get_exercise_logs(conn: &Connection) -> Result<Vec<ExerciseLogEntry>> {
    let mut stmt = conn.prepare(
        "SELECT exercise_name, exercise_type, sets, reps, weight, duration_min, distance_km, logged_at
         FROM exercise_log ORDER BY exercise_name, logged_at",
    )?;
    stmt.query_map([], |row| {
        let type_str: String = row.get(1)?;
        Ok(ExerciseLogEntry {
            exercise_name: row.get(0)?,
            exercise_type: type_str.parse().unwrap_or(ExerciseType::Strength),
            sets: row.get(2)?,
            reps: row.get(3)?,
            weight: row.get(4)?,
            duration_min: row.get(5)?,
            distance_km: row.get(6)?,
            logged_at: row.get(7)?,
        })
    })?
    .collect()
}

/// @brief Fetches all exercise names in the reusable library, alphabetically.
/// @param conn Open database connection.
/// @return Names from `exercise_library`, used to populate the "pick exercise" screen.
pub fn get_library_exercises(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM exercise_library ORDER BY name")?;
    stmt.query_map([], |row| row.get(0))?.collect()
}

/// @brief Adds a new exercise name to the reusable library.
/// @param conn Open database connection.
/// @param name Exercise name to add; must be unique (`exercise_library.name` is `UNIQUE`).
/// @return `Ok(())` on success, or an `rusqlite` error if the name already exists.
pub fn add_to_library(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("INSERT INTO exercise_library (name) VALUES (?1)", params![name])?;
    Ok(())
}

/// @brief Counts how many exercises each workout plan has.
/// @param conn Open database connection.
/// @return A map from `workout_plan_id` to exercise count, for plans with at least one exercise.
pub fn get_exercise_counts(conn: &Connection) -> Result<std::collections::HashMap<i64, usize>> {
    let mut stmt = conn.prepare(
        "SELECT workout_plan_id, COUNT(*) FROM exercises GROUP BY workout_plan_id",
    )?;
    let mut counts = std::collections::HashMap::new();
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        let (plan_id, count) = row?;
        counts.insert(plan_id, count as usize);
    }
    Ok(counts)
}

// --- Personal Metrics ---

/// @brief Records a new personal-metrics reading, timestamped by SQLite at insert time.
/// @param conn Open database connection.
/// @param weight_kg Body weight in kilograms, if measured.
/// @param heart_rate Resting heart rate in bpm, if measured.
/// @param bp_systolic Systolic blood pressure in mmHg, if measured.
/// @param bp_diastolic Diastolic blood pressure in mmHg, if measured.
/// @return `Ok(())` on success.
/// @note Despite the `weight_lbs` column name (a naming holdover), the value stored is kilograms.
pub fn log_metric(
    conn: &Connection,
    weight_kg: Option<f64>,
    heart_rate: Option<i32>,
    bp_systolic: Option<i32>,
    bp_diastolic: Option<i32>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO personal_metrics (weight_lbs, heart_rate, bp_systolic, bp_diastolic)
         VALUES (?1, ?2, ?3, ?4)",
        params![weight_kg, heart_rate, bp_systolic, bp_diastolic],
    )?;
    Ok(())
}

/// @brief Deletes a single personal-metrics reading.
/// @param conn Open database connection.
/// @param id Primary key of the reading to delete.
/// @return `Ok(())` on success.
pub fn delete_metric(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM personal_metrics WHERE id = ?1", params![id])?;
    Ok(())
}

/// @brief Fetches the most recent personal-metrics readings, newest first.
/// @param conn Open database connection.
/// @param limit Maximum number of readings to return.
/// @return Up to `limit` rows from `personal_metrics`, ordered by `recorded_at DESC`.
pub fn get_metrics(conn: &Connection, limit: i64) -> Result<Vec<PersonalMetric>> {
    let mut stmt = conn.prepare(
        "SELECT id, recorded_at, weight_lbs, heart_rate, bp_systolic, bp_diastolic
         FROM personal_metrics ORDER BY recorded_at DESC LIMIT ?1",
    )?;
    stmt.query_map(params![limit], |row| {
        Ok(PersonalMetric {
            id: row.get(0)?,
            recorded_at: row.get(1)?,
            weight_kg: row.get(2)?,
            heart_rate: row.get(3)?,
            bp_systolic: row.get(4)?,
            bp_diastolic: row.get(5)?,
        })
    })?
    .collect()
}

// --- Ingredients ---

/// @brief Fetches all ingredients in the library, alphabetically by name.
/// @param conn Open database connection.
/// @return All rows of `ingredients`; unrecognized `category` values fall back to Pending.
pub fn get_ingredients(conn: &Connection) -> Result<Vec<Ingredient>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, stock_amount, stock_unit,
                calories_per_g, protein_per_g, fat_per_g, fiber_per_g, density_g_per_ml, is_favorite
         FROM ingredients ORDER BY name",
    )?;
    stmt.query_map([], |row| {
        let category_str: String = row.get(2)?;
        Ok(Ingredient {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(IngredientType::Pending),
            stock_amount: row.get(3)?,
            stock_unit: row.get(4)?,
            calories_per_g: row.get(5)?,
            protein_per_g: row.get(6)?,
            fat_per_g: row.get(7)?,
            fiber_per_g: row.get(8)?,
            density_g_per_ml: row.get(9)?,
            is_favorite: row.get::<_, i64>(10)? != 0,
        })
    })?
    .collect()
}

/// @brief Sets an ingredient's per-gram nutrition data and volume-conversion density.
/// @param conn Open database connection.
/// @param id Primary key of the ingredient to update.
/// @param calories_per_g Calories per gram, or `None` to clear.
/// @param protein_per_g Protein (g) per gram, or `None` to clear.
/// @param fat_per_g Fat (g) per gram, or `None` to clear.
/// @param fiber_per_g Fiber (g) per gram, or `None` to clear.
/// @param density_g_per_ml Grams per milliliter (used to convert volume-unit amounts to grams
/// in recipes), or `None` to clear.
/// @return `Ok(())` on success.
pub fn set_ingredient_nutrition(
    conn: &Connection,
    id: i64,
    calories_per_g: Option<f64>,
    protein_per_g: Option<f64>,
    fat_per_g: Option<f64>,
    fiber_per_g: Option<f64>,
    density_g_per_ml: Option<f64>,
) -> Result<()> {
    conn.execute(
        "UPDATE ingredients
         SET calories_per_g = ?1, protein_per_g = ?2, fat_per_g = ?3, fiber_per_g = ?4, density_g_per_ml = ?5
         WHERE id = ?6",
        params![calories_per_g, protein_per_g, fat_per_g, fiber_per_g, density_g_per_ml, id],
    )?;
    Ok(())
}

/// @brief Sets an ingredient's on-hand stock quantity and unit.
/// @param conn Open database connection.
/// @param id Primary key of the ingredient to update.
/// @param amount Quantity on hand, or `None` if no longer tracked.
/// @param unit Unit of measure for `amount`.
/// @return `Ok(())` on success.
pub fn set_ingredient_stock(conn: &Connection, id: i64, amount: Option<f64>, unit: &str) -> Result<()> {
    conn.execute(
        "UPDATE ingredients SET stock_amount = ?1, stock_unit = ?2 WHERE id = ?3",
        params![amount, unit, id],
    )?;
    Ok(())
}

/// @brief Toggles an ingredient's favorited status.
/// @param conn Open database connection.
/// @param id Primary key of the ingredient to toggle.
/// @return `Ok(())` on success.
pub fn toggle_ingredient_favorite(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE ingredients SET is_favorite = NOT is_favorite WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// @brief Inserts a new ingredient into the library.
/// @param conn Open database connection.
/// @param name Ingredient name; must be unique (`ingredients.name` is `UNIQUE`).
/// @param category Classification for the new ingredient.
/// @return `Ok(())` on success, or an `rusqlite` error if the name already exists.
pub fn create_ingredient(conn: &Connection, name: &str, category: &IngredientType) -> Result<()> {
    conn.execute(
        "INSERT INTO ingredients (name, category) VALUES (?1, ?2)",
        params![name, category.as_str()],
    )?;
    Ok(())
}

/// @brief Deletes an ingredient from the library (and, via cascade, any recipe links to it).
/// @param conn Open database connection.
/// @param id Primary key of the ingredient to delete.
/// @return `Ok(())` on success.
pub fn delete_ingredient(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM ingredients WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Recipes ---

/// @brief Fetches all recipes, alphabetically by name.
/// @param conn Open database connection.
/// @return All rows of `recipes`; unrecognized `category` values fall back to Pending.
pub fn get_recipes(conn: &Connection) -> Result<Vec<Recipe>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, prep_min, cook_min, cook_temp, instructions, is_favorite
         FROM recipes ORDER BY name",
    )?;
    stmt.query_map([], |row| {
        let category_str: String = row.get(2)?;
        Ok(Recipe {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            prep_min: row.get(3)?,
            cook_min: row.get(4)?,
            cook_temp: row.get(5)?,
            instructions: row.get(6)?,
            is_favorite: row.get::<_, i64>(7)? != 0,
        })
    })?
    .collect()
}

/// @brief Inserts a new recipe.
/// @param conn Open database connection.
/// @param name Recipe name.
/// @param category Cooking-method classification.
/// @param prep_min Preparation time in minutes.
/// @param cook_min Cooking time in minutes.
/// @param cook_temp Cooking temperature in degrees.
/// @param instructions Step-by-step cooking instructions.
/// @return The `id` of the newly inserted recipe.
pub fn create_recipe(
    conn: &Connection,
    name: &str,
    category: &RecipeType,
    prep_min: i32,
    cook_min: i32,
    cook_temp: i32,
    instructions: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO recipes (name, category, prep_min, cook_min, cook_temp, instructions)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, category.as_str(), prep_min, cook_min, cook_temp, instructions],
    )?;
    Ok(conn.last_insert_rowid())
}

/// @brief Deletes a recipe and (via `ON DELETE CASCADE`) its ingredient links.
/// @param conn Open database connection.
/// @param id Primary key of the recipe to delete.
/// @return `Ok(())` on success.
pub fn delete_recipe(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM recipes WHERE id = ?1", params![id])?;
    Ok(())
}

/// @brief Toggles a recipe's favorited status.
/// @param conn Open database connection.
/// @param id Primary key of the recipe to toggle.
/// @return `Ok(())` on success.
pub fn toggle_recipe_favorite(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE recipes SET is_favorite = NOT is_favorite WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// @brief Fetches all ingredients attached to a recipe, joined with each ingredient's name.
/// @param conn Open database connection.
/// @param recipe_id Primary key of the owning ::Recipe.
/// @return Ingredient links for that recipe, in insertion order.
pub fn get_recipe_ingredients(conn: &Connection, recipe_id: i64) -> Result<Vec<RecipeIngredient>> {
    let mut stmt = conn.prepare(
        "SELECT ri.id, ri.ingredient_id, i.name, ri.amount, ri.unit, ri.grams
         FROM recipe_ingredients ri
         JOIN ingredients i ON i.id = ri.ingredient_id
         WHERE ri.recipe_id = ?1 ORDER BY ri.id",
    )?;
    stmt.query_map(params![recipe_id], |row| {
        Ok(RecipeIngredient {
            id: row.get(0)?,
            ingredient_id: row.get(1)?,
            name: row.get(2)?,
            amount: row.get(3)?,
            unit: row.get(4)?,
            grams: row.get(5)?,
        })
    })?
    .collect()
}

/// @brief Attaches an existing library ingredient to a recipe.
///
/// `amount`/`unit` are converted to grams at attach time (see `models::grams_for`), using the
/// ingredient's `density_g_per_ml` for volume units, and stored alongside for nutrition roll-up.
///
/// @param conn Open database connection.
/// @param recipe_id Primary key of the owning ::Recipe.
/// @param ingredient_id Primary key of the ::Ingredient to attach.
/// @param amount Quantity required, if specified.
/// @param unit Unit of measure for `amount`.
/// @return `Ok(())` on success.
pub fn add_recipe_ingredient(
    conn: &Connection,
    recipe_id: i64,
    ingredient_id: i64,
    amount: Option<f64>,
    unit: &str,
) -> Result<()> {
    let grams = match amount {
        Some(a) => {
            let density: Option<f64> = conn
                .query_row(
                    "SELECT density_g_per_ml FROM ingredients WHERE id = ?1",
                    params![ingredient_id],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            models::grams_for(a, unit, density)
        }
        None => None,
    };
    conn.execute(
        "INSERT INTO recipe_ingredients (recipe_id, ingredient_id, amount, unit, grams) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![recipe_id, ingredient_id, amount, unit, grams],
    )?;
    Ok(())
}

/// @brief Detaches a single ingredient link from a recipe.
/// @param conn Open database connection.
/// @param id Primary key of the `recipe_ingredients` row to delete.
/// @return `Ok(())` on success.
pub fn remove_recipe_ingredient(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM recipe_ingredients WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Recipe Components (sub-recipes) ---

/// @brief Fetches all sub-recipes attached to a recipe, joined with each child recipe's name.
/// @param conn Open database connection.
/// @param parent_recipe_id Primary key of the owning ::Recipe.
/// @return Component links for that recipe, in insertion order.
pub fn get_recipe_components(conn: &Connection, parent_recipe_id: i64) -> Result<Vec<RecipeComponent>> {
    let mut stmt = conn.prepare(
        "SELECT rc.id, rc.child_recipe_id, r.name, rc.amount, rc.unit, rc.grams
         FROM recipe_components rc
         JOIN recipes r ON r.id = rc.child_recipe_id
         WHERE rc.parent_recipe_id = ?1 ORDER BY rc.id",
    )?;
    stmt.query_map(params![parent_recipe_id], |row| {
        Ok(RecipeComponent {
            id: row.get(0)?,
            child_recipe_id: row.get(1)?,
            name: row.get(2)?,
            amount: row.get(3)?,
            unit: row.get(4)?,
            grams: row.get(5)?,
        })
    })?
    .collect()
}

/// @brief Attaches an existing recipe as a sub-recipe (component) of another recipe.
///
/// `amount`/`unit` are converted to grams at attach time (see `models::grams_for`) and stored
/// alongside for nutrition roll-up. Unlike ingredients, a recipe has no density of its own, so
/// only mass units (g/kg/oz/lb) convert here â€” a volume-unit amount stores no grams.
///
/// @param conn Open database connection.
/// @param parent_recipe_id Primary key of the owning ::Recipe.
/// @param child_recipe_id Primary key of the ::Recipe to attach as a component.
/// @param amount Quantity of the sub-recipe required, if specified.
/// @param unit Unit of measure for `amount`.
/// @return `Ok(())` on success.
pub fn add_recipe_component(
    conn: &Connection,
    parent_recipe_id: i64,
    child_recipe_id: i64,
    amount: Option<f64>,
    unit: &str,
) -> Result<()> {
    let grams = amount.and_then(|a| models::grams_for(a, unit, None));
    conn.execute(
        "INSERT INTO recipe_components (parent_recipe_id, child_recipe_id, amount, unit, grams) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![parent_recipe_id, child_recipe_id, amount, unit, grams],
    )?;
    Ok(())
}

/// @brief Detaches a single sub-recipe link from a recipe.
/// @param conn Open database connection.
/// @param id Primary key of the `recipe_components` row to delete.
/// @return `Ok(())` on success.
pub fn remove_recipe_component(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM recipe_components WHERE id = ?1", params![id])?;
    Ok(())
}

/// @brief Checks whether `candidate_id` is reachable from `ancestor_id` through `recipe_components`.
///
/// Used to prevent cycles: before attaching `candidate_id` as a sub-recipe of `ancestor_id`'s
/// recipe, callers instead check `recipe_contains_recipe(candidate_id, ancestor_id)` â€” if
/// `ancestor_id` is already a descendant of `candidate_id`, attaching would close a loop.
///
/// @param conn Open database connection.
/// @param ancestor_id Recipe to walk the sub-recipe tree from.
/// @param candidate_id Recipe to search for within that tree (the seed `ancestor_id` itself counts).
/// @return `true` if `candidate_id` is `ancestor_id` or one of its (possibly transitive) sub-recipes.
pub fn recipe_contains_recipe(conn: &Connection, ancestor_id: i64, candidate_id: i64) -> Result<bool> {
    conn.query_row(
        "WITH RECURSIVE tree(member_id) AS (
            SELECT ?1
            UNION
            SELECT rc.child_recipe_id FROM recipe_components rc JOIN tree t ON rc.parent_recipe_id = t.member_id
         )
         SELECT EXISTS(SELECT 1 FROM tree WHERE member_id = ?2)",
        params![ancestor_id, candidate_id],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    )
}

/// @brief Fetches every ingredient attached anywhere in a recipe's sub-recipe tree (itself plus all descendants).
///
/// Unlike ::get_recipe_ingredients, this is not deduplicated by ingredient: a sub-recipe used
/// twice in the tree contributes its ingredients twice, so callers doing stock depletion
/// deplete the correct total amount.
///
/// @param conn Open database connection.
/// @param root_recipe_id Primary key of the top-level ::Recipe to walk from.
/// @return Every `recipe_ingredients` row found at any depth under `root_recipe_id`.
pub fn get_recipe_ingredients_recursive(conn: &Connection, root_recipe_id: i64) -> Result<Vec<RecipeIngredient>> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE tree(member_id) AS (
            SELECT ?1
            UNION
            SELECT rc.child_recipe_id FROM recipe_components rc JOIN tree t ON rc.parent_recipe_id = t.member_id
         )
         SELECT ri.id, ri.ingredient_id, i.name, ri.amount, ri.unit, ri.grams
         FROM tree t
         JOIN recipe_ingredients ri ON ri.recipe_id = t.member_id
         JOIN ingredients i ON i.id = ri.ingredient_id",
    )?;
    stmt.query_map(params![root_recipe_id], |row| {
        Ok(RecipeIngredient {
            id: row.get(0)?,
            ingredient_id: row.get(1)?,
            name: row.get(2)?,
            amount: row.get(3)?,
            unit: row.get(4)?,
            grams: row.get(5)?,
        })
    })?
    .collect()
}

/// @brief Computes a recipe's nutritional density, rolled up from its direct ingredients and,
/// recursively, its sub-recipes.
///
/// Each direct `recipe_ingredients` row with a known `grams` contributes `grams * macro_per_g`
/// to that macro's running total; each `recipe_components` row with a known `grams` contributes
/// `grams * child.macro_per_g`, where `child` is this same computation run on the sub-recipe.
/// `total_grams` sums every contributing `grams` value. A macro is only `Some` if at least one
/// contributor actually had that value entered â€” a macro with zero known contributors reports
/// `None` rather than a misleading `Some(0.0)`. Cycle-safety is inherited from
/// ::recipe_contains_recipe's insert-time check, so this recursion is bounded by the tree's real
/// depth.
///
/// @param conn Open database connection.
/// @param recipe_id Primary key of the ::Recipe to compute nutrition for.
/// @return The recipe's computed ::RecipeNutrition; all fields default (zero/`None`) if nothing
///         was computable.
pub fn compute_recipe_nutrition(conn: &Connection, recipe_id: i64) -> Result<RecipeNutrition> {
    let mut total_grams = 0.0;
    let (mut total_calories, mut total_protein, mut total_fat, mut total_fiber) = (0.0, 0.0, 0.0, 0.0);
    let (mut has_calories, mut has_protein, mut has_fat, mut has_fiber) = (false, false, false, false);

    let mut stmt = conn.prepare(
        "SELECT ri.grams, i.calories_per_g, i.protein_per_g, i.fat_per_g, i.fiber_per_g
         FROM recipe_ingredients ri
         JOIN ingredients i ON i.id = ri.ingredient_id
         WHERE ri.recipe_id = ?1",
    )?;
    let rows = stmt.query_map(params![recipe_id], |row| {
        Ok((
            row.get::<_, Option<f64>>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?,
            row.get::<_, Option<f64>>(3)?,
            row.get::<_, Option<f64>>(4)?,
        ))
    })?;
    for row in rows {
        let (grams, calories_per_g, protein_per_g, fat_per_g, fiber_per_g) = row?;
        let Some(g) = grams else { continue };
        total_grams += g;
        if let Some(v) = calories_per_g { total_calories += g * v; has_calories = true; }
        if let Some(v) = protein_per_g { total_protein += g * v; has_protein = true; }
        if let Some(v) = fat_per_g { total_fat += g * v; has_fat = true; }
        if let Some(v) = fiber_per_g { total_fiber += g * v; has_fiber = true; }
    }

    for component in get_recipe_components(conn, recipe_id)? {
        let Some(g) = component.grams else { continue };
        let child = compute_recipe_nutrition(conn, component.child_recipe_id)?;
        total_grams += g;
        if let Some(v) = child.calories_per_g { total_calories += g * v; has_calories = true; }
        if let Some(v) = child.protein_per_g { total_protein += g * v; has_protein = true; }
        if let Some(v) = child.fat_per_g { total_fat += g * v; has_fat = true; }
        if let Some(v) = child.fiber_per_g { total_fiber += g * v; has_fiber = true; }
    }

    if total_grams <= 0.0 {
        return Ok(RecipeNutrition::default());
    }
    Ok(RecipeNutrition {
        total_grams,
        calories_per_g: has_calories.then_some(total_calories / total_grams),
        protein_per_g: has_protein.then_some(total_protein / total_grams),
        fat_per_g: has_fat.then_some(total_fat / total_grams),
        fiber_per_g: has_fiber.then_some(total_fiber / total_grams),
    })
}

// --- Meal Log ---

/// @brief Records a meal as eaten in the history log.
///
/// Writes a denormalized snapshot into `meal_log` (name and category are
/// copied by value) so history remains intact even if the source ::Recipe
/// is later edited or deleted.
///
/// @param conn Open database connection.
/// @param meal_name Name of the recipe (or ad-hoc meal) eaten.
/// @param category Cooking-method classification at the time it was eaten.
/// @param servings Number of servings eaten.
/// @return `Ok(())` on success.
pub fn log_meal(conn: &Connection, meal_name: &str, category: &RecipeType, servings: f64) -> Result<()> {
    conn.execute(
        "INSERT INTO meal_log (meal_name, category, servings) VALUES (?1, ?2, ?3)",
        params![meal_name, category.as_str(), servings],
    )?;
    Ok(())
}

/// @brief Fetches the full meal history, newest first.
/// @param conn Open database connection.
/// @return All rows of `meal_log`, ordered by `logged_at DESC`.
pub fn get_meal_log(conn: &Connection) -> Result<Vec<MealLogEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, meal_name, category, servings, logged_at FROM meal_log ORDER BY logged_at DESC",
    )?;
    stmt.query_map([], |row| {
        let category_str: String = row.get(2)?;
        Ok(MealLogEntry {
            id: row.get(0)?,
            meal_name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            servings: row.get(3)?,
            logged_at: row.get(4)?,
        })
    })?
    .collect()
}

// --- Search ---

/// @brief Finds ingredients whose name contains the given substring (case-insensitive).
/// @param conn Open database connection.
/// @param query Substring to search for.
/// @return Matching ingredients, alphabetically by name.
pub fn search_ingredients_by_name(conn: &Connection, query: &str) -> Result<Vec<Ingredient>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, stock_amount, stock_unit,
                calories_per_g, protein_per_g, fat_per_g, fiber_per_g, density_g_per_ml, is_favorite
         FROM ingredients
         WHERE name LIKE ?1 ORDER BY name",
    )?;
    stmt.query_map(params![format!("%{query}%")], |row| {
        let category_str: String = row.get(2)?;
        Ok(Ingredient {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(IngredientType::Pending),
            stock_amount: row.get(3)?,
            stock_unit: row.get(4)?,
            calories_per_g: row.get(5)?,
            protein_per_g: row.get(6)?,
            fat_per_g: row.get(7)?,
            fiber_per_g: row.get(8)?,
            density_g_per_ml: row.get(9)?,
            is_favorite: row.get::<_, i64>(10)? != 0,
        })
    })?
    .collect()
}

/// @brief Finds ingredients in the given category.
/// @param conn Open database connection.
/// @param category Category to match exactly.
/// @return Matching ingredients, alphabetically by name.
pub fn search_ingredients_by_category(conn: &Connection, category: &IngredientType) -> Result<Vec<Ingredient>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, stock_amount, stock_unit,
                calories_per_g, protein_per_g, fat_per_g, fiber_per_g, density_g_per_ml, is_favorite
         FROM ingredients
         WHERE category = ?1 ORDER BY name",
    )?;
    stmt.query_map(params![category.as_str()], |row| {
        let category_str: String = row.get(2)?;
        Ok(Ingredient {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(IngredientType::Pending),
            stock_amount: row.get(3)?,
            stock_unit: row.get(4)?,
            calories_per_g: row.get(5)?,
            protein_per_g: row.get(6)?,
            fat_per_g: row.get(7)?,
            fiber_per_g: row.get(8)?,
            density_g_per_ml: row.get(9)?,
            is_favorite: row.get::<_, i64>(10)? != 0,
        })
    })?
    .collect()
}

/// @brief Finds recipes whose name contains the given substring (case-insensitive).
/// @param conn Open database connection.
/// @param query Substring to search for.
/// @return Matching recipes, alphabetically by name.
pub fn search_recipes_by_name(conn: &Connection, query: &str) -> Result<Vec<Recipe>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, prep_min, cook_min, cook_temp, instructions, is_favorite
         FROM recipes WHERE name LIKE ?1 ORDER BY name",
    )?;
    stmt.query_map(params![format!("%{query}%")], |row| {
        let category_str: String = row.get(2)?;
        Ok(Recipe {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            prep_min: row.get(3)?,
            cook_min: row.get(4)?,
            cook_temp: row.get(5)?,
            instructions: row.get(6)?,
            is_favorite: row.get::<_, i64>(7)? != 0,
        })
    })?
    .collect()
}

/// @brief Common `tree(root_id, member_id)` CTE shared by the recursive recipe searches below.
///
/// Every recipe is a possible `root_id`, expanded via `recipe_components` to its full set of
/// (possibly transitive) sub-recipes as `member_id` â€” including itself, since the seed row pairs
/// each recipe with itself. `UNION` (not `UNION ALL`) dedupes `(root_id, member_id)` pairs, which
/// keeps shared sub-recipes cheap and makes the recursion self-terminating even under a cycle.
const RECIPE_TREE_CTE: &str = "WITH RECURSIVE tree(root_id, member_id) AS (
    SELECT id, id FROM recipes
    UNION
    SELECT t.root_id, rc.child_recipe_id
    FROM tree t JOIN recipe_components rc ON rc.parent_recipe_id = t.member_id
)";

/// @brief Finds recipes that have at least one ingredient â€” anywhere in their sub-recipe tree â€”
/// whose name contains the given substring.
/// @param conn Open database connection.
/// @param query Substring to search for in ingredient names.
/// @return Matching recipes (deduplicated), alphabetically by name.
pub fn search_recipes_by_ingredient_name(conn: &Connection, query: &str) -> Result<Vec<Recipe>> {
    let sql = format!(
        "{RECIPE_TREE_CTE}
         SELECT DISTINCT r.id, r.name, r.category, r.prep_min, r.cook_min, r.cook_temp, r.instructions, r.is_favorite
         FROM recipes r
         JOIN tree t ON t.root_id = r.id
         JOIN recipe_ingredients ri ON ri.recipe_id = t.member_id
         JOIN ingredients i ON i.id = ri.ingredient_id
         WHERE i.name LIKE ?1 ORDER BY r.name"
    );
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_map(params![format!("%{query}%")], |row| {
        let category_str: String = row.get(2)?;
        Ok(Recipe {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            prep_min: row.get(3)?,
            cook_min: row.get(4)?,
            cook_temp: row.get(5)?,
            instructions: row.get(6)?,
            is_favorite: row.get::<_, i64>(7)? != 0,
        })
    })?
    .collect()
}

/// @brief Finds recipes that have at least one ingredient â€” anywhere in their sub-recipe tree â€”
/// in the given category.
/// @param conn Open database connection.
/// @param category Ingredient category to match exactly.
/// @return Matching recipes (deduplicated), alphabetically by name.
pub fn search_recipes_by_ingredient_category(conn: &Connection, category: &IngredientType) -> Result<Vec<Recipe>> {
    let sql = format!(
        "{RECIPE_TREE_CTE}
         SELECT DISTINCT r.id, r.name, r.category, r.prep_min, r.cook_min, r.cook_temp, r.instructions, r.is_favorite
         FROM recipes r
         JOIN tree t ON t.root_id = r.id
         JOIN recipe_ingredients ri ON ri.recipe_id = t.member_id
         JOIN ingredients i ON i.id = ri.ingredient_id
         WHERE i.category = ?1 ORDER BY r.name"
    );
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_map(params![category.as_str()], |row| {
        let category_str: String = row.get(2)?;
        Ok(Recipe {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            prep_min: row.get(3)?,
            cook_min: row.get(4)?,
            cook_temp: row.get(5)?,
            instructions: row.get(6)?,
            is_favorite: row.get::<_, i64>(7)? != 0,
        })
    })?
    .collect()
}

/// @brief Finds recipes that have at least one sub-recipe â€” anywhere in their sub-recipe tree â€”
/// whose name contains the given substring.
/// @param conn Open database connection.
/// @param query Substring to search for in sub-recipe names.
/// @return Matching recipes (deduplicated), alphabetically by name. A recipe matching its own
///         name does not count as containing itself â€” only actual sub-recipes do.
pub fn search_recipes_by_subrecipe_name(conn: &Connection, query: &str) -> Result<Vec<Recipe>> {
    let sql = format!(
        "{RECIPE_TREE_CTE}
         SELECT DISTINCT r.id, r.name, r.category, r.prep_min, r.cook_min, r.cook_temp, r.instructions, r.is_favorite
         FROM recipes r
         JOIN tree t ON t.root_id = r.id
         JOIN recipes sub ON sub.id = t.member_id
         WHERE t.member_id != t.root_id AND sub.name LIKE ?1 ORDER BY r.name"
    );
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_map(params![format!("%{query}%")], |row| {
        let category_str: String = row.get(2)?;
        Ok(Recipe {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(RecipeType::Pending),
            prep_min: row.get(3)?,
            cook_min: row.get(4)?,
            cook_temp: row.get(5)?,
            instructions: row.get(6)?,
            is_favorite: row.get::<_, i64>(7)? != 0,
        })
    })?
    .collect()
}

/// @brief Deletes a single meal-log entry.
/// @param conn Open database connection.
/// @param id Primary key of the entry to delete.
/// @return `Ok(())` on success.
pub fn delete_meal_log_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM meal_log WHERE id = ?1", params![id])?;
    Ok(())
}

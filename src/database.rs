//! @file database.rs
//! @brief SQLite data-access layer: schema management and CRUD queries for
//!        workout plans, exercises, exercise history, the exercise library,
//!        and personal metrics.

use rusqlite::{params, Connection, Result};

use crate::models::{
    Exercise, ExerciseLogEntry, ExerciseType, Ingredient, IngredientType, MealLogEntry,
    PersonalMetric, Recipe, RecipeIngredient, RecipeType, WorkoutPlan,
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
/// @param sets Number of sets (strength; pass 0 for cardio).
/// @param reps Number of reps per set (strength; pass 0 for cardio).
/// @param weight Weight in kg (strength, optional).
/// @param duration_min Duration in minutes (cardio, optional).
/// @param distance_km Distance in km (cardio, optional).
/// @return `Ok(())` on success.
pub fn add_exercise(
    conn: &Connection,
    plan_id: i64,
    name: &str,
    exercise_type: &ExerciseType,
    sets: i32,
    reps: i32,
    weight: Option<f64>,
    duration_min: Option<f64>,
    distance_km: Option<f64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO exercises (workout_plan_id, name, exercise_type, sets, reps, weight, duration_min, distance_km)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![plan_id, name, exercise_type.as_str(), sets, reps, weight, duration_min, distance_km],
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
/// @param exercise_type Whether this was a strength or cardio session.
/// @param sets Sets performed (strength).
/// @param reps Reps performed per set (strength).
/// @param weight Weight used in kg (strength, optional).
/// @param duration_min Duration in minutes (cardio, optional).
/// @param distance_km Distance in km (cardio, optional).
/// @return `Ok(())` on success.
pub fn log_exercise_session(
    conn: &Connection,
    name: &str,
    exercise_type: &ExerciseType,
    sets: i32,
    reps: i32,
    weight: Option<f64>,
    duration_min: Option<f64>,
    distance_km: Option<f64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO exercise_log (exercise_name, exercise_type, sets, reps, weight, duration_min, distance_km)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![name, exercise_type.as_str(), sets, reps, weight, duration_min, distance_km],
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
        "SELECT id, name, category, stock_amount, stock_unit FROM ingredients ORDER BY name",
    )?;
    stmt.query_map([], |row| {
        let category_str: String = row.get(2)?;
        Ok(Ingredient {
            id: row.get(0)?,
            name: row.get(1)?,
            category: category_str.parse().unwrap_or(IngredientType::Pending),
            stock_amount: row.get(3)?,
            stock_unit: row.get(4)?,
        })
    })?
    .collect()
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
        "SELECT id, name, category, prep_min, cook_min, cook_temp, instructions
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

/// @brief Fetches all ingredients attached to a recipe, joined with each ingredient's name.
/// @param conn Open database connection.
/// @param recipe_id Primary key of the owning ::Recipe.
/// @return Ingredient links for that recipe, in insertion order.
pub fn get_recipe_ingredients(conn: &Connection, recipe_id: i64) -> Result<Vec<RecipeIngredient>> {
    let mut stmt = conn.prepare(
        "SELECT ri.id, ri.ingredient_id, i.name, ri.amount, ri.unit
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
        })
    })?
    .collect()
}

/// @brief Attaches an existing library ingredient to a recipe.
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
    conn.execute(
        "INSERT INTO recipe_ingredients (recipe_id, ingredient_id, amount, unit) VALUES (?1, ?2, ?3, ?4)",
        params![recipe_id, ingredient_id, amount, unit],
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

/// @brief Deletes a single meal-log entry.
/// @param conn Open database connection.
/// @param id Primary key of the entry to delete.
/// @return `Ok(())` on success.
pub fn delete_meal_log_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM meal_log WHERE id = ?1", params![id])?;
    Ok(())
}

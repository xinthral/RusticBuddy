//! @file models.rs
//! @brief Plain-data domain types shared between the database layer and the UI/app layers.

/// @brief Discriminates between the two supported categories of exercise.
///
/// Strength exercises are tracked via sets/reps/weight; cardio exercises are
/// tracked via duration/distance instead.
#[derive(Debug, Clone, PartialEq)]
pub enum ExerciseType {
    Strength,
    Cardio,
}

impl ExerciseType {
    /// @brief Returns the lowercase string used to persist this variant in SQLite.
    /// @return `"strength"` or `"cardio"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExerciseType::Strength => "strength",
            ExerciseType::Cardio => "cardio",
        }
    }
}

impl std::str::FromStr for ExerciseType {
    type Err = ();

    /// @brief Parses a stored exercise-type string back into an ::ExerciseType.
    /// @param s The raw string read from the database.
    /// @return `Ok(ExerciseType::Cardio)` if `s == "cardio"`, otherwise `Ok(ExerciseType::Strength)`.
    ///         This never fails; unrecognized values fall back to Strength.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cardio" => Ok(ExerciseType::Cardio),
            _ => Ok(ExerciseType::Strength),
        }
    }
}

/// @brief A named collection of exercises, at most one of which is "active" at a time.
#[derive(Debug, Clone)]
pub struct WorkoutPlan {
    /// @brief Primary key in the `workout_plans` table.
    pub id: i64,
    /// @brief Display name of the plan.
    pub name: String,
    /// @brief Whether this is the currently active plan shown on the dashboard.
    pub is_active: bool,
}

/// @brief A single exercise belonging to a ::WorkoutPlan.
///
/// Strength fields (`sets`, `reps`, `weight`) and cardio fields
/// (`duration_min`, `distance_km`) are both present on every row; which ones
/// are meaningful depends on `exercise_type`.
#[derive(Debug, Clone)]
pub struct Exercise {
    /// @brief Primary key in the `exercises` table.
    pub id: i64,
    /// @brief Foreign key to the owning ::WorkoutPlan.
    #[allow(dead_code)]
    pub workout_plan_id: i64,
    /// @brief Exercise name (e.g. "Bench Press").
    pub name: String,
    /// @brief Whether this is a strength or cardio exercise.
    pub exercise_type: ExerciseType,
    /// @brief Number of sets (strength only).
    pub sets: i32,
    /// @brief Number of reps per set (strength only).
    pub reps: i32,
    /// @brief Weight used, in kg (strength only, optional).
    pub weight: Option<f64>,
    /// @brief Duration in minutes (cardio only, optional).
    pub duration_min: Option<f64>,
    /// @brief Distance in kilometers (cardio only, optional).
    pub distance_km: Option<f64>,
}

/// @brief A historical record of one completed exercise session.
///
/// Unlike ::Exercise, this is a denormalized snapshot (exercise name is
/// copied rather than referenced) so history survives edits/deletion of the
/// originating plan exercise.
#[derive(Debug, Clone)]
pub struct ExerciseLogEntry {
    /// @brief Name of the exercise that was performed.
    pub exercise_name: String,
    /// @brief Whether this session was strength or cardio.
    pub exercise_type: ExerciseType,
    /// @brief Sets performed (strength only).
    pub sets: i32,
    /// @brief Reps performed per set (strength only).
    pub reps: i32,
    /// @brief Weight used, in kg (strength only, optional).
    pub weight: Option<f64>,
    /// @brief Duration in minutes (cardio only, optional).
    pub duration_min: Option<f64>,
    /// @brief Distance in kilometers (cardio only, optional).
    pub distance_km: Option<f64>,
    /// @brief Timestamp the session was logged, as stored by SQLite (`datetime('now')`).
    #[allow(dead_code)]
    pub logged_at: String,
}

/// @brief A single point-in-time body-metrics reading.
#[derive(Debug, Clone)]
pub struct PersonalMetric {
    /// @brief Primary key in the `personal_metrics` table.
    #[allow(dead_code)]
    pub id: i64,
    /// @brief Timestamp the reading was recorded, as stored by SQLite.
    pub recorded_at: String,
    /// @brief Body weight in kilograms, if recorded.
    pub weight_kg: Option<f64>,
    /// @brief Resting heart rate in bpm, if recorded.
    pub heart_rate: Option<i32>,
    /// @brief Systolic blood pressure in mmHg, if recorded.
    pub bp_systolic: Option<i32>,
    /// @brief Diastolic blood pressure in mmHg, if recorded.
    pub bp_diastolic: Option<i32>,
}

/// @brief Declarative macro that generates a category enum plus `Display` and case-insensitive
/// `FromStr` (falling back to `Pending`).
///
/// Shared by ::IngredientType and ::RecipeType, which both need a large, open-ended set of
/// string-backed categories.
macro_rules! category_enum {
    ($enum_name:ident { $($variant:ident => $name:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $enum_name {
            $($variant),+
        }

        impl $enum_name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($enum_name::$variant => $name),+
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $(if s.eq_ignore_ascii_case($name) { return Ok($enum_name::$variant); })+
                Ok($enum_name::Pending)
            }
        }
    };
}

category_enum!(IngredientType {
    Condiment => "Condiment",
    Craft     => "Craft",
    Culinary  => "Culinary",
    Dairy     => "Dairy",
    Fat       => "Fat",
    Fish      => "Fish",
    Fruit     => "Fruit",
    Herb      => "Herb",
    Grain     => "Grain",
    Meat      => "Meat",
    Nuts      => "Nuts",
    Seasoning => "Seasoning",
    Sweetener => "Sweetener",
    Vegetable => "Vegetable",
    Pending   => "Pending",
});

category_enum!(RecipeType {
    Baked    => "Baked",
    Boiled   => "Boiled",
    Brewed   => "Brewed",
    Crafted  => "Crafted",
    Fried    => "Fried",
    Grilled  => "Grilled",
    Mixed    => "Mixed",
    Roasted  => "Roasted",
    Sauteed  => "Sauteed",
    Steamed  => "Steamed",
    Simmered => "Simmered",
    Pending  => "Pending",
});

/// @brief A single cookbook ingredient with a name and category.
#[derive(Debug, Clone)]
pub struct Ingredient {
    /// @brief Primary key in the `ingredients` table.
    pub id: i64,
    /// @brief Ingredient name (e.g. "Chicken").
    pub name: String,
    /// @brief Classification (e.g. Meat, Dairy, Pending).
    pub category: IngredientType,
    /// @brief Quantity currently on hand, if the pantry stock is being tracked.
    pub stock_amount: Option<f64>,
    /// @brief Unit of measure for `stock_amount` (e.g. "lb", "cups").
    pub stock_unit: String,
}

/// @brief A cookbook recipe's metadata and instructions.
///
/// The recipe's ingredients live separately in the `recipe_ingredients` join
/// table (see ::RecipeIngredient) rather than embedded here.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// @brief Primary key in the `recipes` table.
    pub id: i64,
    /// @brief Recipe name.
    pub name: String,
    /// @brief Cooking-method classification.
    pub category: RecipeType,
    /// @brief Preparation time in minutes.
    pub prep_min: i32,
    /// @brief Cooking time in minutes.
    pub cook_min: i32,
    /// @brief Cooking temperature in degrees.
    pub cook_temp: i32,
    /// @brief Step-by-step cooking instructions.
    pub instructions: String,
}

/// @brief One ingredient attached to a recipe, joined with the ingredient's name for display.
#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    /// @brief Primary key in the `recipe_ingredients` table.
    pub id: i64,
    /// @brief Foreign key into `ingredients`.
    pub ingredient_id: i64,
    /// @brief Denormalized ingredient name, joined in at query time.
    pub name: String,
    /// @brief Quantity of the ingredient required, if specified.
    pub amount: Option<f64>,
    /// @brief Unit of measure for `amount` (e.g. "cups", "g").
    pub unit: String,
}

/// @brief A historical record of one eaten meal.
///
/// Denormalized like ::ExerciseLogEntry: `meal_name`/`category` are copied by
/// value so history survives edits or deletion of the originating ::Recipe.
#[derive(Debug, Clone)]
pub struct MealLogEntry {
    /// @brief Primary key in the `meal_log` table.
    pub id: i64,
    /// @brief Name of the recipe (or ad-hoc meal) eaten.
    pub meal_name: String,
    /// @brief Cooking-method classification at the time it was logged.
    pub category: RecipeType,
    /// @brief Number of servings eaten.
    pub servings: f64,
    /// @brief Timestamp the meal was logged, as stored by SQLite (`datetime('now')`).
    pub logged_at: String,
}

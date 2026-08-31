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

/// @brief The strength/cardio performance fields shared by ::Exercise and ::ExerciseLogEntry.
///
/// Bundled so `database::add_exercise`/`database::log_exercise_session` don't need one
/// parameter per field; which fields are meaningful depends on `exercise_type`.
#[derive(Debug, Clone)]
pub struct ExercisePerformance {
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
            /// @brief Every variant, in declaration order — used to drive category-picker screens.
            #[allow(dead_code)]
            pub const ALL: &'static [$enum_name] = &[$($enum_name::$variant),+];

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
    /// @brief Whether the user has favorited this ingredient.
    pub is_favorite: bool,
    /// @brief Calories per gram, if entered.
    pub calories_per_g: Option<f64>,
    /// @brief Protein in grams per gram of this ingredient, if entered.
    pub protein_per_g: Option<f64>,
    /// @brief Fat in grams per gram of this ingredient, if entered.
    pub fat_per_g: Option<f64>,
    /// @brief Fiber in grams per gram of this ingredient, if entered.
    pub fiber_per_g: Option<f64>,
    /// @brief Density in grams per milliliter, used to convert volume units (cups/tbsp/tsp/ml/l)
    /// to grams for nutrition math. `None` means volume-unit amounts of this ingredient can't be
    /// converted to grams.
    pub density_g_per_ml: Option<f64>,
}

/// @brief Converts a recipe-ingredient or recipe-component amount/unit into grams, if possible.
///
/// Mass units (`g`/`kg`/`oz`/`lb`) convert via a fixed factor regardless of `density_g_per_ml`.
/// Volume units (`ml`/`l`/`cup`/`tbsp`/`tsp`) convert via milliliters and then require
/// `density_g_per_ml` to reach grams; without it (e.g. a recipe component, which has no density
/// of its own) volume units return `None`. An unrecognized unit always returns `None`.
///
/// @param amount Numeric quantity, in `unit`.
/// @param unit Unit string as entered by the user (case-insensitive, whitespace-trimmed).
/// @param density_g_per_ml Grams per milliliter, needed only to convert volume units.
/// @return The equivalent mass in grams, or `None` if `unit` can't be converted (as-is or for
///         lack of density).
pub fn grams_for(amount: f64, unit: &str, density_g_per_ml: Option<f64>) -> Option<f64> {
    let unit = unit.trim().to_ascii_lowercase();

    let mass_factor = match unit.as_str() {
        "g" | "gram" | "grams" => Some(1.0),
        "kg" | "kilogram" | "kilograms" => Some(1000.0),
        "oz" | "ounce" | "ounces" => Some(28.3495),
        "lb" | "lbs" | "pound" | "pounds" => Some(453.592),
        _ => None,
    };
    if let Some(factor) = mass_factor {
        return Some(amount * factor);
    }

    let volume_ml_factor = match unit.as_str() {
        "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => Some(1.0),
        "l" | "liter" | "liters" | "litre" | "litres" => Some(1000.0),
        "cup" | "cups" => Some(236.588),
        "tbsp" | "tablespoon" | "tablespoons" => Some(14.7868),
        "tsp" | "teaspoon" | "teaspoons" => Some(4.92892),
        _ => None,
    };
    let ml = volume_ml_factor? * amount;
    Some(ml * density_g_per_ml?)
}

/// @brief A recipe's computed nutritional density, rolled up from its ingredients and sub-recipes.
///
/// Produced by `crate::database::compute_recipe_nutrition`. Each macro field is `None` when no
/// contributing ingredient/sub-recipe had that particular value entered, rather than reporting a
/// misleading zero.
#[derive(Debug, Clone, Default)]
pub struct RecipeNutrition {
    /// @brief Total known weight of the recipe, in grams — the sum of every ingredient/sub-recipe
    /// link that had a convertible gram amount. Zero if nothing could be computed.
    pub total_grams: f64,
    /// @brief Calories per gram of the finished recipe, if any contributor had calorie data.
    pub calories_per_g: Option<f64>,
    /// @brief Protein (g) per gram of the finished recipe, if any contributor had protein data.
    pub protein_per_g: Option<f64>,
    /// @brief Fat (g) per gram of the finished recipe, if any contributor had fat data.
    pub fat_per_g: Option<f64>,
    /// @brief Fiber (g) per gram of the finished recipe, if any contributor had fiber data.
    pub fiber_per_g: Option<f64>,
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
    /// @brief Whether the user has favorited this recipe.
    pub is_favorite: bool,
}

/// @brief One recipe attached as a component (sub-recipe) of another recipe, joined with the
/// child recipe's name for display.
#[derive(Debug, Clone)]
pub struct RecipeComponent {
    /// @brief Primary key in the `recipe_components` table.
    pub id: i64,
    /// @brief Foreign key into `recipes` — the sub-recipe used as a component.
    pub child_recipe_id: i64,
    /// @brief Denormalized child recipe name, joined in at query time.
    pub name: String,
    /// @brief Quantity of the sub-recipe required, if specified.
    pub amount: Option<f64>,
    /// @brief Unit of measure for `amount` (e.g. "cups", "batches").
    pub unit: String,
    /// @brief `amount`/`unit` converted to grams at attach time (see `crate::models::grams_for`),
    /// or `None` if `unit` couldn't be converted. Used for nutrition roll-up, not for display.
    pub grams: Option<f64>,
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
    /// @brief `amount`/`unit` converted to grams at attach time (see `crate::models::grams_for`),
    /// or `None` if `unit` couldn't be converted. Used for nutrition roll-up, not for display.
    pub grams: Option<f64>,
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

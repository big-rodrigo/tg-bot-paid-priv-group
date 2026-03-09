use crate::{
    db::{models::{Answer, PaymentGateCondition}, DbPool},
    db_execute, db_query_as,
    error::Result,
};

pub async fn list_conditions(pool: &DbPool, phase_id: i64) -> Result<Vec<PaymentGateCondition>> {
    db_query_as!(pool, PaymentGateCondition,
        "SELECT * FROM payment_gate_conditions WHERE phase_id = ? ORDER BY id ASC",
        [phase_id], fetch_all)
        .map_err(Into::into)
}

pub async fn create_condition(
    pool: &DbPool,
    phase_id: i64,
    question_id: i64,
    condition_type: &str,
    option_id: Option<i64>,
    text_value: Option<&str>,
) -> Result<i64> {
    let (id,): (i64,) = db_query_as!(pool, (i64,),
        "INSERT INTO payment_gate_conditions (phase_id, question_id, condition_type, option_id, text_value) VALUES (?, ?, ?, ?, ?) RETURNING id",
        [phase_id, question_id, condition_type, option_id, text_value], fetch_one)?;
    Ok(id)
}

pub async fn delete_condition(pool: &DbPool, id: i64) -> Result<()> {
    db_execute!(pool, "DELETE FROM payment_gate_conditions WHERE id = ?", [id])?;
    Ok(())
}

/// Evaluate whether a user's answers satisfy all conditions for a payment gate phase.
/// Returns true if all conditions pass (AND logic).
/// A gate with no conditions is unconditional (always returns true).
pub async fn evaluate_gate(pool: &DbPool, phase_id: i64, user_id: i64) -> Result<bool> {
    let conditions = list_conditions(pool, phase_id).await?;

    if conditions.is_empty() {
        return Ok(true);
    }

    for cond in &conditions {
        let answer = db_query_as!(pool, Answer,
            "SELECT * FROM answers WHERE user_id = ? AND question_id = ?",
            [user_id, cond.question_id], fetch_optional)?;

        let passed = match cond.condition_type.as_str() {
            "option_selected" => answer
                .as_ref()
                .and_then(|a| a.option_id)
                .map(|oid| Some(oid) == cond.option_id)
                .unwrap_or(false),
            "option_not_selected" => answer
                .as_ref()
                .and_then(|a| a.option_id)
                .map(|oid| Some(oid) != cond.option_id)
                .unwrap_or(true),
            "text_contains" => {
                let search = cond.text_value.as_deref().unwrap_or("");
                answer
                    .as_ref()
                    .and_then(|a| a.text_value.as_deref())
                    .map(|tv| tv.to_lowercase().contains(&search.to_lowercase()))
                    .unwrap_or(false)
            }
            "text_not_contains" => {
                let search = cond.text_value.as_deref().unwrap_or("");
                answer
                    .as_ref()
                    .and_then(|a| a.text_value.as_deref())
                    .map(|tv| !tv.to_lowercase().contains(&search.to_lowercase()))
                    .unwrap_or(true)
            }
            _ => false,
        };

        if !passed {
            return Ok(false);
        }
    }

    Ok(true)
}

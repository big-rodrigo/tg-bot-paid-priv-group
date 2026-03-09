use crate::{
    db::{
        models::{Answer, InviteRule, InviteRuleCondition},
        DbPool,
    },
    error::Result,
};

pub async fn list_by_phase(pool: &DbPool, phase_id: i64) -> Result<Vec<InviteRule>> {
    sqlx::query_as::<_, InviteRule>(
        "SELECT * FROM invite_rules WHERE phase_id = ? ORDER BY position ASC",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<InviteRule>> {
    sqlx::query_as::<_, InviteRule>("SELECT * FROM invite_rules WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn create(pool: &DbPool, phase_id: i64, group_id: i64, position: i64) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO invite_rules (phase_id, group_id, position) VALUES (?, ?, ?)",
    )
    .bind(phase_id)
    .bind(group_id)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn update(pool: &DbPool, id: i64, group_id: i64, position: i64) -> Result<()> {
    sqlx::query("UPDATE invite_rules SET group_id = ?, position = ? WHERE id = ?")
        .bind(group_id)
        .bind(position)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM invite_rules WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder(pool: &DbPool, items: &[(i64, i64)]) -> Result<()> {
    for (id, position) in items {
        sqlx::query("UPDATE invite_rules SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn list_conditions(
    pool: &DbPool,
    invite_rule_id: i64,
) -> Result<Vec<InviteRuleCondition>> {
    sqlx::query_as::<_, InviteRuleCondition>(
        "SELECT * FROM invite_rule_conditions WHERE invite_rule_id = ? ORDER BY id ASC",
    )
    .bind(invite_rule_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn create_condition(
    pool: &DbPool,
    invite_rule_id: i64,
    question_id: i64,
    condition_type: &str,
    option_id: Option<i64>,
    text_value: Option<&str>,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO invite_rule_conditions (invite_rule_id, question_id, condition_type, option_id, text_value)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(invite_rule_id)
    .bind(question_id)
    .bind(condition_type)
    .bind(option_id)
    .bind(text_value)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn delete_condition(pool: &DbPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM invite_rule_conditions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Evaluate whether a user's answers satisfy all conditions for an invite rule.
/// Returns true if all conditions pass (AND logic).
/// A rule with no conditions is unconditional (always returns true).
pub async fn evaluate_rule(pool: &DbPool, invite_rule_id: i64, user_id: i64) -> Result<bool> {
    let conditions = list_conditions(pool, invite_rule_id).await?;

    if conditions.is_empty() {
        return Ok(true);
    }

    for cond in &conditions {
        let answer = sqlx::query_as::<_, Answer>(
            "SELECT * FROM answers WHERE user_id = ? AND question_id = ?",
        )
        .bind(user_id)
        .bind(cond.question_id)
        .fetch_optional(pool)
        .await?;

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

use sat_cfdi::RelatedCfdis;
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct RelatedDocument {
    pub id: i32,
    pub relation_type: String,
    pub related_uuid: String,
    pub related_invoice_id: Option<i32>,
}

pub async fn replace_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    user_id: i32,
    related_cfdis: &[RelatedCfdis],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM invoice_related_documents WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&mut *tx)
        .await?;

    for group in related_cfdis {
        let relation_type = group.relation_type.to_string();
        for item in &group.items {
            let related_invoice_id: Option<i32> = sqlx::query_scalar(
                "SELECT id FROM invoices WHERE uuid = $1::UUID AND user_id = $2 LIMIT 1",
            )
            .bind(&item.uuid)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO invoice_related_documents
                    (invoice_id, relation_type, related_uuid, related_invoice_id)
                 VALUES ($1, $2, $3::UUID, $4)",
            )
            .bind(invoice_id)
            .bind(&relation_type)
            .bind(&item.uuid)
            .bind(related_invoice_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    user_id: i32,
) -> Result<Vec<RelatedDocument>, sqlx::Error> {
    sqlx::query_as::<_, RelatedDocument>(
        "SELECT ird.id, ird.relation_type, ird.related_uuid::TEXT, ird.related_invoice_id
         FROM invoice_related_documents ird
         JOIN invoices inv ON inv.id = ird.invoice_id
         WHERE ird.invoice_id = $1 AND inv.user_id = $2
         ORDER BY ird.id",
    )
    .bind(invoice_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

use sat_cfdi::LineItem;
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct InvoiceItem {
    pub id: i32,
    pub invoice_id: i32,
    pub product_service_key: String,
    pub id_number: Option<String>,
    pub quantity: f64,
    pub unit_key: String,
    pub unit: Option<String>,
    pub description: String,
    pub unit_value: f64,
    pub amount: f64,
    pub discount: Option<f64>,
    pub tax_object: Option<String>,
    pub third_party: Option<serde_json::Value>,
    pub customs_info: serde_json::Value,
    pub property_tax_accounts: serde_json::Value,
    pub parts: serde_json::Value,
}

#[derive(FromRow)]
pub struct InvoiceItemTax {
    pub id: i32,
    pub item_id: i32,
    pub tax_type: String,
    pub tax: String,
    pub factor_type: Option<String>,
    pub base: Option<f64>,
    pub rate_or_amount: Option<f64>,
    pub amount: Option<f64>,
}

pub async fn replace_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    items: &[LineItem],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM invoice_items WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&mut *tx)
        .await?;

    for item in items {
        let item_id: i32 = sqlx::query_scalar(
            "INSERT INTO invoice_items
                (invoice_id, product_service_key, id_number, quantity, unit_key, unit,
                 description, unit_value, amount, discount, tax_object,
                 third_party, customs_info, property_tax_accounts, parts)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING id",
        )
        .bind(invoice_id)
        .bind(&item.product_service_key)
        .bind(&item.id_number)
        .bind(item.quantity.parse::<f64>().unwrap_or(0.0))
        .bind(&item.unit_key)
        .bind(&item.unit)
        .bind(&item.description)
        .bind(item.unit_value.parse::<f64>().unwrap_or(0.0))
        .bind(item.amount.parse::<f64>().unwrap_or(0.0))
        .bind(item.discount.as_deref().and_then(|d| d.parse::<f64>().ok()))
        .bind(item.tax_object.as_ref().map(|t| t.to_string()))
        .bind(
            item.third_party
                .as_ref()
                .map(|t| serde_json::to_value(t).ok())
                .flatten(),
        )
        .bind(serde_json::to_value(&item.customs_info).unwrap_or(serde_json::Value::Array(vec![])))
        .bind(
            serde_json::to_value(&item.property_tax_accounts)
                .unwrap_or(serde_json::Value::Array(vec![])),
        )
        .bind(serde_json::to_value(&item.parts).unwrap_or(serde_json::Value::Array(vec![])))
        .fetch_one(&mut *tx)
        .await?;

        if let Some(taxes) = &item.taxes {
            for transfer in taxes.transfers() {
                sqlx::query(
                    "INSERT INTO invoice_item_taxes
                        (item_id, tax_type, tax, factor_type, base, rate_or_amount, amount)
                     VALUES ($1, 'transfer', $2, $3, $4, $5, $6)",
                )
                .bind(item_id)
                .bind(transfer.tax.to_string())
                .bind(transfer.factor_type.to_string())
                .bind(transfer.base.as_deref().and_then(|v| v.parse::<f64>().ok()))
                .bind(
                    transfer
                        .rate_or_amount
                        .as_deref()
                        .and_then(|v| v.parse::<f64>().ok()),
                )
                .bind(
                    transfer
                        .amount
                        .as_deref()
                        .and_then(|v| v.parse::<f64>().ok()),
                )
                .execute(&mut *tx)
                .await?;
            }

            for withholding in taxes.withholdings() {
                sqlx::query(
                    "INSERT INTO invoice_item_taxes
                        (item_id, tax_type, tax, factor_type, base, rate_or_amount, amount)
                     VALUES ($1, 'withholding', $2, $3, $4, $5, $6)",
                )
                .bind(item_id)
                .bind(withholding.tax.to_string())
                .bind(withholding.factor_type.as_ref().map(|f| f.to_string()))
                .bind(
                    withholding
                        .base
                        .as_deref()
                        .and_then(|v| v.parse::<f64>().ok()),
                )
                .bind(
                    withholding
                        .rate_or_amount
                        .as_deref()
                        .and_then(|v| v.parse::<f64>().ok()),
                )
                .bind(withholding.amount.parse::<f64>().ok())
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    user_id: i32,
) -> Result<Vec<(InvoiceItem, Vec<InvoiceItemTax>)>, sqlx::Error> {
    let items = sqlx::query_as::<_, InvoiceItem>(
        "SELECT ii.id, ii.invoice_id, ii.product_service_key, ii.id_number,
                ii.quantity::FLOAT8, ii.unit_key, ii.unit, ii.description,
                ii.unit_value::FLOAT8, ii.amount::FLOAT8, ii.discount::FLOAT8,
                ii.tax_object, ii.third_party, ii.customs_info,
                ii.property_tax_accounts, ii.parts
         FROM invoice_items ii
         JOIN invoices inv ON inv.id = ii.invoice_id
         WHERE ii.invoice_id = $1 AND inv.user_id = $2
         ORDER BY ii.id",
    )
    .bind(invoice_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if items.is_empty() {
        return Ok(vec![]);
    }

    let item_ids: Vec<i32> = items.iter().map(|i| i.id).collect();

    let taxes = sqlx::query_as::<_, InvoiceItemTax>(
        "SELECT id, item_id, tax_type, tax, factor_type,
                base::FLOAT8, rate_or_amount::FLOAT8, amount::FLOAT8
         FROM invoice_item_taxes
         WHERE item_id = ANY($1)
         ORDER BY item_id, id",
    )
    .bind(&item_ids)
    .fetch_all(pool)
    .await?;

    let result = items
        .into_iter()
        .map(|item| {
            let item_taxes: Vec<InvoiceItemTax> = taxes
                .iter()
                .filter(|t| t.item_id == item.id)
                .map(|t| InvoiceItemTax {
                    id: t.id,
                    item_id: t.item_id,
                    tax_type: t.tax_type.clone(),
                    tax: t.tax.clone(),
                    factor_type: t.factor_type.clone(),
                    base: t.base,
                    rate_or_amount: t.rate_or_amount,
                    amount: t.amount,
                })
                .collect();
            (item, item_taxes)
        })
        .collect();

    Ok(result)
}

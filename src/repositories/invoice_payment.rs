use chrono::{DateTime, TimeZone, Utc};
use sat_cfdi::PaymentsComplement;
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct PaymentComplement {
    pub id: i32,
    pub invoice_id: i32,
    pub version: String,
    pub total_payments_amount: Option<f64>,
    pub total_iva_withheld: Option<f64>,
    pub total_isr_withheld: Option<f64>,
    pub total_ieps_withheld: Option<f64>,
    pub total_transferred_iva_base_16: Option<f64>,
    pub total_transferred_iva_tax_16: Option<f64>,
    pub total_transferred_iva_base_8: Option<f64>,
    pub total_transferred_iva_tax_8: Option<f64>,
    pub total_transferred_iva_base_0: Option<f64>,
    pub total_transferred_iva_tax_0: Option<f64>,
    pub total_transferred_iva_base_exempt: Option<f64>,
}

#[derive(FromRow)]
pub struct InvoicePayment {
    pub id: i32,
    pub complement_id: i32,
    pub invoice_id: i32,
    pub payment_date: DateTime<Utc>,
    pub payment_form: String,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub amount: f64,
    pub operation_number: Option<String>,
    pub ordering_account_issuer_tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub ordering_account: Option<String>,
    pub beneficiary_account_issuer_tax_id: Option<String>,
    pub beneficiary_account: Option<String>,
    pub total_transferred_tax: f64,
    pub total_withheld_tax: f64,
}

#[derive(FromRow)]
pub struct PaymentRelatedDocument {
    pub id: i32,
    pub payment_id: i32,
    pub document_id: String,
    pub related_invoice_id: Option<i32>,
    pub series: Option<String>,
    pub fiscal_id: Option<String>,
    pub document_currency: String,
    pub exchange_equivalence: Option<f64>,
    pub installment_number: i32,
    pub previous_balance: f64,
    pub paid_amount: f64,
    pub outstanding_balance: f64,
    pub tax_object: String,
    pub total_transferred_tax: f64,
    pub total_withheld_tax: f64,
}

#[derive(FromRow)]
pub struct PaymentDocumentTax {
    pub id: i32,
    pub related_document_id: i32,
    pub tax_type: String,
    pub tax: String,
    pub factor_type: Option<String>,
    pub base: Option<f64>,
    pub rate_or_amount: Option<f64>,
    pub amount: Option<f64>,
}

pub struct PaymentFilters {
    pub invoice_id: Option<i32>,
    pub payment_form: Option<String>,
    pub currency: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub amount_min: Option<f64>,
    pub amount_max: Option<f64>,
}

const PAYMENT_COLUMNS: &str = "
    p.id, p.complement_id, p.invoice_id, p.payment_date, p.payment_form,
    p.currency, p.exchange_rate::FLOAT8, p.amount::FLOAT8, p.operation_number,
    p.ordering_account_issuer_tax_id, p.bank_name, p.ordering_account,
    p.beneficiary_account_issuer_tax_id, p.beneficiary_account,
    p.total_transferred_tax::FLOAT8, p.total_withheld_tax::FLOAT8";

pub async fn replace_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    user_id: i32,
    complement: &PaymentsComplement,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM invoice_payment_complements WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&mut *tx)
        .await?;

    let totals = &complement.totals;
    let complement_id: i32 = sqlx::query_scalar(
        "INSERT INTO invoice_payment_complements
            (invoice_id, version,
             total_payments_amount, total_iva_withheld, total_isr_withheld, total_ieps_withheld,
             total_transferred_iva_base_16, total_transferred_iva_tax_16,
             total_transferred_iva_base_8,  total_transferred_iva_tax_8,
             total_transferred_iva_base_0,  total_transferred_iva_tax_0,
             total_transferred_iva_base_exempt)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id",
    )
    .bind(invoice_id)
    .bind(&complement.version)
    .bind(totals.total_payments_amount.parse::<f64>().ok())
    .bind(totals.total_iva_withheld.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_isr_withheld.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_ieps_withheld.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_base_16.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_tax_16.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_base_8.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_tax_8.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_base_0.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_tax_0.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .bind(totals.total_transferred_iva_base_exempt.as_deref().and_then(|v| v.parse::<f64>().ok()))
    .fetch_one(&mut *tx)
    .await?;

    for payment in &complement.payments {
        let payment_date = sat_cfdi::parse_cfdi_datetime(&payment.payment_date)
            .map(|ndt| Utc.from_utc_datetime(&ndt))
            .unwrap_or_else(|_| Utc::now());

        let total_transferred_tax: f64 = payment
            .taxes
            .as_ref()
            .map(|t| {
                t.transfers()
                    .iter()
                    .filter_map(|tr| tr.amount.as_deref().and_then(|a| a.parse::<f64>().ok()))
                    .sum()
            })
            .unwrap_or(0.0);

        let total_withheld_tax: f64 = payment
            .taxes
            .as_ref()
            .map(|t| {
                t.withholdings()
                    .iter()
                    .filter_map(|w| w.amount.parse::<f64>().ok())
                    .sum()
            })
            .unwrap_or(0.0);

        let payment_id: i32 = sqlx::query_scalar(
            "INSERT INTO invoice_payments
                (complement_id, invoice_id, payment_date, payment_form, currency, exchange_rate,
                 amount, operation_number, ordering_account_issuer_tax_id, bank_name,
                 ordering_account, beneficiary_account_issuer_tax_id, beneficiary_account,
                 total_transferred_tax, total_withheld_tax)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING id",
        )
        .bind(complement_id)
        .bind(invoice_id)
        .bind(payment_date)
        .bind(payment.payment_form.to_string())
        .bind(payment.currency.to_string())
        .bind(payment.exchange_rate.as_deref().and_then(|r| r.parse::<f64>().ok()))
        .bind(payment.amount.parse::<f64>().unwrap_or(0.0))
        .bind(&payment.operation_number)
        .bind(&payment.ordering_account_issuer_tax_id)
        .bind(&payment.bank_name)
        .bind(&payment.ordering_account)
        .bind(&payment.beneficiary_account_issuer_tax_id)
        .bind(&payment.beneficiary_account)
        .bind(total_transferred_tax)
        .bind(total_withheld_tax)
        .fetch_one(&mut *tx)
        .await?;

        for doc in &payment.related_documents {
            let related_invoice_id: Option<i32> = sqlx::query_scalar(
                "SELECT id FROM invoices WHERE uuid = $1::uuid AND user_id = $2 LIMIT 1",
            )
            .bind(&doc.document_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

            let total_transferred_tax_doc: f64 = doc
                .taxes
                .as_ref()
                .map(|t| {
                    t.transfers()
                        .iter()
                        .filter_map(|tr| tr.amount.as_deref().and_then(|a| a.parse::<f64>().ok()))
                        .sum()
                })
                .unwrap_or(0.0);

            let total_withheld_tax_doc: f64 = doc
                .taxes
                .as_ref()
                .map(|t| {
                    t.withholdings()
                        .iter()
                        .filter_map(|w| w.amount.parse::<f64>().ok())
                        .sum()
                })
                .unwrap_or(0.0);

            let doc_id: i32 = sqlx::query_scalar(
                "INSERT INTO invoice_payment_related_documents
                    (payment_id, document_id, related_invoice_id, series, fiscal_id,
                     document_currency, exchange_equivalence, installment_number,
                     previous_balance, paid_amount, outstanding_balance, tax_object,
                     total_transferred_tax, total_withheld_tax)
                 VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id",
            )
            .bind(payment_id)
            .bind(&doc.document_id)
            .bind(related_invoice_id)
            .bind(&doc.series)
            .bind(&doc.fiscal_id)
            .bind(doc.document_currency.to_string())
            .bind(doc.exchange_equivalence.as_deref().and_then(|e| e.parse::<f64>().ok()))
            .bind(doc.installment_number.parse::<i32>().unwrap_or(1))
            .bind(doc.previous_balance.parse::<f64>().unwrap_or(0.0))
            .bind(doc.paid_amount.parse::<f64>().unwrap_or(0.0))
            .bind(doc.outstanding_balance.parse::<f64>().unwrap_or(0.0))
            .bind(doc.tax_object.to_string())
            .bind(total_transferred_tax_doc)
            .bind(total_withheld_tax_doc)
            .fetch_one(&mut *tx)
            .await?;

            if let Some(taxes) = &doc.taxes {
                for tr in taxes.transfers() {
                    sqlx::query(
                        "INSERT INTO invoice_payment_document_taxes
                            (related_document_id, tax_type, tax, factor_type, base, rate_or_amount, amount)
                         VALUES ($1, 'transfer', $2, $3, $4, $5, $6)",
                    )
                    .bind(doc_id)
                    .bind(tr.tax.to_string())
                    .bind(tr.factor_type.to_string())
                    .bind(tr.base.parse::<f64>().ok())
                    .bind(tr.rate_or_amount.as_deref().and_then(|v| v.parse::<f64>().ok()))
                    .bind(tr.amount.as_deref().and_then(|v| v.parse::<f64>().ok()))
                    .execute(&mut *tx)
                    .await?;
                }

                for w in taxes.withholdings() {
                    sqlx::query(
                        "INSERT INTO invoice_payment_document_taxes
                            (related_document_id, tax_type, tax, factor_type, base, rate_or_amount, amount)
                         VALUES ($1, 'withholding', $2, $3, $4, $5, $6)",
                    )
                    .bind(doc_id)
                    .bind(w.tax.to_string())
                    .bind(w.factor_type.to_string())
                    .bind(w.base.parse::<f64>().ok())
                    .bind(w.rate_or_amount.parse::<f64>().ok())
                    .bind(w.amount.parse::<f64>().ok())
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn find_for_invoice(
    pool: &PgPool,
    invoice_id: i32,
    user_id: i32,
) -> Result<
    Option<(
        PaymentComplement,
        Vec<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>,
    )>,
    sqlx::Error,
> {
    let complement = sqlx::query_as::<_, PaymentComplement>(
        "SELECT c.id, c.invoice_id, c.version,
                c.total_payments_amount::FLOAT8,
                c.total_iva_withheld::FLOAT8, c.total_isr_withheld::FLOAT8,
                c.total_ieps_withheld::FLOAT8,
                c.total_transferred_iva_base_16::FLOAT8, c.total_transferred_iva_tax_16::FLOAT8,
                c.total_transferred_iva_base_8::FLOAT8,  c.total_transferred_iva_tax_8::FLOAT8,
                c.total_transferred_iva_base_0::FLOAT8,  c.total_transferred_iva_tax_0::FLOAT8,
                c.total_transferred_iva_base_exempt::FLOAT8
         FROM invoice_payment_complements c
         JOIN invoices inv ON inv.id = c.invoice_id
         WHERE c.invoice_id = $1 AND inv.user_id = $2",
    )
    .bind(invoice_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let Some(complement) = complement else {
        return Ok(None);
    };

    let payments = fetch_payments_for_complement(pool, complement.id).await?;
    Ok(Some((complement, payments)))
}

async fn fetch_payments_for_complement(
    pool: &PgPool,
    complement_id: i32,
) -> Result<Vec<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>, sqlx::Error>
{
    let payments = sqlx::query_as::<_, InvoicePayment>(&format!(
        "SELECT {PAYMENT_COLUMNS} FROM invoice_payments p WHERE p.complement_id = $1 ORDER BY p.id"
    ))
    .bind(complement_id)
    .fetch_all(pool)
    .await?;

    if payments.is_empty() {
        return Ok(vec![]);
    }

    let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
    let (docs, taxes) = fetch_docs_and_taxes(pool, &payment_ids).await?;

    Ok(payments
        .into_iter()
        .map(|p| {
            let p_docs: Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)> = docs
                .iter()
                .filter(|d| d.payment_id == p.id)
                .map(|d| {
                    let d_taxes: Vec<PaymentDocumentTax> = taxes
                        .iter()
                        .filter(|t| t.related_document_id == d.id)
                        .map(|t| PaymentDocumentTax {
                            id: t.id,
                            related_document_id: t.related_document_id,
                            tax_type: t.tax_type.clone(),
                            tax: t.tax.clone(),
                            factor_type: t.factor_type.clone(),
                            base: t.base,
                            rate_or_amount: t.rate_or_amount,
                            amount: t.amount,
                        })
                        .collect();
                    (
                        PaymentRelatedDocument {
                            id: d.id,
                            payment_id: d.payment_id,
                            document_id: d.document_id.clone(),
                            related_invoice_id: d.related_invoice_id,
                            series: d.series.clone(),
                            fiscal_id: d.fiscal_id.clone(),
                            document_currency: d.document_currency.clone(),
                            exchange_equivalence: d.exchange_equivalence,
                            installment_number: d.installment_number,
                            previous_balance: d.previous_balance,
                            paid_amount: d.paid_amount,
                            outstanding_balance: d.outstanding_balance,
                            tax_object: d.tax_object.clone(),
                            total_transferred_tax: d.total_transferred_tax,
                            total_withheld_tax: d.total_withheld_tax,
                        },
                        d_taxes,
                    )
                })
                .collect();
            (p, p_docs)
        })
        .collect())
}

async fn fetch_docs_and_taxes(
    pool: &PgPool,
    payment_ids: &[i32],
) -> Result<(Vec<PaymentRelatedDocument>, Vec<PaymentDocumentTax>), sqlx::Error> {
    let docs = sqlx::query_as::<_, PaymentRelatedDocument>(
        "SELECT d.id, d.payment_id, d.document_id::TEXT, d.related_invoice_id,
                d.series, d.fiscal_id, d.document_currency,
                d.exchange_equivalence::FLOAT8, d.installment_number,
                d.previous_balance::FLOAT8, d.paid_amount::FLOAT8, d.outstanding_balance::FLOAT8,
                d.tax_object, d.total_transferred_tax::FLOAT8, d.total_withheld_tax::FLOAT8
         FROM invoice_payment_related_documents d
         WHERE d.payment_id = ANY($1)
         ORDER BY d.payment_id, d.id",
    )
    .bind(payment_ids)
    .fetch_all(pool)
    .await?;

    if docs.is_empty() {
        return Ok((vec![], vec![]));
    }

    let doc_ids: Vec<i32> = docs.iter().map(|d| d.id).collect();
    let taxes = sqlx::query_as::<_, PaymentDocumentTax>(
        "SELECT id, related_document_id, tax_type, tax, factor_type,
                base::FLOAT8, rate_or_amount::FLOAT8, amount::FLOAT8
         FROM invoice_payment_document_taxes
         WHERE related_document_id = ANY($1)
         ORDER BY related_document_id, id",
    )
    .bind(&doc_ids)
    .fetch_all(pool)
    .await?;

    Ok((docs, taxes))
}

pub async fn find_payment_for_user(
    pool: &PgPool,
    payment_id: i32,
    user_id: i32,
) -> Result<
    Option<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>,
    sqlx::Error,
> {
    let payment = sqlx::query_as::<_, InvoicePayment>(&format!(
        "SELECT {PAYMENT_COLUMNS}
         FROM invoice_payments p
         JOIN invoice_payment_complements c ON c.id = p.complement_id
         JOIN invoices inv ON inv.id = c.invoice_id
         WHERE p.id = $1 AND inv.user_id = $2"
    ))
    .bind(payment_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let Some(payment) = payment else {
        return Ok(None);
    };

    let (docs, taxes) = fetch_docs_and_taxes(pool, &[payment.id]).await?;
    let related_documents = docs
        .into_iter()
        .map(|d| {
            let d_taxes: Vec<PaymentDocumentTax> = taxes
                .iter()
                .filter(|t| t.related_document_id == d.id)
                .map(|t| PaymentDocumentTax {
                    id: t.id,
                    related_document_id: t.related_document_id,
                    tax_type: t.tax_type.clone(),
                    tax: t.tax.clone(),
                    factor_type: t.factor_type.clone(),
                    base: t.base,
                    rate_or_amount: t.rate_or_amount,
                    amount: t.amount,
                })
                .collect();
            (d, d_taxes)
        })
        .collect();

    Ok(Some((payment, related_documents)))
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: i32,
    filters: PaymentFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<InvoicePayment>, i64), sqlx::Error> {
    let where_clause = "FROM invoice_payments p
         JOIN invoice_payment_complements c ON c.id = p.complement_id
         JOIN invoices inv ON inv.id = c.invoice_id
         WHERE inv.user_id = $1
           AND ($2::INT IS NULL OR p.invoice_id = $2)
           AND ($3::TEXT IS NULL OR p.payment_form = $3)
           AND ($4::TEXT IS NULL OR p.currency = $4)
           AND ($5::TIMESTAMPTZ IS NULL OR p.payment_date >= $5)
           AND ($6::TIMESTAMPTZ IS NULL OR p.payment_date <= $6)
           AND ($7::FLOAT8 IS NULL OR p.amount >= $7)
           AND ($8::FLOAT8 IS NULL OR p.amount <= $8)";

    macro_rules! bind_filters {
        ($q:expr) => {
            $q.bind(user_id)
                .bind(filters.invoice_id)
                .bind(&filters.payment_form)
                .bind(&filters.currency)
                .bind(filters.date_from)
                .bind(filters.date_to)
                .bind(filters.amount_min)
                .bind(filters.amount_max)
        };
    }

    let total: i64 = bind_filters!(sqlx::query_scalar(&format!("SELECT COUNT(*) {where_clause}")))
        .fetch_one(pool)
        .await?;

    let rows = bind_filters!(sqlx::query_as::<_, InvoicePayment>(&format!(
        "SELECT {PAYMENT_COLUMNS} {where_clause} ORDER BY p.payment_date DESC, p.id LIMIT $9 OFFSET $10"
    )))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((rows, total))
}

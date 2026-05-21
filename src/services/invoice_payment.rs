use sqlx::PgPool;

use crate::error::ApiError;
use crate::repositories::invoice_payment::{
    self, InvoicePayment, PaymentDocumentTax, PaymentFilters, PaymentRelatedDocument,
};

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: PaymentFilters,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<InvoicePayment>, i64), ApiError> {
    invoice_payment::list_for_user(pool, user_id, filters, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("failed to list payments: {e}");
            ApiError::Internal
        })
}

pub async fn get(
    pool: &PgPool,
    user_id: i32,
    payment_id: i32,
) -> Result<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>), ApiError> {
    invoice_payment::find_payment_for_user(pool, payment_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch payment {payment_id}: {e}");
            ApiError::Internal
        })?
        .ok_or(ApiError::NotFound("not found"))
}

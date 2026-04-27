pub mod auth;
pub mod crawl;
pub mod credential;
pub mod invoice;
pub mod link;
pub mod taxpayer;

pub fn paginate(page: i64, per_page: i64) -> (i64, i64, i64) {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    (page, per_page, offset)
}

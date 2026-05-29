pub mod manifest;
pub mod fetcher;
pub mod listing;

pub use fetcher::Fetcher;
pub use listing::{fetch_listing_dates, diff_listing_dates};

pub const BDPM_URL: &str =
    "https://base-donnees-publique.medicaments.gouv.fr";
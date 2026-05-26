pub mod manifest;
pub mod state;
pub mod fetcher;
pub mod listing;

pub use manifest::BDPMFile;
pub use fetcher::Fetcher;
pub use listing::{fetch_listing_dates, diff_listing_dates, ListingDates};

pub const BDPM_URL: &str =
    "https://base-donnees-publique.medicaments.gouv.fr";
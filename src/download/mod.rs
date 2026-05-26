pub mod manifest;
pub mod state;
pub mod fetcher;

pub use manifest::BDPMFile;
pub use state::StateStore;
pub use fetcher::Fetcher;

pub const BDPM_BASE_URL: &str = "https://base-donnees-publique.medicaments.gouv.fr/telechargement";
pub const BDPM_URL: &str = "https://base-donnees-publique.medicaments.gouv.fr/telechargement/download/file";
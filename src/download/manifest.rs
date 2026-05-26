//! BDPM file manifest — encoding, field count, date formats, download paths.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    Windows1252,
    Latin1,
    Utf8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DateFormat {
    /// DD/MM/YYYY — e.g., "28/04/2026"
    DDMMYYYY,
    /// YYYYMMDD integer — e.g., 20260422
    YYYYMMDD,
    /// ISO-8601 — e.g., "2026-04-22"
    ISO8601,
}

pub struct FileSchema {
    /// Display name
    pub name: &'static str,
    /// Remote filename on BDPM server
    pub filename: &'static str,
    /// Download URL path
    pub download_path: &'static str,
    /// Character encoding of the file
    pub encoding: Encoding,
    /// Number of tab-delimited fields
    pub field_count: usize,
    /// (0-indexed field position, date format) pairs
    pub date_fields: &'static [(usize, DateFormat)],
    /// CIS_CIP_bdpm.txt has a trailing tab on every line creating a phantom 14th field
    pub has_trailing_tab_fix: bool,
    /// Target SQLite table name
    pub target_table: &'static str,
}

/// All 10 stable BDPM files (CIS_InfoImportantes excluded — safety-critical, Phase 3.5)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BDPMFile {
    /// Core drug records — CIS, name, form, route, status, dates, lab
    CIS_bdpm,
    /// Presentations — CIS, CIP13, labels, price, reimbursement, EAN
    CIS_CIP_bdpm,
    /// Compositions — CIS, substance code/name, dosage, pharm code
    CIS_COMPO_bdpm,
    /// HAS SMR ratings — decision dates YYYYMMDD, levels, avis text
    CIS_HAS_SMR_bdpm,
    /// HAS ASMR ratings — decision dates YYYYMMDD, levels I–V, avis text
    CIS_HAS_ASMR_bdpm,
    /// Generic group membership — type field: 0/1/2/4
    CIS_GENER_bdpm,
    /// Prescription rules — CPD text per CIS
    CIS_CPD_bdpm,
    /// Stock availability — weekly cadence, fields 2+7 intentionally empty
    CIS_CIP_Dispo_Spec,
    /// ATC classification — CIS ↔ ATC mapping
    CIS_MITM,
    /// HAS transparency committee links — UTF-8, pure ASCII
    HAS_LiensPageCT_bdpm,
}

impl BDPMFile {
    /// Remote filename on the BDPM server.
    pub fn filename(&self) -> &'static str {
        self.schema().filename
    }

    /// Full schema metadata for this file.
    pub fn schema(&self) -> &'static FileSchema {
        match self {
            BDPMFile::CIS_bdpm => &CIS_BDPM,
            BDPMFile::CIS_CIP_bdpm => &CIS_CIP_BDPM,
            BDPMFile::CIS_COMPO_bdpm => &CIS_COMPO_BDPM,
            BDPMFile::CIS_HAS_SMR_bdpm => &CIS_HAS_SMR_BDPM,
            BDPMFile::CIS_HAS_ASMR_bdpm => &CIS_HAS_ASMR_BDPM,
            BDPMFile::CIS_GENER_bdpm => &CIS_GENER_BDPM,
            BDPMFile::CIS_CPD_bdpm => &CIS_CPD_BDPM,
            BDPMFile::CIS_CIP_Dispo_Spec => &CIS_CIP_DISPO_SPEC,
            BDPMFile::CIS_MITM => &CIS_MITM,
            BDPMFile::HAS_LiensPageCT_bdpm => &HAS_LIENS_PAGE_CT_BDPM,
        }
    }

    /// Download URL path (appended to BDPM_URL).
    pub fn download_path(&self) -> &'static str {
        self.schema().download_path
    }

    /// Target SQLite table for this file's data.
    pub fn target_table(&self) -> &'static str {
        self.schema().target_table
    }

    /// All 10 stable files in the order used by the monthly sync.
    pub fn all() -> Vec<BDPMFile> {
        vec![
            BDPMFile::CIS_bdpm,
            BDPMFile::CIS_CIP_bdpm,
            BDPMFile::CIS_COMPO_bdpm,
            BDPMFile::CIS_HAS_SMR_bdpm,
            BDPMFile::CIS_HAS_ASMR_bdpm,
            BDPMFile::CIS_GENER_bdpm,
            BDPMFile::CIS_CPD_bdpm,
            BDPMFile::CIS_CIP_Dispo_Spec,
            BDPMFile::CIS_MITM,
            BDPMFile::HAS_LiensPageCT_bdpm,
        ]
    }

    /// Files in the weekly cadence (CIS_CIP_Dispo_Spec updates weekly, independently).
    pub fn weekly() -> Vec<BDPMFile> {
        vec![BDPMFile::CIS_CIP_Dispo_Spec]
    }

    /// Files in the monthly cadence (all stable BDPM files).
    pub fn monthly() -> Vec<BDPMFile> {
        Self::all()
    }
}

// ─── File schemas ────────────────────────────────────────────────────────────

const CIS_BDPM: FileSchema = FileSchema {
    name: "CIS_bdpm",
    filename: "CIS_bdpm.txt",
    download_path: "/download/file/CIS_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 12,
    date_fields: &[(9, DateFormat::DDMMYYYY)],
    has_trailing_tab_fix: false,
    target_table: "drugs",
};

const CIS_CIP_BDPM: FileSchema = FileSchema {
    name: "CIS_CIP_bdpm",
    filename: "CIS_CIP_bdpm.txt",
    download_path: "/download/file/CIS_CIP_bdpm.txt",
    encoding: Encoding::Utf8,
    field_count: 12,
    date_fields: &[(5, DateFormat::DDMMYYYY)],
    has_trailing_tab_fix: true,
    target_table: "presentations",
};

const CIS_COMPO_BDPM: FileSchema = FileSchema {
    name: "CIS_COMPO_bdpm",
    filename: "CIS_COMPO_bdpm.txt",
    download_path: "/download/file/CIS_COMPO_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 8,
    date_fields: &[],
    has_trailing_tab_fix: false,
    target_table: "compositions",
};

const CIS_HAS_SMR_BDPM: FileSchema = FileSchema {
    name: "CIS_HAS_SMR_bdpm",
    filename: "CIS_HAS_SMR_bdpm.txt",
    download_path: "/download/file/CIS_HAS_SMR_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 6,
    // Field 3: decision_date — YYYYMMDD integer
    date_fields: &[(3, DateFormat::YYYYMMDD)],
    has_trailing_tab_fix: false,
    target_table: "smr",
};

const CIS_HAS_ASMR_BDPM: FileSchema = FileSchema {
    name: "CIS_HAS_ASMR_bdpm",
    filename: "CIS_HAS_ASMR_bdpm.txt",
    download_path: "/download/file/CIS_HAS_ASMR_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 6,
    // Field 3: decision_date — YYYYMMDD integer
    date_fields: &[(3, DateFormat::YYYYMMDD)],
    has_trailing_tab_fix: false,
    target_table: "asmr",
};

const CIS_GENER_BDPM: FileSchema = FileSchema {
    name: "CIS_GENER_bdpm",
    filename: "CIS_GENER_bdpm.txt",
    download_path: "/download/file/CIS_GENER_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 5,
    date_fields: &[],
    has_trailing_tab_fix: false,
    target_table: "generic_groups",
};

const CIS_CPD_BDPM: FileSchema = FileSchema {
    name: "CIS_CPD_bdpm",
    filename: "CIS_CPD_bdpm.txt",
    download_path: "/download/file/CIS_CPD_bdpm.txt",
    encoding: Encoding::Windows1252,
    field_count: 2,
    date_fields: &[],
    has_trailing_tab_fix: false,
    target_table: "prescription_rules",
};

const CIS_CIP_DISPO_SPEC: FileSchema = FileSchema {
    name: "CIS_CIP_Dispo_Spec",
    filename: "CIS_CIP_Dispo_Spec.txt",
    download_path: "/download/file/CIS_CIP_Dispo_Spec.txt",
    encoding: Encoding::Latin1,
    field_count: 8,
    date_fields: &[
        (4, DateFormat::DDMMYYYY), // date_debut
        (5, DateFormat::DDMMYYYY), // date_fin (nullable)
        (6, DateFormat::DDMMYYYY), // date_remise (nullable)
    ],
    has_trailing_tab_fix: false,
    target_table: "availability",
};

const CIS_MITM: FileSchema = FileSchema {
    name: "CIS_MITM",
    filename: "CIS_MITM.txt",
    download_path: "/download/file/CIS_MITM.txt",
    encoding: Encoding::Windows1252,
    field_count: 4,
    date_fields: &[],
    has_trailing_tab_fix: false,
    target_table: "mitm",
};

const HAS_LIENS_PAGE_CT_BDPM: FileSchema = FileSchema {
    name: "HAS_LiensPageCT_bdpm",
    filename: "HAS_LiensPageCT_bdpm.txt",
    download_path: "/download/file/HAS_LiensPageCT_bdpm.txt",
    encoding: Encoding::Utf8,
    field_count: 2,
    date_fields: &[],
    has_trailing_tab_fix: false,
    target_table: "has_links",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_files_have_unique_filenames() {
        let mut names: Vec<&str> = BDPMFile::all()
            .iter()
            .map(|f| f.filename())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), BDPMFile::all().len());
    }

    #[test]
    fn cis_cip_trailing_tab_flag() {
        assert!(CIS_CIP_BDPM.has_trailing_tab_fix);
        // All others should be false
        for f in BDPMFile::all() {
            if !matches!(f, BDPMFile::CIS_CIP_bdpm) {
                assert!(!f.schema().has_trailing_tab_fix, "{}", f.filename());
            }
        }
    }

    #[test]
    fn smr_asmr_have_yyyymmdd_dates() {
        assert_eq!(
            BDPMFile::CIS_HAS_SMR_bdpm.schema().date_fields[0],
            (3, DateFormat::YYYYMMDD)
        );
        assert_eq!(
            BDPMFile::CIS_HAS_ASMR_bdpm.schema().date_fields[0],
            (3, DateFormat::YYYYMMDD)
        );
    }

    #[test]
    fn cis_bdpm_has_12_fields() {
        assert_eq!(CIS_BDPM.field_count, 12);
    }
}
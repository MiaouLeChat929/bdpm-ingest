use phf::{phf_map, phf_set};
use unicode_normalization::UnicodeNormalization;
use gtin_validate::gtin13;

/// Strip combining diacritical marks (U+0300-U+036F) via NFD decomposition.
/// Produces ASCII-searchable form: "Doliprane" matches "Doliprane", "Doliprane" etc.
/// Use for accent-insensitive comparison and FTS normalization.
pub fn strip_diacritics(s: &str) -> String {
    s.nfd().filter(|c| !matches!(c, '\u{0300}'..='\u{036F}')).collect()
}

/// Strip leading/trailing whitespace from a field.
/// Handles BDPM quirk: lab names start with ' '.
pub fn strip_field(raw: &str) -> String {
    raw.trim().to_string()
}

/// Normalize double-spaces in a string (CIS_GENER names).
pub fn normalize_spaces(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip 34009 prefix from EAN to get canonical CIP-7.
/// If already 7 digits, return as-is.
pub fn strip_cip_ean(raw: &str) -> String {
    let t = raw.trim();
    if t.len() == 13 && t.starts_with("34009") {
        t[6..].to_string()
    } else {
        t.to_string()
    }
}

/// Normalize generic type: "0"→"reference", "1"→"generic", "2"→"cross-group", "4"→"sustained-release".
pub fn normalize_generic_type(raw: &str) -> &'static str {
    match raw.trim() {
        "0" => "reference",
        "1" => "generic",
        "2" => "cross-group",
        "4" => "sustained-release",
        _ => "unknown",
    }
}

/// Validate EAN-13 / CIP13 checksum. Returns true if valid, false otherwise.
/// Invalid checksums indicate data quality issues (legacy codes, import errors).
pub fn validate_ean13(code: &str) -> bool {
    let code = code.trim();
    if code.len() != 13 { return false; }
    gtin13::check(code)
}

// ---------------------------------------------------------------------------
// Form & Route Canonicalization (Task 1 + Task 2)
// ---------------------------------------------------------------------------

/// Canonical form mapping: 800+ raw forms → ~50 canonical codes.
/// Based on ANSM pharmaceutical form taxonomy. Values are short codes
/// used in UI labels and FTS normalization.
/// Each key is separate — phf_map! does not support OR syntax.
pub static FORM_CANONICAL: phf::Map<&'static str, &'static str> = phf_map! {
    // Comprimés
    "comprime" => "CPR",
    "comprime pellicule" => "CPR",
    "comprime pellicule secable" => "CPR",
    "comprime enrobe" => "CPR",
    "comprime enrobe gastro" => "CPR",
    "comprime secable" => "CPR",
    "comprime gastro-resistant(e)" => "CPR",
    "comprime pellicule gastro-resistant(e)" => "CPR",
    "comprime enrobe gastro-resistant(e)" => "CPR",
    "comprime a liberation prolongee" => "CPR",
    "comprime pellicule a liberation prolongee" => "CPR",
    "comprime a liberation modifiee" => "CPR",
    "comprime quadrisecable" => "CPR",
    "comprime effervescent(e) secable" => "CPR_DISP",
    "comprime dispersible ou a croquer" => "CPR_DISP",
    "comprime dispersible secable" => "CPR_DISP",
    "comprime dispersible" => "CPR_DISP",
    "comprime effervescent" => "CPR_DISP",
    "comprime effervescent(e)" => "CPR_DISP",
    "comprime a avaler" => "CPR_DISP",
    "comprime orodispersible" => "CPR_DISP",
    "comprime a sucer" => "CPR_SUCK",
    "comprime a dissoudre" => "CPR_DISP",
    "comprime a croquer" => "CPR_DISP",
    // Gélules
    "gelule" => "GEL",
    "gelule molle" => "GEL",
    "gelule a liberation prolongee" => "GEL",
    "gelule gastro-resistant(e)" => "GEL",
    "capsule" => "GEL",
    "capsule molle" => "GEL",
    // Injectables
    "solution injectable" => "INJ",
    "solution injectable ou pour perfusion" => "INJ",
    "suspension injectable" => "INJ",
    "suspension injectable a liberation prolongee" => "INJ",
    "solution pour injection" => "INJ",
    "ampoule injectable" => "INJ",
    "flacon injectable" => "INJ",
    "emulsion pour perfusion" => "INJ",
    "solution a diluer pour perfusion" => "INJ",
    "solution pour perfusion" => "INJ",
    "poudre pour solution injectable" => "INJ_POW",
    "poudre pour solution injectable ou pour perfusion" => "INJ_POW",
    "poudre pour solution a diluer pour perfusion" => "INJ_POW",
    "poudre pour solution pour perfusion" => "INJ_POW",
    "poudre injectable" => "INJ_POW",
    "poudre et solvant pour solution injectable" => "INJ_POW",
    // Liquides
    "solution buvable" => "SOL",
    "solution buvable en gouttes" => "SOL",
    "solution" => "SOL",
    "sirop" => "SOL",
    "suspension buvable" => "SOL",
    "granules pour solution buvable" => "SOL",
    "granules pour suspension buvable" => "SOL",
    "poudre pour suspension buvable" => "POW",
    "collyre" => "COL",
    "collyre en solution" => "COL",
    "solution pour instillation" => "COL",
    "gouttes" => "COL",
    "suppositoire" => "SUP",
    "suppo" => "SUP",
    "pommade" => "POM",
    "creme" => "POM",
    "gel" => "POM",
    "gel pour application" => "POM",
    "pommade dermatologique" => "POM",
    "solution pour application" => "POM",
    "patch" => "PATCH",
    "dispositif transdermique" => "PATCH",
    "granules" => "GRAN",
    "granules enrobe" => "GRAN",
    "globules" => "GRAN",
    "poudre" => "POW",
    "poudre pour solution buvable" => "POW",
    "poudre pour inhalation" => "POW",
    "poudre pour inhalation en gelule" => "POW",
    // Inhalation
    "solution pour inhalation" => "INH",
    "solution pour inhalation par nebuliseur" => "INH",
    "suspension pour inhalation" => "INH",
    "suspension pour inhalation par nebuliseur" => "INH",
    "gaz pour inhalation" => "INH",
    // Sprays
    "solution pour pulverisation" => "SPRAY",
    "suspension pour pulverisation" => "SPRAY",
    // Misc
    "dispositif" => "DEVICE",
    "lyophilisat" => "LYO",
    "pastille" => "PAST",
    "gomme a macher medicamenteux(se)" => "PAST",
    "vernis a ongles medicamenteux(se)" => "VER",
    "trousse pour preparation radiopharmaceutique" => "RADIO",
    "solution pour bain de bouche" => "MOUTHWASH",
    // Dispersion/perfusion
    "dispersion pour perfusion" => "INJ",
    "poudre pour dispersion injectable pour perfusion" => "INJ_POW",
    "poudre et solution pour usage parenteral" => "INJ_POW",
    // Extended-release and modified-release variants
    "comprime enrobe a liberation prolongee" => "CPR",
    "comprime pellicule secable a liberation prolongee" => "CPR",
    "comprime secable a liberation prolongee" => "CPR",
    "comprime secable a liberation modifiee" => "CPR",
    "comprime pellicule a liberation modifiee" => "CPR",
    "solution injectable a liberation prolongee" => "INJ",
    "gelule a liberation modifiee" => "GEL",
    "granules a liberation prolongee" => "GRAN",
    "ovule a liberation prolongee" => "SUP",
    "collyre en suspension" => "COL",
    // Injectable variants
    "dispersion injectable" => "INJ",
    "solution injectable pour perfusion" => "INJ",
    "solution injectable pour usage dentaire" => "INJ",
    "emulsion injectable" => "INJ",
    "poudre et solvant pour suspension injectable" => "INJ_POW",
    "poudre et solvant pour solution pour perfusion" => "INJ_POW",
    "poudre pour suspension injectable" => "INJ_POW",
    // Solution variants
    "solution et solution pour dialyse peritoneale" => "DIALYSE",
    "solution et solution pour perfusion" => "INJ",
    "solution pour lavage" => "SOL",
    "solution buvable gouttes" => "SOL",
    // Application/topical
    "poudre pour application" => "POW",
    "emulsion pour application" => "POM",
    "emplatre medicamenteux(se)" => "PATCH",
    // Misc
    "gelule et gelule" => "GEL",
    "film orodispersible" => "CPR_DISP",
    // Combination forms (et = and, after normalize_spaces fixes double-space)
    "comprime pellicule et comprime pellicule" => "CPR",
    "comprime et comprime" => "CPR",
    // Hemodialysis/perfusion combinations
    "solution et solution pour hemofiltration pour hemodialyse et pour hemodiafiltration" => "INJ",
    "poudre et solution pour preparation injectable" => "INJ_POW",

    // ============================================================
    // NEW ENTRIES from non_canonical_forms.tsv (238 forms)
    // ============================================================

    // Gaz medical
    "gaz" => "GAZ",

    // Shampooing
    "shampooing" => "SHAMPOO",

    // Ovules (already have "ovule a liberation prolongee")
    "ovule" => "SUP",

    // Suspensions (various) — "suspension injectable" already defined above
    "suspension" => "SOL",
    "suspension pour injection" => "INJ",
    "suspension pour instillation" => "COL",
    // (suspension pour inhalation already defined above)
    // (suspension buvable already defined above)
    "suspension buvable ou" => "SOL",
    "suspension buvable ou pour instillation" => "SOL",
    "suspension buvable a diluer" => "SOL",
    "suspension colloidale injectable" => "INJ",
    "suspension et gel pour gel" => "POM",
    "suspension et poudre effervescent(e) pour suspension buvable" => "SOL",

    // Systeme de diffusion
    "systeme de diffusion" => "DEVICE",

    // Poudre et solvant combinations (injectable/perfusion)
    "poudre et solvant pour solution a diluer pour perfusion" => "INJ_POW",
    "poudre et solvant pour solution injectable ou pour perfusion" => "INJ_POW",
    "poudre et solvant pour solution injectable pour perfusion" => "INJ_POW",
    "poudre et solvant pour solution injectable pour perfusion ou buvable" => "INJ_POW",
    "poudre et solvant injectable injectable pour solution injectable" => "INJ_POW",
    "poudre et solvant pour solution injectable et pour perfusion" => "INJ_POW",
    "poudre et solvant" => "INJ_POW",
    // (poudre et solvant pour suspension injectable already defined above)
    // (poudre et solvant pour solution pour perfusion already defined above)
    "poudre et solvant pour suspension" => "INJ_POW",
    "poudre et solvant pour solution" => "INJ_POW",
    "poudre et solvant pour preparation injectable" => "INJ_POW",
    "poudre et solvant et solvant pour solution injectable" => "INJ_POW",
    "poudre et solvant pour dispersion injectable" => "INJ_POW",
    "poudre et solvant pour suspension pour instillation" => "INJ_POW",
    "poudre et solvant pour solution pour solution pour solution a diluer pour perfusion" => "INJ_POW",
    "poudre et solvant pour solution pour inhalation par nebuliseur" => "INH",
    "poudre et solvant pour inhalation par nebuliseur" => "INH",
    "poudre et solvant et matrice pour matrice implantable" => "INJ_POW",
    "poudre et solvant pour suspension injectable a liberation prolongee" => "INJ_POW",
    "poudre et solution pour solution injectable" => "INJ_POW",
    "emulsion injectable pour perfusion" => "INJ",
    "comprime a sucer secable" => "CPR_SUCK",
    "comprime pellicule et comprime pellicule et comprime pellicule" => "CPR",
    "poudre pour solution buvable et enteral(e)" => "SOL",

    // Liquide pour inhalation
    "liquide pour inhalation par vapeur" => "INH",
    "liquide" => "SOL",

    // Comprime variants (croquer/sucer combinations)
    "comprime a sucer ou a croquer" => "CPR_SUCK",
    "comprime a croquer ou a sucer" => "CPR_SUCK",
    "comprime a croquer a sucer ou dispersible" => "CPR_SUCK",
    "comprime(s)" => "CPR",
    "comprime enrobe secable" => "CPR",
    "comprime secable pellicule" => "CPR",
    "comprime pellicule quadrisecable" => "CPR",
    "comprime osmotique" => "CPR",
    "comprime muco-adhesif" => "CPR",
    "comprime a liberation modifiee secable" => "CPR",
    "comprime enrobe et comprime enrobe" => "CPR",
    "comprime enrobe et comprime enrobe et comprime enrobe" => "CPR",
    "comprime pellicule et comprime pellicule et comprime pellicule pellicule" => "CPR",
    "comprime et comprime et comprime" => "CPR",
    "comprime et comprime pellicule" => "CPR",
    "comprime et gelule" => "CPR",
    "comprime pellicule et granules effervescent(e)" => "CPR",
    "comprime pour solution buvable" => "CPR",
    "comprime pour suspension buvable" => "CPR",
    "comprime secable pour suspension buvable" => "CPR",

    // Dialyse
    "solution pour dialyse peritoneale" => "DIALYSE",
    // (solution et solution pour dialyse peritoneale already defined above)

    // Poudre et poudre combinations
    "poudre et poudre pour inhalation" => "POW",
    "poudre et poudre pour solution buvable" => "POW",
    "poudre et poudre" => "POW",
    "poudre et poudre effervescent(e) pour suspension buvable" => "POW",
    "poudre buvable buvable et poudre buvable buvable" => "POW",
    "poudre buvable buvable et poudre buvable buvable et poudre buvable buvable" => "POW",

    // Lyophilisat
    "lyophilisat pour usage parenteral" => "LYO",
    "lyophilisat et solution pour usage parenteral" => "LYO",
    "lyophilisat pour preparation injectable" => "LYO",
    "lyophilisat et solvant pour collyre" => "LYO",
    "lyophilisat et solution pour preparation injectable" => "LYO",

    // Implant
    "implant" => "IMPLANT",

    // Poudre injectable/perfusion with various modifiers
    "poudre pour solution injectable pour perfusion ou buvable" => "INJ_POW",
    // (poudre pour dispersion injectable pour perfusion already defined above)
    "poudre et suspension pour suspension injectable" => "INJ_POW",
    "poudre pour solution injectable pour perfusion ou" => "INJ_POW",
    "poudre pour solution injectable pour perfusion" => "INJ_POW",
    "poudre pour solution" => "POW",
    "poudre pour dispersion pour perfusion" => "INJ_POW",
    "poudre pour usage parenteral" => "INJ_POW",
    // (poudre pour suspension injectable already defined above)
    // (poudre et solution pour usage parenteral already defined above)
    "poudre pour solution injectable pour perfusion ou pour inhalation par nebuliseur" => "INJ_POW",
    "poudre pour solution buvable et gastro-enterale" => "POW",
    "poudre pour solution a diluer pour perfusion ou buvable" => "INJ_POW",
    "poudre pour concentre pour solution pour perfusion" => "INJ_POW",
    "poudre a diluer pour solution pour perfusion" => "INJ_POW",
    "poudre pour solution pour injection ou pour perfusion" => "INJ_POW",
    "poudre pour solution a diluer injectable ou pour perfusion" => "INJ_POW",
    "poudre pour injection ou pour perfusion" => "INJ_POW",
    "poudre pour injection" => "INJ_POW",
    "poudre pour dispersion a diluer pour perfusion" => "INJ_POW",
    "poudre pour aerosol et pour usage parenteral" => "INJ_POW",
    "poudre et solution pour usage parenteral a diluer" => "INJ_POW",
    "poudre a diluer a diluer et solution pour solution pour perfusion" => "INJ_POW",
    "poudre pour solution injectable ou" => "INJ_POW",
    "poudre pour suspension ou" => "POW",

    // Pastille
    "pastille a sucer" => "PAST",

    // Dispersion
    "dispersion a diluer pour dispersion injectable" => "INJ",
    // (dispersion injectable already defined above)
    "dispersion injectable ou pour perfusion" => "INJ",
    "dispersion a diluer pour perfusion" => "INJ",
    "dispersion pour inhalation par nebuliseur" => "INH",
    "dispersion et dispersion pour perfusion" => "INJ",

    // Comprime dispersible/orodispersible combinations
    "comprime dispersible et orodispersible" => "CPR_DISP",
    "comprime pellicule dispersible" => "CPR_DISP",
    "comprime orodispersible secable" => "CPR_DISP",

    // Collutoire
    "collutoire" => "COLLUTOIRE",

    // Radiopharmaceutique
    "trousse et trousse pour preparation radiopharmaceutique" => "RADIO",
    "trousse et trousse pour preparation radiopharmaceutique pour injection" => "RADIO",
    "trousse radiopharmaceutique" => "RADIO",
    "trousse pour preparation radiopharmaceutique et trousse pour preparation radiopharmaceutique" => "RADIO",
    "trousse et trousse radiopharmaceutique" => "RADIO",
    "generateur radiopharmaceutique" => "RADIO",
    "precurseur radiopharmaceutique" => "RADIO",
    "precurseur radiopharmaceutique en solution" => "RADIO",

    // Solution injectable/perfusion (various combinations)
    "solution injectable pour perfusion ou" => "INJ",
    "solution et solution et emulsion pour perfusion" => "INJ",
    "solution injectable et solution injectable" => "INJ",
    "solution a diluer pour solution injectable ou pour perfusion" => "INJ",
    "solution a diluer injectable" => "INJ",
    "emulsion injectable ou pour perfusion" => "INJ",
    "solution injectable et buvable" => "SOL",
    // (solution injectable pour perfusion already defined above)
    // (solution injectable a liberation prolongee already defined above)
    // (solution injectable pour usage dentaire already defined above)
    "solution injectable pour usage parenteral" => "INJ",
    // (emulsion injectable already defined above)
    // (emulsion injectable pour perfusion already defined above)
    // (solution pour injection already defined above)
    "solution pour injection ou pour perfusion" => "INJ",
    "solution a diluer et solvant pour solution pour perfusion" => "INJ",
    "solution a diluer et solvant pour solution injectable" => "INJ",
    "solution injectable ou a diluer pour perfusion" => "INJ",
    "solution injectable hypertonique pour perfusion" => "INJ",
    "solution injectable a diluer ou pour perfusion" => "INJ",
    "solution injectable a diluer" => "INJ",
    "solution et suspension pour suspension injectable" => "INJ",
    "solution pour preparation injectable" => "INJ",
    "solution pour preparation injectable ou pour perfusion" => "INJ",
    "solution pour perfusion ou" => "INJ",
    "solution ou injectable" => "INJ",
    "solution a diluer injectable ou pour perfusion" => "INJ",
    "solution a diluer et solvant pour solution a diluer pour perfusion" => "INJ",
    "solution cardioplegique" => "INJ",
    "solution cardioplegique ou" => "INJ",
    "solution a diluer pour solution pour perfusion" => "INJ",
    "solution a diluer pour injection ou pour perfusion" => "INJ",

    // Pate
    "pate dentifrice" => "POM",
    "pate pour application" => "POM",
    "pate" => "POM",
    "pate a sucer" => "PAST",

    // Microgranules
    "microgranule a liberation prolongee en gelule" => "GEL",
    "microgranule en comprime" => "CPR",
    "microgranule gastro-resistant(e) en gelule" => "GEL",

    // Granules (various)
    "granules en gelule" => "GRAN",
    "granules gastro-resistant(e)" => "GRAN",
    "granules effervescent(e) pour solution buvable" => "GRAN",
    "granules orodispersible" => "GRAN",
    "granules gastro-resistant(e) pour suspension buvable" => "GRAN",
    "granules et solvant pour suspension buvable" => "GRAN",
    "granules enrobe en vrac" => "GRAN",
    "granules a croquer" => "GRAN",

    // Suppositoire variants
    "suppositoire secable" => "SUP",
    "suppositoire effervescent(e)" => "SUP",

    // Solvant
    "solvant pour preparation parenterale" => "SOL",
    "solvant(s) et poudre(s) pour solution injectable" => "INJ_POW",

    // Solution pour hemo
    "solution et solution pour hemodialyse pour hemofiltration" => "INJ",
    "solution et solution pour hemofiltration et pour hemodialyse" => "INJ",
    "solution et solution pour hemodialyse et pour hemofiltration" => "INJ",
    "solution pour hemofiltration" => "SOL",

    // Solution pour colle
    "solution et solution pour colle" => "SOL",

    // Poudre effervescent
    "poudre effervescent(e) pour suspension buvable" => "POW",
    "poudre effervescent(e) pour solution buvable" => "POW",

    // Pansement
    "pansement adhesif(ve)" => "POM",
    "pansement medicamenteux(se)" => "POM",

    // Comprime pour solution buvable (already defined above)

    // Compresse
    "compresse impregne(e)" => "COMPRESSE",
    "compresse impregne(e) pour usage dentaire" => "COMPRESSE",
    "compresse et solution(s) et generateur radiopharmaceutique" => "COMPRESSE",

    // Collyre
    "collyre a liberation prolongee" => "COL",
    "collyre en solution a liberation prolongee" => "COL",
    "collyre en emulsion" => "COL",
    "collyre en solution unidose" => "COL",

    // Baton
    "baton pour application" => "BATON",

    // Solution pour inhalation
    "solution pour inhalation par fumigation" => "INH",

    // Solution gouttes
    "solution gouttes" => "SOL",

    // Solution buvable
    "solution buvable ou" => "SOL",
    "solution buvable et injectable" => "SOL",
    "solution buvable a diluer" => "SOL",
    "solution a diluer pour solution buvable" => "SOL",

    // Solution et solution
    // (solution et solution pour perfusion already defined above)
    "solution et solution buvable" => "SOL",
    "solution et solution pour marquage" => "SOL",
    "solution et solution pour application" => "POM",
    "solution et emulsion pour emulsion injectable" => "INJ",
    "solution et emulsion et solution pour perfusion" => "INJ",
    "solution et" => "SOL",
    // (solution already defined above)
    "solution ou" => "SOL",

    // Solution moussant
    "solution moussant(e)" => "SOL",

    // Solution pour bain de bouche
    "solution pour gargarisme ou pour bain de bouche" => "MOUTHWASH",

    // Solution pour pulverisation
    "solution pour pulverisation et" => "SPRAY",
    "solution pour pulverisation endo-buccal(e)" => "SPRAY",

    // Solution pour preparation parenterale
    "solution pour preparation parenterale" => "SOL",

    // Solution pour marquage
    "solution pour marquage" => "SOL",

    // Solution pour irrigation
    "solution pour irrigation oculaire" => "COL",

    // Solution sterile
    "solution pour application sterile" => "POM",

    // Solution pour administration
    "solution pour administration intravesicale" => "SOL",

    // Solution filmogene
    "solution filmogene pour application" => "POM",

    // Solution concentre
    "solution concentre(e) a diluer pour solution pour perfusion" => "INJ",

    // Poudre en gelule
    "poudre en gelule" => "GEL",
    "poudre en gelule et poudre en gelule" => "GEL",
    "poudre en gelule et poudre en gelule en gelule" => "GEL",

    // Plante(s) pour tisane
    "plante(s) pour tisane" => "TISANE",
    "plante(s) pour tisane en vrac" => "TISANE",
    "plante(s) en vrac" => "TISANE",
    "melange de plantes pour tisane" => "TISANE",

    // Pate pour usage dentaire
    "pate pour usage dentaire" => "POM",

    // Mousse
    "mousse pour application" => "MOUSSE",
    "mousse" => "MOUSSE",

    // Microsphere
    "microsphere et solution pour usage parenteral ou a liberation prolongee" => "INJ_POW",

    // Gomme
    "gomme a macher" => "PAST",
    "gomme" => "PAST",

    // Gel
    "gel intestinal" => "POM",
    "gel sterile" => "POM",
    "gel pour usage dentaire" => "POM",
    "gel et" => "POM",
    "gel dentifrice" => "POM",
    "gel buvable" => "SOL",

    // Emplatre
    "emplatre adhesif(ve)" => "PATCH",
    "emplatre" => "PATCH",

    // Dispositif
    "dispositif pour application" => "DEVICE",

    // Solution pour prick-test
    "solution pour prick-test" => "SOL",

    // Tampon impregne
    "tampon impregne(e) pour inhalation par fumigation" => "INH",
    "tampon impregne(e)" => "POM",

    // Pommade
    "pommade pour application et" => "POM",

    // Matrice
    "matrice" => "DEVICE",

    // Insert
    "insert" => "INSERT",

    // Gelee
    "gelee" => "POM",

    // Emulsion
    "emulsion fluide pour application" => "POM",
    "emulsion pour inhalation par fumigation" => "INH",

    // Creme
    "creme sterile" => "POM",
    "creme pour usage dentaire" => "POM",
    "creme pour application" => "POM",
    "creme epaisse pour application" => "POM",

    // Capsule molle
    "capsule molle ou" => "GEL",

    // Capsule pour inhalation
    "capsule pour inhalation par vapeur" => "INH",

    // Cartouche
    "cartouche pour inhalation" => "CARTOUCHE",

    // Bain de bouche
    "bain de bouche" => "MOUTHWASH",

    // Lotion
    "lotion" => "LOTION",

    // Patch
    "patch(s)" => "PATCH",
    "patch et gel" => "PATCH",

    // Poudre pour suspension
    "poudre pour suspension et" => "POW",

    // Poudre pour solution pour inhalation par nebuliseur
    "poudre pour solution pour inhalation par nebuliseur" => "INH",
};

/// Canonicalize a BDPM form string to short code. Falls back to lowercased input.
/// Strips diacritics so accented inputs like "comprimé" match ASCII keys like "comprime".
pub fn canonicalize_form(form: &str) -> String {
    let normalized = strip_diacritics(&form.trim().to_lowercase());
    // Collapse double-spaces so "poudre et  solvant" matches "poudre et solvant"
    let collapsed: String = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    FORM_CANONICAL.get(&collapsed).map(|s| s.to_string()).unwrap_or(collapsed)
}

/// Route canonical map: raw French route strings → canonical forms.
/// Each key is separate — phf_map! does not support OR syntax.
/// Diacritics are stripped before lookup, so "intraveineuse" and "intraveineuse"
/// both match "intraveineuse" key.
pub static ROUTE_CANONICAL: phf::Map<&'static str, &'static str> = phf_map! {
    "orale" => "orale",
    "voie orale" => "orale",
    "usage oral" => "orale",
    "cutanee" => "cutanee",
    "usage cutane" => "cutanee",
    "dermique" => "cutanee",
    "rectale" => "rectale",
    "voie rectale" => "rectale",
    "usage rectal" => "rectale",
    "vaginale" => "vaginale",
    "voie vaginale" => "vaginale",
    "usage vaginal" => "vaginale",
    "inhalation" => "inhalation",
    "usage inhalation" => "inhalation",
    "pour inhalation" => "inhalation",
    "inhalee" => "inhalation",  // "inhalée" stripped of diacritics
    "inhale" => "inhalation",   // "inhalé" stripped of diacritics
    "oculaire" => "oculaire",
    "voie oculaire" => "oculaire",
    "usage ophtalmique" => "oculaire",
    "auriculaire" => "auriculaire",
    "voie auriculaire" => "auriculaire",
    "sublinguale" => "sublinguale",
    "voie sublinguale" => "sublinguale",
    "transdermique" => "transdermique",
    "dispositif transdermique" => "transdermique",
    "intraveineuse" => "intraveineuse",
    "intraveineux" => "intraveineuse",
    "voie intraveineuse" => "intraveineuse",
    "intramusculaire" => "intramusculaire",
    "voie intramusculaire" => "intramusculaire",
    "sous-cutanee" => "sous-cutanee",
    "voie sous-cutanee" => "sous-cutanee",
    "ophtalmique" => "oculaire",
    "buccale" => "buccale",
    "voie buccale" => "buccale",
    "nasale" => "nasale",
    "voie nasale" => "nasale",
};

/// Canonicalize a BDPM route string. Strips diacritics before lookup so
/// accented inputs like "intraveineuse" match the ASCII key "intraveineuse".
/// Handles semicolon-separated multi-value routes (e.g., "intramusculaire;intraveineuse").
pub fn canonicalize_route(route: &str) -> String {
    let normalized = strip_diacritics(&route.trim().to_lowercase());
    // Handle semicolon-separated multi-value routes
    if normalized.contains(';') {
        return normalized.split(';')
            .map(|part| {
                let trimmed = part.trim();
                ROUTE_CANONICAL.get(trimmed).map(|s| s.to_string()).unwrap_or(trimmed.to_string())
            })
            .collect::<Vec<_>>()
            .join(";");
    }
    ROUTE_CANONICAL.get(&normalized).map(|s| s.to_string()).unwrap_or(normalized)
}

// ---------------------------------------------------------------------------
// Lab Family Canonicalization (Task 3)
// ---------------------------------------------------------------------------

/// Lab family mapping: subsidiary names → canonical family name.
/// Used to group drugs from the same parent company (Viatris, Mylan, Teva, etc.).
pub static LAB_FAMILY_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    // Viatris family
    "VIATRIS SOLUTE INJ" => "VIATRIS",
    "VIATRIS FRANCE" => "VIATRIS",
    "VIATRIS SANTE" => "VIATRIS",
    "VIATRIS HOLDING" => "VIATRIS",
    "VIATRIS MEDICAL" => "VIATRIS",
    "VIATRIS UP" => "VIATRIS",
    "VIATRIS HEALTHCARE" => "VIATRIS",
    "VIATRIS" => "VIATRIS",
    // Mylan family
    "MYLAN PHARMA" => "MYLAN",
    "MYLAN MEDICAL" => "MYLAN",
    "MYLAN LABORATORIES" => "MYLAN",
    "MYLAN SAS" => "MYLAN",
    "MYLAN PHARMACEUTICALS" => "MYLAN",
    "MYLAN" => "MYLAN",
    "MYLAN IRELAND" => "MYLAN",
    "MYLAN IRE HEALTHCARE" => "MYLAN",
    "ACTAVIS GROUP" => "MYLAN",
    "ACTAVIS FRANCE" => "MYLAN",
    "ACTAVIS GROUP PTC" => "MYLAN",
    // Biogaran
    "BIOGARAN" => "BIOGARAN",
    "BIOGARAN SACHE" => "BIOGARAN",
    // Arrow
    "ARROW GENERIQUES" => "ARROW",
    "ARROW FRANCE" => "ARROW",
    "ARROW LABORATOIRES" => "ARROW",
    "ARROW" => "ARROW",
    // Teva
    "TEVA HEALTH" => "TEVA",
    "TEVA SANTE" => "TEVA",
    "TEVA FRANCE" => "TEVA",
    "TEVA BV" => "TEVA",
    "TEVA" => "TEVA",
    // Sandoz
    "SANDOZ" => "SANDOZ",
    "SANDOZ GMBH" => "SANDOZ",
    // Sanofi family
    "SANOFI" => "SANOFI",
    "SANOFI WINTHROP" => "SANOFI",
    "SANOFI WINTHROP INDUSTRIE" => "SANOFI",
    "SANOFI AVENTIS" => "SANOFI",
    "NOFI WINTHROP INDUSTRIE" => "SANOFI",
    "NOFI AVENTIS" => "SANOFI",
    "NOFI-AVENTIS GROUPE" => "SANOFI",
    "OPELLA HEALTHCARE" => "SANOFI",
    // EG Labo
    "EG LABO - LABORATOIRES EUROGENERICS" => "EG LABO",
    "EG LABO" => "EG LABO",
    // Accord
    "ACCORD HEALTHCARE" => "ACCORD",
    "ACCORD HEALTHCARE FRANCE" => "ACCORD",
    "ACCORD HEALTHCARE (ESPAGNE)" => "ACCORD",
    "ACCORD" => "ACCORD",
    // Pfizer family
    "PFIZER HOLDING" => "PFIZER",
    "PFIZER HOLDING FRANCE" => "PFIZER",
    "PFIZER EUROPE MA EEIG" => "PFIZER",
    "PFIZER P.G.M." => "PFIZER",
    "PFIZER IRELAND PHARMACEUTICALS" => "PFIZER",
    "PFIZER" => "PFIZER",
    // GSK family
    "GLAXOSMITHKLINE" => "GSK",
    "GLAXOSMITHKLINE HEALTHCARE" => "GSK",
    "GLAXOSMITHKLINE TRADING SERVICES" => "GSK",
    "GLAXOSMITHKLINE BIOLOGICALS" => "GSK",
    "GSK VACCINES SRL" => "GSK",
    "GSK" => "GSK",
    // Novartis family
    "NOVARTIS EUROPHARM" => "NOVARTIS",
    "NOVARTIS PHARMA" => "NOVARTIS",
    "NOVARTIS" => "NOVARTIS",
    // Servier family
    "LES LABORATOIRES SERVIER" => "SERVIER",
    "BIOCODEX" => "SERVIER",
    "SERVIER" => "SERVIER",
    // Roche family
    "ROCHE REGISTRATION" => "ROCHE",
    "ROCHE" => "ROCHE",
    // Janssen family
    "JANSSEN CILAG INTERNATIONAL NV" => "JANSSEN",
    "JANSSEN CILAG" => "JANSSEN",
    "JANSSEN" => "JANSSEN",
    // Amgen
    "AMGEN EUROPE" => "AMGEN",
    "AMGEN TECHNOLOGY" => "AMGEN",
    "AMGEN" => "AMGEN",
    // Bayer
    "BAYER HEALTHCARE" => "BAYER",
    "BAYER AG" => "BAYER",
    "BAYER" => "BAYER",
    // Bouchara/Recordati family
    "BB FARMA" => "BOUCHARA",
    "BOUCHARA-RECORDATI" => "BOUCHARA",
    "BOUCHARA" => "BOUCHARA",
    // UPSA
    "UP" => "UPSA",
    // Cooper
    "COOPER" => "COOPER",
    "COOPER CONSUMER NETHERLANDS" => "COOPER",
    // Major generics companies (count >= 10)
    "ZENTIVA" => "ZENTIVA",
    "CRISTERS" => "CRISTERS",
    "ZYDUS" => "ZYDUS",
    "EVOLUPHARM" => "EVOLUPHARM",
    "KRKA" => "KRKA",
    "EUGIA" => "EUGIA",
    "ORGANON" => "ORGANON",
    "SUN" => "SUN PHARMA",
    "SUN PHARMACEUTICAL INDUSTRIES EUROPE" => "SUN PHARMA",
    "SUN PHARMA" => "SUN PHARMA",
    // Sanofi generics spinoffs
    "PIERRE FABRE MEDICAMENT" => "PIERRE FABRE",
    "PIERRE FABRE" => "PIERRE FABRE",
    // Cheplapharm
    "CHEPLAPHARM ARZNEIMITTEL" => "CHEPLAPHARM",
    "CHEPLAPHARM REGISTRATION" => "CHEPLAPHARM",
    "CHEPLAPHARM" => "CHEPLAPHARM",
    // Other notable labs (count >= 10)
    "AGUETTANT" => "AGUETTANT",
    "DIFARMED" => "DIFARMED",
    "FRESENIUS KABI" => "FRESENIUS KABI",
    "FRESENIUS KABI DEUTSCHLAND" => "FRESENIUS KABI",
    "FRESENIUS KABI AUSTRIA" => "FRESENIUS KABI",
    "FRESENIUS MEDICAL CARE DEUTSCHLAND" => "FRESENIUS",
    "FRESENIUS MEDICAL CARE NEPHROLOGICA DEUTSCHLAND" => "FRESENIUS",
    "FRESENIUS" => "FRESENIUS",
    "ALTER" => "ALTER",
    "LABORATOIRE RENAUDIN" => "LABORATOIRE RENAUDIN",
    "GRUNENTHAL" => "GRUNENTHAL",
    "HCS" => "HCS",
    "IB" => "IB",
    "TEOFARMA" => "TEOFARMA",
    "NEURAXPHARM" => "NEURAXPHARM",
    "NEURAXPHARM PHARMACEUTICALS" => "NEURAXPHARM",
    // Hikma
    "HIKMA FARMACEUTICA" => "HIKMA",
    "HIKMA" => "HIKMA",
    // Thea (with subsidiary Chauvin)
    "THEA" => "THEA",
    "LABORATOIRE CHAUVIN" => "THEA",
    // BMS
    "BRISTOL-MYERS SQUIBB" => "BMS",
    "BRISTOL-MYERS SQUIBB / PFIZER EEIG" => "BMS",
    "BRISTOL MYERS SQUIBB" => "BMS",
    "BMS" => "BMS",
    // Other companies
    "ETHYPHARM" => "ETHYPHARM",
    "KENVUE" => "KENVUE",
    // Baxter
    "BAXTER" => "BAXTER",
    "BAXALTA INNOVATIONS" => "BAXTER",
    "BAXTER HOLDING" => "BAXTER",
    // Aspen
    "ASPEN PHARMA TRADING" => "ASPEN",
    "ASPEN" => "ASPEN",
    // Boehringer
    "BOEHRINGER INGELHEIM INTERNATIONAL" => "BOEHRINGER",
    "BOEHRINGER INGELHEIM" => "BOEHRINGER",
    "BOEHRINGER" => "BOEHRINGER",
    // Menarini
    "MENARINI INTERNATIONAL OPERATIONS LUXEMBOURG" => "MENARINI",
    "MENARINI" => "MENARINI",
    "MENARINI ITALIE" => "MENARINI",
    // Bailleul
    "BAILLEUL" => "BAILLEUL",
    // Stragen
    "STRAGEN-" => "STRAGEN",
    "STRAGEN" => "STRAGEN",
    // Merck / MSD
    "MERCK SHARP & DOHME" => "MSD",
    "MERCK" => "MERCK",
    "MERCK EUROPE" => "MERCK",
    "MERCK SERONO EUROPE" => "MERCK",
    "MERCK KGAA" => "MERCK",
    "MSD" => "MSD",
    // Eli Lilly
    "ELI LILLY NEDERLAND BV" => "ELI LILLY",
    "ELI LILLY NEDERLAND" => "ELI LILLY",
    "ELI LILLY" => "ELI LILLY",
    "ELI LILLY AND COMPANY" => "ELI LILLY",
    "LILLY" => "ELI LILLY",
    // Theramex
    "THERAMEX IRELAND" => "THERAMEX",
    "THERAMEX" => "THERAMEX",
    // B. Braun
    "B BRAUN MELSUNGEN" => "B BRAUN",
    "B BRAUN MEDICAL" => "B BRAUN",
    "B BRAUN" => "B BRAUN",
    "B BRAUN AVITUM" => "B BRAUN",
    // Mundi
    "MUNDI" => "MUNDI",
    // Alfasigma
    "ALFASIGMA" => "ALFASIGMA",
    // UCB
    "UCB PHARMA BELGIQUE" => "UCB",
    "UCB" => "UCB",
    // AbbVie
    "ABBVIE DEUTSCHLAND" => "ABBVIE",
    "ABBVIE" => "ABBVIE",
    // Mayoly
    "MAYOLY SPINDLER" => "MAYOLY",
    "MAYOLY" => "MAYOLY",
    // Amdipharm
    "AMDIPHARM" => "AMDIPHARM",
    // AstraZeneca
    "ASTRAZENECA AB" => "ASTRAZENECA",
    "ASTRAZENECA" => "ASTRAZENECA",
    // Gilead
    "GILEAD SCIENCES IRELAND UC" => "GILEAD",
    "GILEAD SCIENCES" => "GILEAD",
    "GILEAD" => "GILEAD",
    // Medac
    "MEDAC GESELLSCHAFT FUR KLINISCHE SPEZIALPRAPARATE" => "MEDAC",
    "MEDAC" => "MEDAC",
    // Chaix
    "CHAIX ET DU MARAIS" => "CHAIX",
    "CHAIX" => "CHAIX",
    // Ferring
    "FERRING" => "FERRING",
    "FERRING PHARMACEUTICALS" => "FERRING",
    // Urgo
    "URGO HEALTHCARE" => "URGO",
    "URGO" => "URGO",
    // CSL Behring
    "CSL BEHRING" => "CSL BEHRING",
    // Haleon
    "HALEON" => "HALEON",
    "HALEON IRELAND DUNGARVAN LIMITED" => "HALEON",
    // Reckitt
    "RECKITT BENCKISER HEALTHCARE" => "RECKITT",
    "RECKITT" => "RECKITT",
    // Tillomed
    "TILLOMED" => "TILLOMED",
    // Octa
    "OCTA" => "OCTA",
    // Elerte
    "ELERTE" => "ELERTE",
    // Zambon
    "ZAMBON" => "ZAMBON",
    // Chiesi
    "CHIESI" => "CHIESI",
    "CHIESI FARMACEUTICI" => "CHIESI",
    // Ipsen
    "IPSEN" => "IPSEN",
    "IPSEN CONSUMER HEALTHCARE" => "IPSEN",
    // Dr. Reddy's
    "REDDY" => "DR REDDY'S",
    "DR. REDDY'S NETHERLANDS" => "DR REDDY'S",
    "REDDY HOLDING" => "DR REDDY'S",
    // Takeda
    "TAKEDA" => "TAKEDA",
    "TAKEDA PHARMACEUTICALS INTERNATIONAL" => "TAKEDA",
    "TAKEDA MANUFACTURING AUSTRIA" => "TAKEDA",
    // Celltrion
    "CELLTRION HEALTHCARE HUNGARY" => "CELLTRION",
    "CELLTRION" => "CELLTRION",
    // Delbert
    "DELBERT" => "DELBERT",
    // Phoenix
    "PHOENIX LABS" => "PHOENIX",
    "PHOENIX" => "PHOENIX",
    "PHOENIX HEALTHCARE" => "PHOENIX",
    // Besins
    "BESINS HEALTHCARE" => "BESINS",
    "BESINS" => "BESINS",
    "BESINS INTERNATIONAL" => "BESINS",
    // Galderma
    "GALDERMA INTERNATIONAL" => "GALDERMA",
    "GALDERMA" => "GALDERMA",
    // Orion
    "ORION CORPORATION" => "ORION",
    "ORION" => "ORION",
    // Kalceks
    "AS KALCEKS" => "KALCEKS",
    "KALCEKS" => "KALCEKS",
    // Cis Bio
    "CIS BIO INTERNATIONAL" => "CIS BIO",
    "CIS BIO" => "CIS BIO",
    // XO
    "LABORATOIRE XO" => "XO",
    "XO" => "XO",
    // Lundbeck
    "LUNDBECK" => "LUNDBECK",
    // Atnahs
    "ATNAHS PHARMA NETHERLANDS" => "ATNAHS",
    "ATNAHS PHARMA UK" => "ATNAHS",
    "ATNAHS" => "ATNAHS",
    // Exeltis
    "EXELTIS" => "EXELTIS",
    "EXELTIS HEALTHCARE" => "EXELTIS",
    // Stallergenes
    "STALLERGENES" => "STALLERGENES",
    "STALLERGENES GREER" => "STALLERGENES",
    // Eisai
    "EISAI" => "EISAI",
    // LFB
    "LFB-BIOMEDICAMENTS" => "LFB",
    "LFB" => "LFB",
    // Recordati
    "RECORDATI RARE DISEASES" => "RECORDATI",
    "RECORDATI INDUSTRIA CHIMICA E FARMACEUTICA" => "RECORDATI",
    "RECORDATI IRELAND" => "RECORDATI",
    "RECORDATI NETHERLANDS" => "RECORDATI",
    "RECORDATI" => "RECORDATI",
    // Almirall
    "ALMIRALL" => "ALMIRALL",
    // Guerbet
    "GUERBET" => "GUERBET",
    // Majorelle
    "MAJORELLE" => "MAJORELLE",
    // Altan
    "ALTAN PHARMACEUTICALS" => "ALTAN",
    "ALTAN" => "ALTAN",
    // Astellas
    "ASTELLAS PHARMA EUROPE" => "ASTELLAS",
    "ASTELLAS" => "ASTELLAS",
    // GE Healthcare
    "GE HEALTHCARE" => "GE HEALTHCARE",
    // Gifrer
    "GIFRER BARBEZAT" => "GIFRER",
    "GIFRER" => "GIFRER",
    // Granions
    "GRANIONS" => "GRANIONS",
    // Nordic
    "NORDIC GROUP" => "NORDIC",
    "NORDIC" => "NORDIC",
    // ViiV Healthcare
    "VIIV HEALTHCARE" => "VIIV HEALTHCARE",
    "VIIV HEALTHCARE UK" => "VIIV HEALTHCARE",
    // Air Liquide
    "AIR LIQUIDE SANTE INTERNATIONAL" => "AIR LIQUIDE",
    "AIR LIQUIDE" => "AIR LIQUIDE",
    // Effik
    "EFFIK" => "EFFIK",
    // Gedeon Richter
    "GEDEON RICHTER" => "GEDEON RICHTER",
    // Gilbert
    "GILBERT" => "GILBERT",
    // P&G
    "P&G" => "P&G",
    "PROCTER & GAMBLE" => "P&G",
    // Vertex
    "VERTEX PHARMACEUTICALS" => "VERTEX",
    "VERTEX" => "VERTEX",
    // Horus
    "HORUS" => "HORUS",
    // Karo
    "KARO" => "KARO",
    "KARO HEALTHCARE" => "KARO",
    // Leo
    "LEO PHARMA A/S" => "LEO",
    "LEO PHARMACEUTICAL PRODUCTS" => "LEO",
    "LEO" => "LEO",
    // Ratiopharm
    "RATIOPHARM" => "RATIOPHARM",
    // SERP
    "SERP" => "SERP",
    // Tamrisa
    "TAMRISA ACCESS" => "TAMRISA",
    // Curium
    "CURIUM NETHERLANDS" => "CURIUM",
    "CURIUM AUSTRIA" => "CURIUM",
    "CURIUM INTERNATIONAL" => "CURIUM",
    "CURIUM PET" => "CURIUM",
    "CURIUM PET LIEGE" => "CURIUM",
    "CURIUM" => "CURIUM",
    // Innotech
    "INNOTECH INTERNATIONAL" => "INNOTECH",
    "INNOTECH" => "INNOTECH",
    // Labcatal
    "LABCATAL" => "LABCATAL",
    // BioNTech
    "BIONTECH MANUFACTURING" => "BIONTECH",
    "BIONTECH" => "BIONTECH",
    // BioSimilar Collaborations
    "BIOSIMILAR COLLABORATIONS IRELAND" => "BIOSIMILAR COLLABORATIONS",
    "BIOSIMILAR COLLABORATIONS" => "BIOSIMILAR COLLABORATIONS",
    // Grimberg
    "GRIMBERG" => "GRIMBERG",
    // HAC
    "HAC" => "HAC",
    // Norgine
    "NORGINE" => "NORGINE",
    "NORGINE HEALTHCARE" => "NORGINE",
    // Otsuka
    "OTSUKA PHARMACEUTICAL NETHERLANDS" => "OTSUKA",
    "OTSUKA" => "OTSUKA",
    "OTSUKA NOVEL PRODUCTS" => "OTSUKA",
    // Bioprojet
    "BIOPROJET" => "BIOPROJET",
    "BIOPROJET EUROPE" => "BIOPROJET",
    // Eumedica
    "EUMEDICA PHARMACEUTICALS" => "EUMEDICA",
    "EUMEDICA" => "EUMEDICA",
    // Istituto Gentili
    "ISTITUTO GENTILI" => "ISTITUTO GENTILI",
    // Reig Jofre
    "LABORATORIO REIG JOFRE" => "REIG JOFRE",
    "REIG JOFRE" => "REIG JOFRE",
    // Panmedica
    "PANMEDICA" => "PANMEDICA",
    // Septodont
    "SEPTODONT" => "SEPTODONT",
    "SEPTODONT HOLDING" => "SEPTODONT",
    // Stada
    "STADA ARZNEIMITTEL AG" => "STADA",
    "STADA" => "STADA",
    // Advanz Pharma
    "ADVANZ PHARMA LIMITED" => "ADVANZ PHARMA",
    "ADVANZ PHARMA" => "ADVANZ PHARMA",
    // Bracco
    "BRACCO IMAGING" => "BRACCO",
    "BRACCO INTERNATIONAL" => "BRACCO",
    "BRACCO" => "BRACCO",
    // Europhta
    "EUROPHTA" => "EUROPHTA",
    // Expanscience
    "EXPANSCIENCE" => "EXPANSCIENCE",
    // Piramal
    "PIRAMAL CRITICAL CARE" => "PIRAMAL",
    "PIRAMAL" => "PIRAMAL",
    // Swedish Orphan Biovitrum
    "SWEDISH ORPHAN BIOVITRUM INTERNATIONAL" => "SWEDISH ORPHAN BIOVITRUM",
    "SWEDISH ORPHAN BIOVITRUM" => "SWEDISH ORPHAN BIOVITRUM",
    // Uni-Pharma
    "UNI-PHARMA KLEON TSETIS PHARMACEUTICAL LABORATORIES" => "UNI-PHARMA",
    "UNI-PHARMA" => "UNI-PHARMA",
    // Upjohn
    "UPJOHN" => "UPJOHN",
    // Biogen
    "BIOGEN NETHERLANDS" => "BIOGEN",
    "BIOGEN" => "BIOGEN",
    // Essential
    "ESSENTIAL" => "ESSENTIAL",
    // Ever Valinject
    "EVER VALINJECT" => "EVER VALINJECT",
    // G.L.
    "G.L." => "G.L.",
    // Nten
    "NTEN OY" => "NTEN OY",
    // Vantive
    "VANTIVE" => "VANTIVE",
    "VANTIVE BELGIUM" => "VANTIVE",
    // Bluefish
    "BLUEFISH PHARMACEUTICALS" => "BLUEFISH",
    "BLUEFISH" => "BLUEFISH",
    // Ethyx
    "ETHYX PHARMACEUTICALS" => "ETHYX",
    "ETHYX" => "ETHYX",
    // SERB
    "SERB" => "SERB",
    // Sifi
    "SIFI" => "SIFI",
    // Therabel
    "THERABEL LUCIEN" => "THERABEL",
    "THERABEL" => "THERABEL",
    // Arko
    "ARKO" => "ARKO",
    // Esteve
    "ESTEVE PHARMACEUTICALS" => "ESTEVE",
    "ESTEVE" => "ESTEVE",
    // Perrigo
    "LABORATOIRE PERRIGO" => "PERRIGO",
    "PERRIGO" => "PERRIGO",
    // Rovi
    "LABORATORIOS FARMACEUTICOS ROVI" => "ROVI",
    "ROVI" => "ROVI",
    // Qilu
    "QILU PHARMA SPAIN" => "QILU",
    "QILU" => "QILU",
    // Frilab
    "FRILAB" => "FRILAB",
    // Sol
    "SOL" => "SOL",
    // Alcon
    "ALCON" => "ALCON",
    // And
    "AND" => "AND",
    // Efisciens
    "EFISCIENS" => "EFISCIENS",
    // Falk
    "FALK" => "FALK",
    // Infectopharm
    "INFECTOPHARM ARZNEIMITTEL UND CONSILIUM" => "INFECTOPHARM",
    "INFECTOPHARM" => "INFECTOPHARM",
    // Juvise
    "JUVISE PHARMACEUTICALS" => "JUVISE",
    "JUVISE" => "JUVISE",
    // Lipomed
    "LIPOMED" => "LIPOMED",
    // Noridem
    "NORIDEM" => "NORIDEM",
    "NORIDEM ENTREPRISES LIMITED" => "NORIDEM",
    // Richard
    "RICHARD" => "RICHARD",
    // Tillotts
    "TILLOTTS" => "TILLOTTS",
    "TILLOTTS PHARMA" => "TILLOTTS",
    // Alexion
    "ALEXION EUROPE" => "ALEXION",
    "ALEXION" => "ALEXION",
    // ALK
    "ALK ABELLO" => "ALK",
    "ALK" => "ALK",
    // Angelini
    "ANGELINI" => "ANGELINI",
    // Camurus
    "CAMURUS" => "CAMURUS",
    // Fidia
    "FIDIA" => "FIDIA",
    "FIDIA FARMACEUTICI S.P.A." => "FIDIA",
    // Incyte
    "INCYTE BIOSCIENCES DISTRIBUTION" => "INCYTE",
    "INCYTE" => "INCYTE",
    // Grifols
    "INSTITUTO GRIFOLS" => "GRIFOLS",
    "GRIFOLS" => "GRIFOLS",
    "GRIFOLS DEUTSCHLAND" => "GRIFOLS",
    // CCD
    "LABORATOIRE CCD" => "CCD",
    "CCD" => "CCD",
    // Medisol
    "MEDISOL" => "MEDISOL",
    // Provepharm
    "PROVEPHARM" => "PROVEPHARM",
    // T & A
    "T & A" => "T & A",
    // Venipharm
    "VENIPHARM" => "VENIPHARM",
    // Vivanta
    "VIVANTA GENERICS" => "VIVANTA",
    "VIVANTA" => "VIVANTA",
    // Alliance Healthcare
    "ALLIANCE" => "ALLIANCE",
    // Biomarin
    "BIOMARIN INTERNATIONAL LIMITED" => "BIOMARIN",
    "BIOMARIN" => "BIOMARIN",
    // Chemineau
    "CHEMINEAU" => "CHEMINEAU",
    // CNX Therapeutics
    "CNX THERAPEUTICS" => "CNX THERAPEUTICS",
    "CNX THERAPEUTICS IRELAND LIMITED" => "CNX THERAPEUTICS",
    // DB
    "DB" => "DB",
    // Developpement
    "DEVELOPPEMENT" => "DEVELOPPEMENT",
    // Dipharma
    "DIPHARMA ARZNEIMITTEL" => "DIPHARMA",
    "DIPHARMA" => "DIPHARMA",
    // Ferrer
    "FERRER INTERNACIONAL" => "FERRER",
    "FERRER" => "FERRER",
    // Immedica
    "IMMEDICA" => "IMMEDICA",
    // Jolly Jatel
    "JOLLY JATEL" => "JOLLY JATEL",
    "JOLLY" => "JOLLY JATEL",
    // Kreussler
    "KREUSSLER & CO ALLEMAGNE" => "KREUSSLER",
    "KREUSSLER" => "KREUSSLER",
    "CHEMISCHE FABRIK KREUSSLER" => "KREUSSLER",
    // Merus
    "MERUS LABS LUXCO II" => "MERUS",
    "MERUS LABS" => "MERUS",
    "MERUS" => "MERUS",
    // Mitem
    "MITEM" => "MITEM",
    // Molteni
    "MOLTENI & ALITTI" => "MOLTENI",
    "MOLTENI" => "MOLTENI",
    // Ndoz
    "NDOZ PHARMACEUTICALS" => "NDOZ",
    "NDOZ" => "NDOZ",
    // Orpha Devel
    "ORPHA DEVEL HANDELS & VERTRIEBS" => "ORPHA",
    "ORPHA" => "ORPHA",
    // Orphelia
    "ORPHELIA" => "ORPHELIA",
    // Orphan Europe
    "ORPHAN EUROPE" => "ORPHAN EUROPE",
    // Adienne
    "ADIENNE" => "ADIENNE",
    // Blueprint
    "BLUEPRINT MEDICINES" => "BLUEPRINT",
    "BLUEPRINT" => "BLUEPRINT",
    // PCA
    "CIE CENTRALE DES ARMEES- PCA" => "PCA",
    "PCA" => "PCA",
    // Double-E
    "DOUBLE-E" => "DOUBLE-E",
    // Sit
    "FARMACEUTICO SIT" => "SIT",
    "SIT" => "SIT",
    // Kowa
    "KOWA PHARMACEUTICAL EUROPE" => "KOWA",
    "KOWA" => "KOWA",
    // Merz
    "MERZ THERAPEUTICS" => "MERZ",
    "MERZ" => "MERZ",
    // Messer
    "MESSER" => "MESSER",
    // Pierrel
    "PIERREL" => "PIERREL",
    // PTC
    "PTC THERAPEUTICS INTERNATIONAL" => "PTC",
    "PTC" => "PTC",
    // Techni
    "TECHNI-" => "TECHNI",
    "TECHNI" => "TECHNI",
    // Theravia
    "THERAVIA" => "THERAVIA",
    // Bausch Health
    "BAUSCH HEALTH IRELAND" => "BAUSCH HEALTH",
    "BAUSCH HEALTH" => "BAUSCH HEALTH",
    // Bavarian Nordic
    "BAVARIAN NORDIC" => "BAVARIAN NORDIC",
    // Benta
    "BENTA" => "BENTA",
    "BENTA LYON" => "BENTA",
    // Bionorica
    "BIONORICA" => "BIONORICA",
    // Brothier
    "BROTHIER" => "BROTHIER",
    // Dentsply Sirona
    "DENTSPLY SIRONA" => "DENTSPLY SIRONA",
    // Egis
    "EGIS PHARMACEUTICALS" => "EGIS",
    "EGIS" => "EGIS",
    // Exod
    "EXOD" => "EXOD",
    // Gen.Orph
    "GEN.ORPH" => "GEN.ORPH",
    // Glenwood
    "GLENWOOD GMBH PHARMAZEUTISCHE ERZEUGNISSE" => "GLENWOOD",
    "GLENWOOD" => "GLENWOOD",
    // Gomenol
    "GOMENOL" => "GOMENOL",
    // Indivior
    "INDIVIOR EUROPE" => "INDIVIOR",
    "INDIVIOR" => "INDIVIOR",
    // Iphym
    "IPHYM" => "IPHYM",
    // Italfarmaco
    "ITALFARMACO" => "ITALFARMACO",
    // Laboratoires Vicks
    "LABORATOIRE VICKS" => "VICKS",
    "VICKS" => "VICKS",
    // Legras
    "LEGRAS" => "LEGRAS",
    // Liberty
    "LIBERTY" => "LIBERTY",
    // Linde
    "LINDE" => "LINDE",
    "LINDE HEALTHCARE" => "LINDE",
    // Medice
    "MEDICE ARZNEIMITTEL PUTTER" => "MEDICE",
    "MEDICE" => "MEDICE",
    // Pierre Rolland
    "PRODUITS DENTAIRES PIERRE ROLLAND" => "PIERRE ROLLAND",
    "PIERRE ROLLAND" => "PIERRE ROLLAND",
    // Techdow
    "TECHDOW PHARMA NETHERLANDS" => "TECHDOW",
    "TECHDOW" => "TECHDOW",
    // Vifor
    "VIFOR FRESENIUS MEDICAL CARE RENAL" => "VIFOR FRESENIUS",
    "VIFOR" => "VIFOR",
    "VIFOR FRESENIUS" => "VIFOR FRESENIUS",
    // Waymade
    "WAYMADE" => "WAYMADE",
    // Willmar Schwabe
    "WILLMAR SCHWABE" => "WILLMAR SCHWABE",
    // Alnylam
    "ALNYLAM NETHERLANDS" => "ALNYLAM",
    "ALNYLAM" => "ALNYLAM",
    // Becton Dickinson
    "BECTON DICKINSON" => "BECTON DICKINSON",
    // Heel
    "BIOLOGISCHE HEILMITTEL HEEL" => "HEEL",
    "HEEL" => "HEEL",
    // Clinigen
    "CLINIGEN HEALTHCARE" => "CLINIGEN",
    "CLINIGEN" => "CLINIGEN",
    // Deciphera
    "DECIPHERA PHARMACEUTICALS" => "DECIPHERA",
    "DECIPHERA" => "DECIPHERA",
    // Desitin
    "DESITIN ARZNEIMITTEL" => "DESITIN",
    "DESITIN" => "DESITIN",
    // Domes
    "DOMES" => "DOMES",
    // Fertin
    "FERTIN" => "FERTIN",
    // Jazz
    "JAZZ PHARMACEUTICALS IRELAND LIMITED" => "JAZZ",
    "JAZZ" => "JAZZ",
    // Kyowa Kirin
    "KYOWA KIRIN HOLDINGS" => "KYOWA KIRIN",
    "KYOWA KIRIN" => "KYOWA KIRIN",
    // Laboratoires Bailly Creat
    "LABORATOIRE BAILLY CREAT" => "BAILLY CREAT",
    "BAILLY CREAT" => "BAILLY CREAT",
    // Laboratoires de l'Homme de Fer
    "LABORATOIRE DE L'HOMME DE FER" => "HOMME DE FER",
    "HOMME DE FER" => "HOMME DE FER",
    // Leurquin
    "LEURQUIN MEDIOLANUM" => "LEURQUIN",
    "LEURQUIN" => "LEURQUIN",
    // LG Homeo
    "LG HOMEO" => "LG HOMEO",
    // Meda
    "MEDA" => "MEDA",
    // Medgen
    "MEDGEN" => "MEDGEN",
    // Medipha
    "MEDIPHA" => "MEDIPHA",
    // Melisana
    "MELISANA" => "MELISANA",
    // Moderna
    "MODERNA BIOTECH SPAIN SL" => "MODERNA",
    "MODERNA" => "MODERNA",
    // Omedicamed
    "OMEDICAMED UNIPESSOAL" => "OMEDICAMED",
    "OMEDICAMED" => "OMEDICAMED",
    // Pharmholding
    "PHARMHOLDING" => "PHARMHOLDING",
    // Sciencex
    "SCIENCEX" => "SCIENCEX",
    // SGP
    "SGP" => "SGP",
    // STD
    "STD PHARMACEUTICAL" => "STD",
    "STD" => "STD",
    // Theradial
    "THERADIAL" => "THERADIAL",
    // Theranol
    "THERANOL DEGLAUDE" => "THERANOL",
    "THERANOL" => "THERANOL",
    // AAA
    "ADVANCED ACCELERATOR APPLICATIONS" => "AAA",
    "AAA" => "AAA",
    "ADVANCED ACCELERATOR APPLICATIONS MOLECULAR IMAGING" => "AAA",
    // AGB
    "AGB-" => "AGB",
    "AGB" => "AGB",
    // Alkalon
    "ALKALON" => "ALKALON",
    // Amicus
    "AMICUS THERAPEUTICS EUROPE" => "AMICUS",
    "AMICUS" => "AMICUS",
    // Argenx
    "ARGENX" => "ARGENX",
    // Arkomedica
    "ARKOMEDICA" => "ARKOMEDICA",
    // Ascendis
    "ASCENDIS PHARMA BONE DISEASES" => "ASCENDIS",
    "ASCENDIS" => "ASCENDIS",
    // B.E. Imaging
    "B.E.IMAGING" => "B.E.IMAGING",
    // Basi
    "BASI" => "BASI",
    // Bioluz
    "BIOLUZ" => "BIOLUZ",
    // Biose
    "BIOSE INDUSTRIE" => "BIOSE",
    "BIOSE" => "BIOSE",
    // Biotest
    "BIOTEST" => "BIOTEST",
    // Chemi
    "CHEMI" => "CHEMI",
    // Cilfa
    "CILFA DEVELOPPEMENT" => "CILFA",
    "CILFA" => "CILFA",
    // Colgate
    "COLGATE PALMOLIVE" => "COLGATE",
    "COLGATE" => "COLGATE",
    // Doliage
    "DOLIAGE DEVELOPPEMENT" => "DOLIAGE",
    "DOLIAGE" => "DOLIAGE",
    // Endo
    "ENDO OPERATIONS" => "ENDO",
    "ENDO" => "ENDO",
    // Eurocept
    "EUROCEPT INTERNATIONAL" => "EUROCEPT",
    "EUROCEPT" => "EUROCEPT",
    // Ever Neuro
    "EVER NEURO" => "EVER NEURO",
    // Exelgyn
    "EXELGYN" => "EXELGYN",
    // Genzyme
    "GENZYME EUROPE" => "GENZYME",
    "GENZYME" => "GENZYME",
    // Helsinn
    "HELSINN BIREX PHARMACEUTICALS" => "HELSINN",
    "HELSINN" => "HELSINN",
    // Iprad
    "IPRAD" => "IPRAD",
    // La Colina
    "LA COLINA" => "LA COLINA",
    "LA COLINA COMERCIO FARMACEUTICO" => "LA COLINA",
    // Laboratorios Inib
    "LABORATORIOS INIB" => "INIB",
    "INIB" => "INIB",
    // Madrigal
    "MADRIGAL PHARMACEUTICALS EU" => "MADRIGAL",
    "MADRIGAL" => "MADRIGAL",
    // Mediam
    "MEDIAM" => "MEDIAM",
    // Mendelikabs
    "MENDELIKABS EUROPE" => "MENDELIKABS",
    "MENDELIKABS" => "MENDELIKABS",
    // Mundipharma
    "MUNDIPHARMA CORPORATION" => "MUNDIPHARMA",
    "MUNDIPHARMA COPORATION" => "MUNDIPHARMA",
    "MUNDIPHARMA" => "MUNDIPHARMA",
    // Molnlycke
    "MÖLNLYCKE HEALTH CARE" => "MÖLNLYCKE",
    "MÖLNLYCKE" => "MÖLNLYCKE",
    // Nikkiso
    "NIKKISO BELGIUM" => "NIKKISO",
    "NIKKISO" => "NIKKISO",
    // Paion
    "PAION" => "PAION",
    // Rad Neurim
    "RAD NEURIM PHARMACEUTICALS EEC" => "RAD NEURIM",
    "RAD NEURIM" => "RAD NEURIM",
    // Rotop
    "ROTOP PHARMAKA" => "ROTOP",
    "ROTOP RADIOPHARMACY" => "ROTOP",
    "ROTOP" => "ROTOP",
    // RottaPharm
    "ROTTAPHARM" => "ROTTAPHARM",
    // SCA
    "SCA" => "SCA",
    // Siemens
    "SIEMENS HEALTHCARE" => "SIEMENS",
    "SIEMENS" => "SIEMENS",
    // Stemline
    "STEMLINE THERAPEUTICS" => "STEMLINE",
    "STEMLINE" => "STEMLINE",
    // TAW
    "TAW" => "TAW",
    // TRB
    "TRB CHEMEDICA" => "TRB",
    "TRB" => "TRB",
    // Valneva
    "VALNEVA AUSTRIA" => "VALNEVA",
    "VALNEVA SWEDEN" => "VALNEVA",
    "VALNEVA" => "VALNEVA",
};

/// Canonicalize a lab name to its family group.
/// Falls back to suffix-stripped name if no exact match.
/// Strips parenthetical country suffixes like "(PAYS-BAS)", "(ESPAGNE)".
pub fn canonicalize_lab(lab: &str) -> String {
    let mut normalized = lab.trim().to_uppercase();
    // Strip parenthetical country/location: "TEVA (PAYS-BAS)" → "TEVA"
    if let Some(pos) = normalized.find(" (") {
        normalized = normalized[..pos].trim().to_string();
    }
    LAB_FAMILY_MAP.get(&normalized)
        .map(|s| s.to_string())
        .unwrap_or_else(|| strip_lab_suffix(&normalized))
}

/// Strip common legal suffixes from lab name to get base name.
/// Iterates until no more suffixes match (handles "PHARMA GMBH" → "SOME").
fn strip_lab_suffix(lab: &str) -> String {
    // Suffixes that may appear at either start or end of name
    // (descriptive words that won't cause false positives)
    let both = ["LABORATOIRES", "PHARMA", "GMBH", "FRANCE", "HEALTH", "SANTE"];
    // Legal entity suffixes — only strip from the END, NOT from start.
    // "SA" as prefix would corrupt "SANOFI" → "NOFI".
    let only_suffix = ["SA", "SAS", "SARL", "LTD"];
    let mut result = lab.to_string();
    loop {
        let before = result.clone();
        for suffix in &both {
            if let Some(pos) = result.strip_prefix(suffix) {
                result = pos.trim().to_string();
            }
            if let Some(pos) = result.strip_suffix(suffix) {
                result = pos.trim().to_string();
            }
        }
        for suffix in &only_suffix {
            if let Some(pos) = result.strip_suffix(suffix) {
                result = pos.trim().to_string();
            }
        }
        if result == before {
            break;
        }
    }
    if result.is_empty() { lab.to_string() } else { result }
}

// ---------------------------------------------------------------------------
// Salt Prefix/Suffix Stripping (Task 1)
// ---------------------------------------------------------------------------

/// Salt prefixes — matched longest-first. Used to strip chemical form
/// from active ingredient names (e.g., "chlorhydrate de paracétamol" → "paracétamol").
pub static SALT_PREFIXES: &[&str] = &[
    // Longest compound prefixes first
    "dichlorhydrate monohydrate de", "dichlorhydrate monohydratée de",
    "chlorhydrate monohydrate de", "chlorhydrate monohydratée de",
    "chlorhydrate anhydre de",
    "dichlorhydrate de", "dichlorhydrate d'",
    "chlorhydrate de", "chlorhydrate d'",
    "hémifumarate de",
    "hydrogénosulfate de",
    "hydrogénotartrate de",
    "hydrogentartrate de",
    "métasulfobenzoate sodique de",
    "metasulfobenzoate sodique de",
    "sulfate de", "sulfate d'",
    "maléate de", "maléate d'", "maleate de", "maleate d'",
    "malate de", "malate d'",
    "fumarate de", "fumarate d'",
    "bromhydrate de", "bromhydrate d'",
    "tartrate de", "tartrate d'",
    "ditrartrate de",
    "ditartrate de",
    "glycolate de",
    "nicotinate de",
    "phosphate de", "phosphate d'",
    "acétate de", "acétate d'", "acetate de", "acetate d'",
    "carbonate de", "carbonate d'",
    "bicarbonate de",
    "borate de",
    "citrate de", "citrate d'",
    "glucoheptonate de",
    "gluconate de", "gluconate d'",
    "lactate de", "lactate d'",
    "camphosulfonate de",
    "bésylate de", "bésylate d'", "besylate de", "besylate d'",
    "bésilate de", "bésilate d'", "besilate de", "besilate d'",
    "mésylate de", "mésylate d'", "mesylate de", "mesylate d'",
    "mésilate de", "mésilate d'", "mesilate de", "mesilate d'",
    "dimésylate de", "dimésilate de", "dimésylate d'", "dimésilate d'",
    "ésilate de", "ésilate d'", "esilate de", "esilate d'",
    "pamoate de", "embonate de",
    "succinate de",
    "oxalate de", "oxalate d'",
    "propionate de", "propionate d'", "dipropionate de",
    "clavulanate de",
    "alendronate de",
    "sel de", "sel d'",
    "iodure de",
    "bromure de", "bromure d'", "butylbromure de",
    "chlorure de", "chlorure d'",
    "nitrate de", "nitrate d'", "dinitrate de", "mononitrate de",
    "ranélate de",
    // Additional salt forms from audit
    "xinafoate de",
    "furoate de",
    "digluconate de",
    "fluorure de", "fluorure d'",
    "résinate de", "resinate de",
    "palmitate de",
    "folinate de", "lévofolinate de",
    "cromoglicate de",
    "valproate de", "divalproate de",
    "tosylate de", "tosilate de",
    "benzoate de", "benzoate d'",
    "valérate de", "valérate d'", "valerate de", "valerate d'",
    "ascorbate de",
    "hémisuccinate de", "hemisuccinate de",
    "hydrogénosuccinate de",
    "acéponate de", "aceponate de",
    "acétylsalicylate de",
    "amidotrizoate de",
    "antimoniate de",
    "bitartrate de",
    "camsylate de",
    "canrénoate de", "canrenoate de",
    "caproate de",
    "clodronate de",
    "diaspartate de",
    "diphosphate de",
    "dobésilate de", "dobesilate de",
    "énanthate de", "énantate de", "enanthe de", "enantate de",
    "fusidate de",
    "gadobénate de", "gadobenate de",
    "gadotérate de", "gadoterate de",
    "gluconolactate de",
    "glycérophosphate de", "glycerophosphate de",
    "hémicitrate de",
    "hémintertrate de",
    "hydroxy-4-butyrate de",
    "laurilsulfate de",
    "lysinate de",
    "oxoglurate de",
    "oxybate de",
    "pamidronate de",
    "pentétate de", "pentetate de",
    "picosulfate de",
    "pivalate de",
    "salicylate de",
    "sous-citrate de",
    "thiosulfate de",
    "trifluoroacétate de",
    "trifénapate de",
    "triméthylacétate de",
    "trinitrate de",
    "trisilicate de",
    "undécanoate de",
    "d'",  // strips leading "d'" from remaining substance names after prefix removal
    "aspartate d'", "aspartate de",
];

/// Salt suffixes — matched iteratively until no change (multi-pass).
/// E.g., "paracetamol chlorhydrate monohydrate" → "paracetamol"
pub static SALT_SUFFIXES: &[&str] = &[
    // Compound forms (multi-word, longest first)
    "chlorhydrate monohydrate", "chlorhydrate monohydratée",
    "chlorhydrate anhydre",
    "dichlorhydrate monohydrate", "dichlorhydrate monohydratée",
    "dichlorhydrate",
    "chlorhydrate",
    "sulfate anhydre", "sulfate",
    "malate", "bromhydrate",
    "tartrate", "glycolate",
    "base anhydre", "base",
    "sel sodique", "sel",
    "chlorhydrate de sodium",
    "sel de sodium",
    "monosodique trihydrate", "monosodique trihydratée",
    "monosodique",
    "sesquihydratée", "sesquihydrate",
    "métasulfobenzoate sodique",
    "metasulfobenzoate sodique",
    // Hydrate forms
    "dihydrate", "dihydratée",
    "trihydrate", "trihydratée",
    "monohydrate", "monohydratée",
    "hémipentahydraté", "hemipentahydrate", "hémiptentahydraté",
    "pentahydrate", "pentahydratée",
    "hexahydraté", "hexahydrate",
    "heptahydraté", "heptahydrate",
    "anhydre",
    // Compound salt forms
    "sodique sesquihydraté", "sodique sesquihydrate",
    "disodique hémipentahydraté", "disodique hemipentahydrate",
    // disodique is prefix-only (e.g., "disodique acetylsalicylate");
    // not a suffix — removing prevents stripping "d'arginine" → "d"
    "hémifumarate", "hemifumarate",
    "hydrogénosulfate", "hydrogensulfate",
    "hydrogénotartrate", "hydrogentartrate",
    // Frequently-missed forms from audit
    "sodique",
    "calcique",
    "potassique",
    "mésylate", "mesylate", "mésilate", "mesilate", "dimésylate", "dimésilate",
    "fumarate",
    "succinate",
    "camphosulfonate",
    "pamoate", "embonate",
    "ésilate", "esilate",
    "bésylate", "besylate", "bésilate", "besilate",
    "xinafoate",
    "furoate",
    "digluconate",
    "dipropionate",
    "résinate", "resinate",
    "palmitate",
    "folinate",
    "cromoglicate",
    "valproate",
    "tosylate", "tosilate",
    "benzoate",
    "valérate", "valerate",
    "nitrate", "dinitrate", "mononitrate", "trinitrate",
    "bromure",
    "fluorure",
    "cilexétil", "cilexetil",
    "aspartate d'", "aspartate de",
    "tert-butylamine",
];

/// Strip salt prefixes and suffixes from active ingredient names.
/// Multi-pass for suffixes: strips one suffix, then re-checks until no change.
/// Uses diacritics-stripped comparison so accented inputs match ASCII suffix keys.
pub fn strip_salt(s: &str) -> String {
    let s = s.trim();
    // Strip prefix (check start of string, longest-first — first match wins)
    let mut result = s.to_string();
    for prefix in SALT_PREFIXES.iter() {
        let prefix_norm = strip_diacritics(&prefix.to_uppercase());
        let result_norm = strip_diacritics(&result.to_uppercase());
        if result_norm.starts_with(&prefix_norm) {
            result = result[prefix.len()..].trim().to_string();
            break;
        }
    }
    // Multi-pass suffix stripping (iterate until no change)
    loop {
        let before = result.clone();
        for suffix in SALT_SUFFIXES.iter() {
            let suffix_norm = strip_diacritics(&suffix.to_uppercase());
            let result_norm = strip_diacritics(&result.to_uppercase());
            if result_norm.len() < suffix_norm.len() {
                continue;
            }
            if result_norm.ends_with(&suffix_norm) {
                let end_pos = result_norm.len() - suffix_norm.len();
                result = result[..end_pos].trim().to_string();
            }
        }
        if result == before { break; }
    }
    result
}

/// Strip parenthetical salt annotations mid-string.
/// Handles nested parens by matching outermost balanced parens
/// (e.g., "(SERENOA REPENS (W.BARTRAM) SMALL.)" stripped entirely).
pub fn strip_parens(s: &str) -> String {
    use regex_lite::Regex;
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        // Balanced-parentheses regex: matches (...), including nested (...)
        Regex::new(r"\s*\([^()]*(?:\([^()]*\)[^()]*)*\)").unwrap()
    });
    RE.replace_all(s, "").trim().to_string()
}

/// Noise words stripped from FTS index for cleaner search.
pub static NOISE_WORDS: phf::Set<&'static str> = phf_set! {
    "de", "du", "la", "le", "les", "et", "ou", "en", "un", "une",
    "des", "aux", "au", "a", "l", "d",
};

/// Generate an FTS-searchable version of a drug name.
/// Strips diacritics, normalizes spaces, removes noise words.
pub fn fts_normalize(s: &str) -> String {
    strip_diacritics(&s.to_lowercase())
        .split_whitespace()
        .filter(|w| !NOISE_WORDS.contains(w))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_lab_space() {
        assert_eq!(strip_field(" SANOFI"), "SANOFI");
        assert_eq!(strip_field("SANOFI "), "SANOFI");
    }

    #[test]
    fn test_generic_type_0() {
        assert_eq!(normalize_generic_type("0"), "reference");
    }

    #[test]
    fn test_generic_type_2() {
        assert_eq!(normalize_generic_type("2"), "cross-group");
    }

    #[test]
    fn test_generic_type_4() {
        assert_eq!(normalize_generic_type("4"), "sustained-release");
    }

    #[test]
    fn test_double_space() {
        assert_eq!(normalize_spaces("PARACETAMOL  1000  mg"), "PARACETAMOL 1000 mg");
    }

    #[test]
    fn test_strip_cip_ean_13_digit_non_34009() {
        // 13-digit EAN not starting with 34009 — kept as-is
        assert_eq!(strip_cip_ean("1234567890123"), "1234567890123");
    }

    #[test]
    fn test_strip_cip_ean_7_digit() {
        // 7-digit CIP kept as-is
        assert_eq!(strip_cip_ean("3000001"), "3000001");
    }

    #[test]
    fn test_strip_cip_ean_empty() {
        assert_eq!(strip_cip_ean(""), "");
    }

    // --- strip_diacritics ---

    #[test]
    fn test_strip_diacritics() {
        assert_eq!(strip_diacritics("Doliprane"), "Doliprane");
        assert_eq!(strip_diacritics("Doliprane"), "Doliprane");
        assert_eq!(strip_diacritics("café"), "cafe");
        assert_eq!(strip_diacritics("Doliprane"), "Doliprane");
        assert_eq!(strip_diacritics(""), "");
    }

    // --- validate_ean13 ---

    #[test]
    fn test_validate_ean13() {
        // Valid EAN-13 (known valid from gtin-validate test suite)
        assert!(validate_ean13("1498279802125"));
        // Invalid checksum (last digit wrong: 4 instead of 5)
        assert!(!validate_ean13("1498279802124"));
        // Wrong length — 12 digits
        assert!(!validate_ean13("340093000001"));
        // Wrong length — 14 digits
        assert!(!validate_ean13("34009300000123"));
        // Empty string
        assert!(!validate_ean13(""));
        // Non-numeric
        assert!(!validate_ean13("340093000001a"));
    }

    // --- canonicalize_form ---

    #[test]
    fn test_canonicalize_form() {
        assert_eq!(canonicalize_form("comprime"), "CPR");
        assert_eq!(canonicalize_form("comprime pellicule"), "CPR");
        assert_eq!(canonicalize_form("Gelule"), "GEL");
        assert_eq!(canonicalize_form("capsule molle"), "GEL");
        assert_eq!(canonicalize_form("solution injectable"), "INJ");
        assert_eq!(canonicalize_form("collyre"), "COL");
        assert_eq!(canonicalize_form("Unknown form"), "unknown form"); // falls through
        assert_eq!(canonicalize_form(""), "");
    }

    #[test]
    fn test_canonicalize_form_accented() {
        // Accented forms must be canonicalized via diacritics stripping
        assert_eq!(canonicalize_form("comprimé"), "CPR");
        assert_eq!(canonicalize_form("Comprimé Pelliculé"), "CPR");
        assert_eq!(canonicalize_form("gélule"), "GEL");
        assert_eq!(canonicalize_form("crème"), "POM");
        assert_eq!(canonicalize_form("solution pour perfusion"), "INJ");
        assert_eq!(canonicalize_form("comprimé sécable"), "CPR");
    }

    #[test]
    fn test_canonicalize_form_extended() {
        // New codes from the extended FORM_CANONICAL map
        assert_eq!(canonicalize_form("gaz"), "GAZ");
        assert_eq!(canonicalize_form("shampooing"), "SHAMPOO");
        assert_eq!(canonicalize_form("ovule"), "SUP");
        assert_eq!(canonicalize_form("implant"), "IMPLANT");
        assert_eq!(canonicalize_form("collutoire"), "COLLUTOIRE");
        assert_eq!(canonicalize_form("lyophilisat"), "LYO");
        assert_eq!(canonicalize_form("pastille a sucer"), "PAST");
        assert_eq!(canonicalize_form("comprime a sucer ou a croquer"), "CPR_SUCK");
        assert_eq!(canonicalize_form("poudre et solvant pour solution a diluer pour perfusion"), "INJ_POW");
        assert_eq!(canonicalize_form("solution pour dialyse peritoneale"), "DIALYSE");
        assert_eq!(canonicalize_form("plante(s) pour tisane"), "TISANE");
        assert_eq!(canonicalize_form("generateur radiopharmaceutique"), "RADIO");
        assert_eq!(canonicalize_form("systeme de diffusion"), "DEVICE");
        // Truncated BDPM forms
        assert_eq!(canonicalize_form("comprime(s)"), "CPR");
    }

    // --- canonicalize_route ---

    #[test]
    fn test_canonicalize_route() {
        assert_eq!(canonicalize_route("Voie orale"), "orale");
        assert_eq!(canonicalize_route("usage oral"), "orale");
        assert_eq!(canonicalize_route("usage cutane"), "cutanee");
        assert_eq!(canonicalize_route("dermique"), "cutanee");
        assert_eq!(canonicalize_route("voie rectale"), "rectale");
        assert_eq!(canonicalize_route("usage ophtalmique"), "oculaire");
        assert_eq!(canonicalize_route("dispositif transdermique"), "transdermique");
        assert_eq!(canonicalize_route("unknown route"), "unknown route"); // falls through
        assert_eq!(canonicalize_route(""), "");
    }

    #[test]
    fn test_canonicalize_route_extended() {
        // Semicolon-separated multi-value routes
        assert_eq!(canonicalize_route("intramusculaire;intraveineuse"), "intramusculaire;intraveineuse");
        // Accented variants stripped via diacritics
        assert_eq!(canonicalize_route("intraveineuse"), "intraveineuse");
        assert_eq!(canonicalize_route("intraveineux"), "intraveineuse");
        assert_eq!(canonicalize_route("voie sous-cutanée"), "sous-cutanee");
        assert_eq!(canonicalize_route("ophtalmique"), "oculaire");
        assert_eq!(canonicalize_route("inhalée"), "inhalation");
        assert_eq!(canonicalize_route("inhalé"), "inhalation");
        assert_eq!(canonicalize_route("intramusculaire"), "intramusculaire");
        assert_eq!(canonicalize_route("nasale"), "nasale");
        assert_eq!(canonicalize_route("buccale"), "buccale");
    }

    // --- canonicalize_lab ---

    #[test]
    fn test_canonicalize_lab() {
        assert_eq!(canonicalize_lab("VIATRIS SOLUTE INJ"), "VIATRIS");
        assert_eq!(canonicalize_lab("VIATRIS FRANCE"), "VIATRIS");
        assert_eq!(canonicalize_lab("MYLAN PHARMA"), "MYLAN");
        assert_eq!(canonicalize_lab("ACTAVIS GROUP"), "MYLAN");
        assert_eq!(canonicalize_lab("BIOGARAN"), "BIOGARAN");
        assert_eq!(canonicalize_lab("ARROW GENERIQUES"), "ARROW");
        assert_eq!(canonicalize_lab("TEVA HEALTH"), "TEVA");
        assert_eq!(canonicalize_lab("SANDOZ GMBH"), "SANDOZ");
        assert_eq!(canonicalize_lab("GENERIC LABS SAS"), "GENERIC LABS");
        assert_eq!(canonicalize_lab("SOME PHARMA GMBH"), "SOME");
        assert_eq!(canonicalize_lab("NOFI WINTHROP INDUSTRIE"), "SANOFI");
        assert_eq!(canonicalize_lab("PFIZER HOLDING"), "PFIZER");
        assert_eq!(canonicalize_lab("OPELLA HEALTHCARE"), "SANOFI");
        assert_eq!(canonicalize_lab("UP"), "UPSA");
        // SA-prefix bug regression test: "SA" must not be stripped as prefix
        assert_eq!(canonicalize_lab("SANOFI WINTHROP INDUSTRIE"), "SANOFI");
        assert_eq!(canonicalize_lab("SANOFI WINTHROP"), "SANOFI");
    }

    // --- strip_salt ---

    #[test]
    fn test_strip_salt() {
        assert_eq!(strip_salt("chlorhydrate de paracétamol"), "paracétamol");
        assert_eq!(strip_salt("paracetamol sel de sodium"), "paracetamol");
        assert_eq!(strip_salt("sulfate de morphine"), "morphine");
        assert_eq!(strip_salt("aspirine 500mg"), "aspirine 500mg"); // no salt
        assert_eq!(strip_salt("paracetamol chlorhydrate monohydrate"), "paracetamol");
        assert_eq!(strip_salt(""), "");
        assert_eq!(strip_salt("chlorhydrate de sodium"), "sodium");
    }

    #[test]
    fn test_strip_salt_sodique() {
        assert_eq!(strip_salt("diclofenac sodique"), "diclofenac");
    }
    #[test]
    fn test_strip_salt_calcique() {
        assert_eq!(strip_salt("atorvastatine calcique"), "atorvastatine");
    }
    #[test]
    fn test_strip_salt_trihydratee() {
        assert_eq!(strip_salt("amoxicilline trihydratée"), "amoxicilline");
    }
    #[test]
    fn test_strip_salt_potassique() {
        assert_eq!(strip_salt("cloxacilline potassique"), "cloxacilline");
    }

    // --- strip_parens ---

    #[test]
    fn test_strip_parens() {
        assert_eq!(strip_parens("Paracetamol (chlorhydrate)"), "Paracetamol");
        assert_eq!(strip_parens("Aspirin"), "Aspirin");
        assert_eq!(strip_parens("Drug (extra info here)"), "Drug");
        assert_eq!(strip_parens(""), "");
    }

    #[test]
    fn test_strip_parens_nested() {
        // Nested parens regression: (SERENOA REPENS (W.BARTRAM) SMALL.)
        // is fully stripped, not left as "PALMIER DE FLORIDE SMALL.)"
        assert_eq!(strip_parens("PALMIER DE FLORIDE (SERENOA REPENS (W.BARTRAM) SMALL.)"), "PALMIER DE FLORIDE");
        // Strip balanced parens only
        assert_eq!(strip_parens("BÉTAMETHASONE (PHOSPHATE DE) ET DISODIUM"), "BÉTAMETHASONE ET DISODIUM");
    }

    #[test]
    fn test_strip_salt_new_prefixes() {
        // New prefix forms: fumarate, maleate, oxalate, propionate, etc.
        assert_eq!(strip_salt("fumarate de bisoprolol"), "bisoprolol");
        assert_eq!(strip_salt("maléate de timolol"), "timolol");
        assert_eq!(strip_salt("oxalate d'escitalopram"), "escitalopram");
        assert_eq!(strip_salt("malate de sitagliptine"), "sitagliptine");
        assert_eq!(strip_salt("propionate de fluticasone"), "fluticasone");
        assert_eq!(strip_salt("carbonate de calcium"), "calcium");
        assert_eq!(strip_salt("bicarbonate de sodium"), "sodium");
        assert_eq!(strip_salt("clavulanate de potassium"), "potassium");
    }

    #[test]
    fn test_strip_salt_new_suffixes() {
        assert_eq!(strip_salt("pantoprazole sodique sesquihydraté"), "pantoprazole");
        assert_eq!(strip_salt("fumarate de quétiapine"), "quétiapine");
        // hydrates
        assert_eq!(strip_salt("calcium dihydraté"), "calcium");
        assert_eq!(strip_salt("magnésium hexahydraté"), "magnésium");
    }

    #[test]
    fn test_strip_salt_amino_acids_preserved() {
        // Arginine is an amino acid (active ingredient), NOT a salt form.
        // It must NOT be stripped as a suffix — regression test for BDPM substance_code 01178.
        assert_eq!(strip_salt("arginine"), "arginine");
        assert_eq!(strip_salt("L-arginine"), "L-arginine");
        assert_eq!(strip_salt("ARGININE"), "ARGININE");
        // "d'arginine" directly: prefix "d'" matches → stripped to "arginine"
        assert_eq!(strip_salt("d'arginine"), "arginine");
        // "chlorhydrate d'arginine" → compound prefix "chlorhydrate d'" stripped → "arginine"
        assert_eq!(strip_salt("chlorhydrate d'arginine"), "arginine");
        // "arginine (chlorhydrate)" — parenthetical is NOT stripped by strip_salt alone
        // (normalize_compo strips parens via strip_parens first, then passes result to strip_salt)
        // Here we just verify strip_salt doesn't corrupt the input:
        assert_eq!(strip_salt("arginine (chlorhydrate)"), "arginine (chlorhydrate)");
    }

    #[test]
    fn test_strip_salt_audit_prefixes() {
        // High-frequency salt forms found during DB audit
        // bésilate (alternate spelling of bésylate) — 157 rows
        assert_eq!(strip_salt("bésilate d'amlodipine"), "amlodipine");
        assert_eq!(strip_salt("BÉSILATE D'AMLODIPINE"), "AMLODIPINE");
        // xinafoate — 42 rows
        assert_eq!(strip_salt("xinafoate de salmétérol"), "salmétérol");
        // digluconate — 27 rows
        assert_eq!(strip_salt("digluconate de chlorhexidine"), "chlorhexidine");
        // dipropionate — 38 rows
        assert_eq!(strip_salt("dipropionate de béclométasone"), "béclométasone");
        // furoate — 30 rows
        assert_eq!(strip_salt("furoate de mométasone"), "mométasone");
        // nitrate d' — 45 rows
        assert_eq!(strip_salt("nitrate d'éconazole"), "éconazole");
        // bromure d' — 36 rows
        assert_eq!(strip_salt("bromure d'ipratropium"), "ipratropium");
        // mésylate d' — 22 rows
        assert_eq!(strip_salt("mésylate d'imatimib"), "imatimib");
        // mésilate (I-variant) — same salt form, alternate spelling
        assert_eq!(strip_salt("mésilate d'imatimib"), "imatimib");
        assert_eq!(strip_salt("MÉSILATE D'IMATINIB"), "IMATINIB");
        assert_eq!(strip_salt("dimésylate de lisdexamfetamine"), "lisdexamfetamine");
        assert_eq!(strip_salt("dimésilate de lisdexamfetamine"), "lisdexamfetamine");
        // palmitate — 17 rows
        assert_eq!(strip_salt("palmitate de palipéridone"), "palipéridone");
        // résinate — 17 rows
        assert_eq!(strip_salt("résinate de nicotine"), "nicotine");
        // tosylate/tosilate — 13 rows
        assert_eq!(strip_salt("tosylate de périndopril"), "périndopril");
        assert_eq!(strip_salt("tosilate de périndopril"), "périndopril");
    }

    // --- fts_normalize ---

    #[test]
    fn test_fts_normalize() {
        assert_eq!(fts_normalize("Doliprane"), "doliprane");
        assert_eq!(fts_normalize("Doliprane"), "doliprane"); // same, no accented
        assert_eq!(fts_normalize("comprimé de paracétamol"), "comprime paracetamol"); // diacritics stripped
        assert_eq!(fts_normalize("Aspirin"), "aspirin");
        assert_eq!(fts_normalize(""), "");
        // Noise word filtering
        assert_eq!(fts_normalize("Doliprane de Sanofi"), "doliprane sanofi"); // "de" removed
    }
}

// --- insta snapshots for strip_salt ---

#[cfg(test)]
mod strip_salt_snapshots {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_strip_salt_snapshot_complex_forms() {
        let cases = vec![
            // High-frequency salt forms
            ("bésilate d'amlodipine", "amlodipine"),
            ("BÉSILATE D'AMLODIPINE", "AMLODIPINE"),
            ("chlorhydrate d'arginine", "arginine"),
            ("mésylate d'imatinib", "imatinib"),
            ("MÉSILATE D'IMATINIB", "IMATINIB"),
            ("dimésylate de lisdexamfetamine", "lisdexamfetamine"),
            ("xinafoate de salmétérol", "salmétérol"),
            ("digluconate de chlorhexidine", "chlorhexidine"),
            ("dipropionate de béclométasone", "béclométasone"),
            ("furoate de mométasone", "mométasone"),
            ("nitrate d'éconazole", "éconazole"),
            ("bromure d'ipratropium", "ipratropium"),
            // Hydrate forms
            ("amoxicilline trihydratée", "amoxicilline"),
            ("pantoprazole sodique sesquihydraté", "pantoprazole"),
            ("calcium dihydraté", "calcium"),
            ("magnésium hexahydraté", "magnésium"),
            // Amino acids (must NOT be stripped)
            ("arginine", "arginine"),
            ("L-arginine", "L-arginine"),
            ("d'arginine", "arginine"),
            // Multi-pass suffix stripping
            ("paracetamol chlorhydrate monohydrate", "paracetamol"),
            ("diclofenac sodique", "diclofenac"),
            ("atorvastatine calcique", "atorvastatine"),
            ("chlorhydrate de sodium", "sodium"),
            ("bicarbonate de sodium", "sodium"),
            // No salt form
            ("paracetamol 500mg", "paracetamol 500mg"),
            ("aspirine", "aspirine"),
            // Empty
            ("", ""),
        ];

        for (input, _expected) in cases {
            let output = strip_salt(input);
            assert_eq!(output, _expected, "input: {input}");
            assert_debug_snapshot!(input.replace(['\'', ' '], "_"), &output);
        }
    }

    #[test]
    fn test_strip_salt_ascorbate_sodique() {
        // ASCORBATE SODIQUE — regression: ascorbate was in SALT_SUFFIXES
        // and stripped the entire active ingredient to empty string.
        assert_eq!(strip_salt("ASCORBATE SODIQUE"), "ASCORBATE");
    }
}

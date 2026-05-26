# Analyse d'Encodage et Quirks des Fichiers BDPM

> Analyse approfondie des problèmes d'encodage, de formatage et de cohérence dans les fichiers de la BDPM.
> Date d'analyse : 26 mai 2026

---

## 1. Problème central : Encodage mixte

### 1.1 Constat

Les fichiers BDPM utilisent **deux encodages différents** sans aucune indication dans les fichiers (pas de BOM, pas d'en-tête) :

| Fichier | Encodage réel | Détection chardet | Confiance chardet |
|---------|--------------|-------------------|-------------------|
| CIS_bdpm.txt | **Windows-1252** | ISO-8859-1 | 0.730 |
| CIS_CIP_bdpm.txt | **UTF-8** | UTF-8 | 0.990 |
| CIS_COMPO_bdpm.txt | **Windows-1252** | ISO-8859-1 | 0.730 |
| CIS_HAS_SMR_bdpm.txt | **Windows-1252** | Windows-1252 | 0.724 |
| CIS_HAS_ASMR_bdpm.txt | **Windows-1252** | Windows-1252 | 0.728 |
| HAS_LiensPageCT_bdpm.txt | **ASCII** | ASCII | 1.000 |
| CIS_GENER_bdpm.txt | **Windows-1252** | Windows-1252 | 0.730 |
| CIS_CPD_bdpm.txt | **Windows-1252** | ISO-8859-1 | 0.730 |
| CIS_CIP_Dispo_Spec.txt | **Windows-1252** | ISO-8859-1 | 0.730 |
| CIS_MITM.txt | **Windows-1252** | ISO-8859-1 | 0.730 |

### 1.2 Pourquoi la distinction CP1252 vs ISO-8859-1 est critique

Windows-1252 (CP1252) et ISO-8859-1 (Latin-1) sont identiques pour les octets 0x00-0x7F (ASCII) et 0xA0-0xFF (caractères accentués). La différence critique se trouve dans la plage **0x80-0x9F** :

| Octet | ISO-8859-1 | Windows-1252 |
|-------|-----------|--------------|
| `0x80` | Non défini | `€` (euro) |
| `0x85` | Non défini | `…` (ellipsis) |
| `0x89` | Non défini | `‰` (per mille) |
| `0x91` | Non défini | `'` (left single quote) |
| `0x92` | Non défini | `'` (right single quote / apostrophe) |
| `0x95` | Non défini | `•` (bullet) |
| `0x96` | Non défini | `–` (en dash) |
| `0x97` | Non défini | `—` (em dash) |
| `0x99` | Non défini | `™` (trademark) |

En ISO-8859-1, ces octets sont **illégaux** (C1 control characters). En Windows-1252, ce sont des caractères typographiques courants en français.

### 1.3 Inventaire des bytes CP1252 par fichier

Analyse exhaustive des octets dans la plage 0x80-0x9F dans chaque fichier :

#### CIS_bdpm.txt
| Octet | Caractère | Occurrences | Contexte typique |
|-------|-----------|-------------|-----------------|
| `0x92` | `'` | 28 | Apostrophe française dans les noms et titulaires |

#### CIS_CIP_bdpm.txt (UTF-8)
| Octet | Séquence UTF-8 | Caractère | Occurrences | Contexte |
|-------|---------------|-----------|-------------|----------|
| `0x80` | Partie de séquence multi-octets | `€` | 126 | Symbole euro dans les prix |
| `0x89` | Partie de séquence multi-octets | `‰` | 2 | Per mille |
| `0x97` | Partie de séquence multi-octets | `—` | 8 | Tiret long |
| `0x99` | Partie de séquence multi-octets | `™` | 126 | Trademark |

> Note : Dans un fichier UTF-8, les octets 0x80-0x9F apparaissent uniquement comme **continuation bytes** dans des séquences multi-octets. Ils ne sont pas des caractères CP1252 isolés.

#### CIS_HAS_SMR_bdpm.txt (le plus riche en CP1252)
| Octet | Caractère | Occurrences | Contexte typique |
|-------|-----------|-------------|-----------------|
| `0x85` | `…` | 12 | Points de suspension dans les libellés |
| `0x89` | `‰` | 1 | Symbole per mille |
| `0x91` | `'` | 8 | Guillemet gauche |
| `0x92` | `'` | 22 253 | **Apostrophe française** — le byte le plus fréquent ! |
| `0x95` | `•` | 4 159 | Puces dans les libellés structurés |
| `0x96` | `–` | 8 | Tiret demi-cadratin |

#### CIS_HAS_ASMR_bdpm.txt
| Octet | Caractère | Occurrences | Contexte typique |
|-------|-----------|-------------|-----------------|
| `0x85` | `…` | 6 | Points de suspension |
| `0x91` | `'` | 10 | Guillemet gauche |
| `0x92` | `'` | **29 704** | Apostrophe française — record absolu |
| `0x95` | `•` | 7 163 | Puces dans les libellés |
| `0x96` | `–` | 35 | Tiret demi-cadratin |
| `0x99` | `™` | 3 | Trademark |

#### CIS_GENER_bdpm.txt
| Octet | Caractère | Occurrences |
|-------|-----------|-------------|
| `0x92` | `'` | 26 |
| `0x96` | `–` | 85 |

#### CIS_CPD_bdpm.txt
| Octet | Caractère | Occurrences |
|-------|-----------|-------------|
| `0x92` | `'` | 141 |

#### CIS_MITM.txt
| Octet | Caractère | Occurrences |
|-------|-----------|-------------|
| `0x92` | `'` | 5 |

### 1.4 Stratégie de décodage recommandée

```
┌─────────────────────────────────────┐
│  Pour chaque fichier BDPM :         │
│                                     │
│  1. Lire les données brutes         │
│  2. Tenter UTF-8 en premier         │
│     ├─ Succès → utiliser UTF-8      │
│     └─ Échec → utiliser CP1252     │
│  3. Convertir vers UTF-8 interne    │
│     pour tout le pipeline           │
└─────────────────────────────────────┘
```

**Implémentation Rust :**

```rust
fn decode_bdpm_bytes(raw: &[u8]) -> String {
    // Tentative UTF-8 d'abord
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => {
            // Fallback CP1252 — sûr pour tous les fichiers BDPM
            encoding_rs::WINDOWS_1252.decode(raw).0.into_owned()
        }
    }
}
```

**Pourquoi CP1252 plutôt que Latin-1 ?**
- CP1252 est un sur-ensemble de Latin-1 pour les octets 0x80-0x9F
- Latin-1 decoderait ces octets en caractères de contrôle invisibles ou les rejetterait
- CP1252 les décode correctement en caractères typographiques français
- La bibliothèque `encoding_rs` en Rust gère CP1252 nativement et efficacement

---

## 2. Problème des fins de ligne

### 2.1 Constat

| Fichier | CRLF (`\r\n`) | LF (`\n`) | Mixte |
|---------|--------------|-----------|-------|
| CIS_bdpm.txt | 15 848 | 0 | Non |
| CIS_CIP_bdpm.txt | **0** | **20 903** | Non |
| CIS_COMPO_bdpm.txt | 32 389 | 0 | Non |
| CIS_HAS_SMR_bdpm.txt | 15 257 | 0 | Non |
| CIS_HAS_ASMR_bdpm.txt | 9 906 | 0 | Non |
| HAS_LiensPageCT_bdpm.txt | 10 342 | 0 | Non |
| CIS_GENER_bdpm.txt | 10 704 | 0 | Non |
| CIS_CPD_bdpm.txt | 28 154 | 0 | Non |
| CIS_CIP_Dispo_Spec.txt | 766 | 0 | Non |
| CIS_MITM.txt | 7 711 | 0 | Non |

### 2.2 Impact

Le fichier `CIS_CIP_bdpm.txt` utilise exclusivement LF (Unix), tandis que tous les autres utilisent CRLF (Windows). Un parser qui supprime les `\r` avant de découper les lignes fonctionnera correctement pour les deux. Un parser qui suppose CRLF partout laissera des `\r` en fin de ligne pour les fichiers Windows.

### 2.3 Recommandation

```rust
// Normaliser toutes les fins de ligne au moment du parsing
fn normalize_line_endings(raw: &[u8]) -> String {
    let text = decode_bdpm_bytes(raw);
    text.replace("\r\n", "\n").replace('\r', "\n")
}
```

---

## 3. Problème des tabulations de fin (Trailing Tabs)

### 3.1 Constat

Le fichier `CIS_CIP_bdpm.txt` contient **20 089 lignes sur 20 903** (96,1%) qui se terminent par une ou plusieurs tabulations. Cela crée un champ vide supplémentaire à la fin de chaque ligne affectée.

### 3.2 Impact

Si on compte les champs par simple `split('\t')`, les lignes avec trailing tabs donneront 14 champs au lieu de 13. Le champ 14 sera une chaîne vide.

### 3.3 Recommandation

```rust
fn parse_bdpm_line(line: &str, expected_fields: usize) -> Vec<String> {
    let mut fields: Vec<String> = line.split('\t').map(String::from).collect();
    // Supprimer les champs vides de fin
    while fields.len() > expected_fields && fields.last().map_or(false, |f| f.is_empty()) {
        fields.pop();
    }
    // Compléter si pas assez de champs
    while fields.len() < expected_fields {
        fields.push(String::new());
    }
    fields
}
```

---

## 4. Problème des formats de date incohérents

### 4.1 Constat

Deux formats de date coexistent dans les fichiers BDPM :

| Fichier | Champ | Format | Exemple |
|---------|-------|--------|---------|
| CIS_bdpm.txt | Date d'AMM (champ 8) | **DD/MM/YYYY** | `12/03/1998` |
| CIS_CIP_bdpm.txt | Date de déclaration (champ 6) | **DD/MM/YYYY** | `16/03/2011` |
| CIS_CIP_Dispo_Spec.txt | Dates (champs 5-7) | **DD/MM/YYYY** | `14/04/2026` |
| CIS_HAS_SMR_bdpm.txt | Date de l'avis (champ 4) | **YYYYMMDD** | `20260401` |
| CIS_HAS_ASMR_bdpm.txt | Date de l'avis (champ 4) | **YYYYMMDD** | `20260401` |
| CIS_InfoImportantes.txt | Dates (champs 2-3) | **DD/MM/YYYY** | variable |

### 4.2 Recommandation

Normaliser toutes les dates au format ISO 8601 (`YYYY-MM-DD`) dans la base SQLite :

```rust
fn normalize_date(input: &str) -> Option<String> {
    if input.is_empty() {
        return None;
    }
    // Format YYYYMMDD (fichiers HAS)
    if input.len() == 8 && input.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("{}-{}-{}", &input[0..4], &input[4..6], &input[6..8]));
    }
    // Format DD/MM/YYYY
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() == 3 {
        return Some(format!("{}-{}-{}", parts[2], parts[1], parts[0]));
    }
    None
}
```

---

## 5. Problème du contenu HTML embarqué

### 5.1 Constat

Plusieurs fichiers contiennent du HTML brut dans certains champs textuels :

| Fichier | Champ concerné | Lignes avec HTML | Type de HTML |
|---------|---------------|-----------------|--------------|
| CIS_CIP_bdpm.txt | Indications remboursement (champ 13) | 814 | `<br>`, `<a href>`, `¿` |
| CIS_HAS_SMR_bdpm.txt | Libellé SMR (champ 6) | 1 975 | `<br>`, `•`, `–` |
| CIS_HAS_ASMR_bdpm.txt | Libellé ASMR (champ 6) | 2 060 | `<br>`, `•`, `–` |

### 5.2 Exemples de contenu HTML

**CIS_HAS_SMR_bdpm.txt :**
```
Le service médical rendu par KISQALI 200 mg (ribociclib), comprimé pelliculé, est important :<br>• en association au fulvestrant chez les femmes ménopausées ayant un cancer du sein localement avancé ou métastatique RH+/HER2-...
```

**CIS_CIP_bdpm.txt :**
```
Ce médicament peut être pris en charge ou remboursé par l'Assurance Maladie dans les cas suivants :<br><br>- Asthme persistant sévère de l¿enfant (en traitement quotidien).
```

### 5.3 Problème du `¿` dans CIS_CIP_bdpm.txt

Dans le fichier UTF-8 `CIS_CIP_bdpm.txt`, le caractère `¿` (U+00BF, INVERTED QUESTION MARK) apparaît fréquemment dans les indications de remboursement. Ce n'est PAS une erreur d'encodage — c'est utilisé comme substitut à l'apostrophe française dans certains contextes (probablement un artefact du système source qui produit ces données). Il devrait être traité ou nettoyé lors du parsing.

### 5.4 Recommandation

Deux approches possibles :
1. **Conservation brute** : Stocker le HTML tel quel dans la base, laisser l'application consommatrice le traiter
2. **Nettoyage** : Supprimer les balises HTML et normaliser les caractères spéciaux au moment de l'import

Pour une base de données de référence, l'approche **hybride** est recommandée :
- Stocker le texte brut (champ `libelle_raw`)
- Stocker une version nettoyée/sanitisée (champ `libelle_clean`)

---

## 6. Problème de l'inconsistance des taux de remboursement

### 6.1 Constat

Le champ taux de remboursement (CIS_CIP_bdpm.txt, champ 9) présente une **inconsistance de formatage** :

| Format | Exemples | Nombre d'occurrences |
|--------|---------|---------------------|
| Avec espace | `65 %`, `100 %`, `30 %`, `15 %` | 2 453 |
| Sans espace | `65%`, `100%`, `30%`, `15%`, `35%` | 12 093 |
| Vide | `""` | 7 357 |

### 6.2 Recommandation

Normaliser systématiquement en supprimant les espaces et en convertissant en nombre entier :

```rust
fn parse_taux(input: &str) -> Option<u8> {
    if input.is_empty() {
        return None;
    }
    let cleaned: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    cleaned.parse().ok()
}
```

---

## 7. Problème du fichier dynamique CIS_InfoImportantes.txt

### 7.1 Constat

Ce fichier est unique dans l'écosystème BDPM :

1. **Chemin différent** : `/download/CIS_InfoImportantes.txt` (sans `/file/`)
2. **Nom dynamique** : Le Content-Disposition renvoie `CIS_InfoImportantes_20260526114141_bdpm.txt` avec un timestamp
3. **Peut être vide** : Le fichier peut contenir 0 octets
4. **Génération en direct** : Le fichier est généré à la volée au moment de la requête, pas pré-généré comme les autres

### 7.2 En-têtes HTTP observés

```
Content-Type: application/force-download
Content-Disposition: attachment; filename=CIS_InfoImportantes_20260526114141_bdpm.txt
```

### 7.3 Recommandation

- Toujours vérifier la taille du fichier après téléchargement (0 octets = pas d'information en cours)
- Extraire le timestamp du nom de fichier pour tracker la fraîcheur des données
- Prévoir un mécanisme de retry car la génération dynamique peut échouer

---

## 8. Problème des enregistrements orphelins

### 8.1 Constat

L'analyse d'intégrité référentielle révèle des CIS codes présents dans les fichiers secondaires mais absents du fichier central (CIS_bdpm.txt) :

| Fichier | CIS uniques | Orphelins | % orphelins |
|---------|-------------|-----------|-------------|
| CIS_CIP_bdpm.txt | 14 573 | 4 | 0,03% |
| CIS_COMPO_bdpm.txt | 15 846 | 0 | 0% |
| CIS_HAS_SMR_bdpm.txt | 9 014 | 2 806 | 31,1% |
| CIS_HAS_ASMR_bdpm.txt | 6 172 | 1 567 | 25,4% |
| CIS_GENER_bdpm.txt | 10 628 | 2 503 | 23,6% |
| CIS_CPD_bdpm.txt | 12 492 | 0 | 0% |
| CIS_CIP_Dispo_Spec.txt | 727 | 12 | 1,6% |
| CIS_MITM.txt | 7 711 | 0 | 0% |

### 8.2 Explication

Les fichiers HAS (SMR/ASMR) et GENER contiennent des avis et classifications pour des spécialités qui **ne sont plus dans le fichier central** CIS_bdpm.txt. La BDPM ne couvre que les médicaments commercialisés ou retirés depuis moins de 5 ans. Les avis HAS et groupes génériques peuvent concerner des spécialités plus anciennes.

### 8.3 Recommandation

- **Ne PAS rejeter** les enregistrements orphelins — ils contiennent des informations historiques utiles
- Créer une table `cis_orphelins` pour tracker ces CIS codes
- Dans le schéma SQLite, utiliser des **FOREIGN KEY sans ON DELETE CASCADE** et permettre les clés étrangères optionnelles

---

## 9. Tableau récapitulatif des quirks

| # | Quirk | Sévérité | Fichiers affectés | Solution |
|---|-------|----------|-------------------|----------|
| 1 | Encodage mixte CP1252/UTF-8 | **Critique** | Tous sauf LiensPageCT | Détection automatique + fallback CP1252 |
| 2 | Fins de ligne mixtes CRLF/LF | **Moyen** | CIS_CIP_bdpm.txt | Normalisation universelle |
| 3 | Tabulations de fin | **Moyen** | CIS_CIP_bdpm.txt (96% des lignes) | Trim des champs vides de fin |
| 4 | Dates en deux formats | **Élevé** | Fichiers HAS vs autres | Normalisation ISO 8601 |
| 5 | HTML dans les champs texte | **Moyen** | CIP, SMR, ASMR | Double stockage (raw + clean) |
| 6 | Taux de remboursement inconsistent | **Faible** | CIS_CIP_bdpm.txt | Normalisation (suppression espaces) |
| 7 | Fichier dynamique InfoImportantes | **Élevé** | CIS_InfoImportantes.txt | Gestion spéciale du chemin et du timestamp |
| 8 | Enregistrements orphelins | **Faible** | HAS, GENER | Clés étrangères optionnelles |
| 9 | Inconsistance de capitalisation | **Faible** | CIS_CIP_Dispo_Spec.txt | Normalisation du texte |
| 10 | Pas de BOM ni en-tête | **Faible** | Tous | Détection automatique d'encodage |
| 11 | Virgule comme séparateur décimal | **Faible** | CIS_CIP_bdpm.txt | Conversion en point décimal pour SQLite |
| 12 | `¿` dans les indications UTF-8 | **Faible** | CIS_CIP_bdpm.txt | Nettoyage ou mapping |

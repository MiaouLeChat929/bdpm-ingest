# 03 — Méthodologie de collecte et de mise à jour

---

## 3.1 Stratégie de téléchargement respectueuse

### Principes

La collecte des fichiers BDPM doit respecter la source et éviter toute approche agressive. Les principes directeurs sont :

1. **Pas de requêtes parallèles** : télécharger un fichier à la fois
2. **Intervalle minimum entre les requêtes** : 2 secondes minimum
3. **User-Agent explicite** : identifier le client (ex : `BDPM-Importer/1.0 (contact@example.com)`)
4. **Pas de requête inutile** : ne télécharger que si le contenu a changé
5. **Archivage systématique** : conserver chaque version téléchargée

### Configuration de téléchargement recommandée

```rust
struct FetchConfig {
    base_url: &'static str,  // "https://base-donnees-publique.medicaments.gouv.fr"
    user_agent: &'static str, // "BDPM-Importer/1.0"
    min_interval_secs: u64,   // 2
    max_retries: u32,         // 3
    retry_backoff_secs: u64,  // 5 (puis 10, 15)
    timeout_secs: u64,        // 60
    archive_dir: PathBuf,     // "./archive/"
}
```

### Processus de téléchargement

```
Pour chaque fichier :
  1. Vérifier le hash SHA-256 de l'archive précédente (si existant)
  2. Télécharger le fichier binaire (sans décodage texte)
  3. Calculer le SHA-256 du contenu téléchargé
  4. Si hash identique au précédent → court-circuiter, journaliser "no_change"
  5. Si hash différent → archiver le fichier brut avec horodatage
  6. Mettre à jour le manifeste de collecte
  7. Attendre min_interval_secs avant la prochaine requête
```

---

## 3.2 Détection de changement : comparaison des approches

| Approche | Faisabilité | Fiabilité | Coût réseau | Verdict |
|----------|------------|-----------|-------------|---------|
| **ETag / If-None-Match** | Non disponible (serveur ne fournit pas ETag) | N/A | N/A | ❌ Écartée |
| **Last-Modified / If-Modified-Since** | Non disponible | N/A | N/A | ❌ Écartée |
| **HEAD + Content-Length** | Non disponible (Content-Length absent) | N/A | N/A | ❌ Écartée |
| **SHA-256 du contenu** | Pleine | 100% | Complet mais nécessaire | ✅ **Recommandée** |
| **Date page HTML** | Possible (scraping) | Partielle (page pas toujours à jour) | Léger | 🟡 Complémentaire |
| **Taille fichier** | Simple | Faible (changement possible même taille) | N/A | ❌ Insuffisante |

### Approche recommandée : SHA-256

Le SHA-256 est la seule méthode fiable à 100% pour détecter tout changement de contenu. Le coût réseau est inévitable (téléchargement complet), mais la taille totale des fichiers (~27,6 Mo) reste modeste.

**Optimisation** : Pour les 10 fichiers statiques (hors InfoImportantes), le téléchargement hebdomadaire représente ~27,6 Mo × 52 semaines ≈ 1,4 Go/an, ce qui est négligeable.

### Approche complémentaire : scraping de la page HTML

La page de téléchargement affiche les dates de mise à jour de chaque fichier. Un scraping léger (une requête GET sur la page HTML) permet de détecter visuellement si une mise à jour a eu lieu avant de lancer les téléchargements complets. Cette approche est optionnelle mais réduit le trafic réseau dans les semaines sans changement.

```rust
async fn check_page_updates(client: &reqwest::Client) -> Result<HashMap<String, String>> {
    // Scrape la page /telechargement
    // Extrait les dates de mise à jour par fichier
    // Compare avec les dates de l'import précédent
    // Retourne la liste des fichiers dont la date a changé
}
```

---

## 3.3 Fréquences de vérification

Deux régimes sont recommandés selon le type de fichier :

### Régime standard (10 fichiers statiques)

| Paramètre | Valeur |
|-----------|--------|
| Fréquence de vérification | Hebdomadaire (ex : lundi 3h du matin) |
| Méthode | Téléchargement + SHA-256 |
| Fichiers concernés | CIS_bdpm, CIS_CIP, CIS_COMPO, CIS_HAS_SMR, CIS_HAS_ASMR, HAS_LiensPageCT, CIS_GENER, CIS_CPD, CIS_CIP_Dispo_Spec, CIS_MITM |

Justification : la cadence mensuelle des mises à jour rend la vérification hebdomadaire suffisante. Toute mise à jour sera détectée dans les 7 jours.

### Régime dynamique (CIS_InfoImportantes.txt)

| Paramètre | Valeur |
|-----------|--------|
| Fréquence de vérification | Quotidienne (ex : 6h du matin) |
| Méthode | Téléchargement + SHA-256 |
| Fichiers concernés | CIS_InfoImportantes.txt |

Justification : ce fichier est généré en temps réel et peut contenir des alertes de sécurité urgentes. La vérification quotidienne est un compromis entre réactivité et charge réseau.

### Planning de vérification recommandé

```
Lundi 03:00  →  Tous les 11 fichiers (vérification hebdomadaire + quotidienne)
Mardi 06:00  →  CIS_InfoImportantes.txt
Mercredi 06:00  →  CIS_InfoImportantes.txt
Jeudi 06:00  →  CIS_InfoImportantes.txt
Vendredi 06:00  →  CIS_InfoImportantes.txt
Samedi 06:00  →  CIS_InfoImportantes.txt
Dimanche 06:00  →  CIS_InfoImportantes.txt
```

---

## 3.4 Structure d'archivage

Chaque fichier téléchargé est archivé avec son horodatage pour permettre le rejeu et le diagnostic :

```
archive/
├── 2026-05-26T030000/
│   ├── CIS_bdpm.txt
│   ├── CIS_CIP_bdpm.txt
│   ├── CIS_COMPO_bdpm.txt
│   ├── CIS_HAS_SMR_bdpm.txt
│   ├── CIS_HAS_ASMR_bdpm.txt
│   ├── HAS_LiensPageCT_bdpm.txt
│   ├── CIS_GENER_bdpm.txt
│   ├── CIS_CPD_bdpm.txt
│   ├── CIS_CIP_Dispo_Spec.txt
│   ├── CIS_MITM.txt
│   ├── CIS_InfoImportantes.txt
│   └── manifest.json
├── 2026-05-27T060000/
│   ├── CIS_InfoImportantes.txt
│   └── manifest.json
└── ...
```

### Format du manifeste (manifest.json)

```json
{
  "timestamp": "2026-05-26T03:00:00Z",
  "files": [
    {
      "name": "CIS_bdpm.txt",
      "sha256": "a1b2c3d4e5f6...",
      "size_bytes": 3164943,
      "http_status": 200,
      "download_duration_ms": 842
    },
    {
      "name": "CIS_InfoImportantes.txt",
      "sha256": "f6e5d4c3b2a1...",
      "size_bytes": 4219972,
      "http_status": 200,
      "content_disposition": "CIS_InfoImportantes_20260526111334_bdpm.txt",
      "download_duration_ms": 1203
    }
  ],
  "changed_files": ["CIS_InfoImportantes.txt"],
  "unchanged_files": ["CIS_bdpm.txt", "CIS_CIP_bdpm.txt", "..."]
}
```

---

## 3.5 Traçabilité des imports

Chaque import dans la base SQLite doit être tracé avec les métadonnées suivantes :

| Champ | Type | Description |
|-------|------|-------------|
| `id` | INTEGER PK | Identifiant auto-incrémenté |
| `timestamp` | TEXT ISO 8601 | Horodatage de l'import |
| `file_name` | TEXT | Nom du fichier source |
| `sha256` | TEXT | Hash SHA-256 du fichier source |
| `rows_read` | INTEGER | Nombre de lignes lues dans le fichier |
| `rows_inserted` | INTEGER | Nombre de lignes insérées |
| `rows_updated` | INTEGER | Nombre de lignes mises à jour |
| `rows_deleted` | INTEGER | Nombre de suppressions logiques |
| `status` | TEXT | `success` / `partial` / `failed` |
| `duration_ms` | INTEGER | Durée de l'import en millisecondes |
| `error_message` | TEXT | Message d'erreur éventuel |

Cette table de log constitue la base de la reproductibilité et du diagnostic. En cas de problème, elle permet de savoir exactement quelle version de quel fichier a été importée et quand.

---

## 3.6 Gestion des erreurs réseau

### Scénarios d'erreur et réponses

| Scénario | Code HTTP | Action |
|----------|-----------|--------|
| Fichier disponible | 200 | Télécharger et traiter |
| Fichier introuvable | 404 | Alerte critique : fichier supprimé ou déplacé |
| Rate limiting | 429 | Backoff exponentiel (5s, 10s, 15s) puis retry |
| Erreur serveur | 500/502/503 | Backoff exponentiel, max 3 retries |
| Timeout | N/A | Retry avec timeout augmenté |
| Certificat SSL invalide | N/A | Échec immédiat, ne pas ignorer |

### Politique de retry

```rust
async fn fetch_with_retry(client: &Client, url: &str, max_retries: u32) -> Result<Vec<u8>> {
    let mut attempt = 0;
    loop {
        match client.get(url).send().await {
            Ok(response) if response.status() == 200 => {
                return Ok(response.bytes().await?.to_vec());
            }
            Ok(response) if response.status() == 429 || response.status().is_server_error() => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(FetchError::MaxRetriesExceeded);
                }
                let delay = Duration::from_secs(5 * attempt as u64);
                tokio::time::sleep(delay).await;
            }
            Ok(response) if response.status() == 404 => {
                return Err(FetchError::FileNotFound(url.to_string()));
            }
            Ok(response) => {
                return Err(FetchError::UnexpectedStatus(response.status()));
            }
            Err(e) if e.is_timeout() => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(FetchError::Timeout);
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(FetchError::Network(e)),
        }
    }
}
```

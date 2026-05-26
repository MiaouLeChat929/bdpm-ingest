# 08 — Risques techniques et points à valider

---

## 8.1 Risques identifiés

### R1 : Changement de schéma sans notification

**Sévérité** : 🔴 Critique  
**Probabilité** : Moyenne (la doc de format est en v4, preuve de changements passés)  
**Impact** : Le parser échoue silencieusement ou produit des données corrompues

**Description** : L'absence de changelog et de versionnement signifie qu'un changement de format (ajout de colonne, modification d'énumération, changement d'encodage) ne sera détecté qu'au moment du parsing. La documentation PDF de format existe en version 4, ce qui prouve que des évolutions ont eu lieu par le passé.

**Mitigation** :
- Le parser doit journaliser tout écart par rapport au schéma attendu
- En cas de nombre de colonnes inattendu : alerter et échouer gracieusement, ne pas insérer silencieusement
- Ajouter un mécanisme de `schema_version` dans la configuration
- Surveiller la page de téléchargement pour les changements de documentation
- Archiver les fichiers de plusieurs mois consécutifs et comparer les structures

---

### R2 : Disparition ou déplacement d'un fichier

**Sévérité** : 🔴 Critique  
**Probabilité** : Faible  
**Impact** : Données manquantes sans alerte

**Description** : Si un fichier est renommé ou supprimé du serveur BDPM, le pipeline doit le détecter et alerter, pas ignorer silencieusement.

**Mitigation** :
- Le fetcher doit vérifier le code HTTP et lever une alerte pour tout statut différent de 200
- La liste des fichiers attendus est codée en dur dans la configuration
- Toute réponse 404 génère une alerte critique immédiate
- Le rapport de validation vérifie que les 11 tables ont été mises à jour dans le même import

---

### R3 : Encodage changeant

**Sévérité** : 🟡 Important  
**Probabilité** : Faible (mais probable lors d'une migration technique)  
**Impact** : Caractères de substitution ou erreurs de décodage

**Description** : Si l'ANSM migre un fichier de cp1252 vers UTF-8 sans annonce, le parser échouera sur les caractères spécifiques à cp1252.

**Mitigation** :
- Implémenter un détecteur d'encodage de secours qui s'active si le décodage par défaut produit des caractères de substitution (U+FFFD)
- Le crate `encoding_rs` fournit `encoding_rs::detect()` pour la détection statistique
- Journaliser le nombre de caractères de substitution pour chaque fichier
- En cas de détection d'encodage différent de la configuration, alerter et utiliser l'encodage détecté

```rust
fn decode_with_fallback(data: &[u8], expected: FileEncoding) -> String {
    let primary = decode_bytes(data, expected);
    let substitution_count = primary.chars().filter(|&c| c == '\u{FFFD}').count();
    
    if substitution_count > 0 && substitution_count < data.len() / 100 {
        // Quelques substitutions : probablement un mauvais encodage
        // Essayer UTF-8 en fallback
        let fallback = String::from_utf8_lossy(data).to_string();
        let fallback_subs = fallback.chars().filter(|&c| c == '\u{FFFD}').count();
        if fallback_subs < substitution_count {
            tracing::warn!("Encoding fallback: {:?} → UTF-8", expected);
            return fallback;
        }
    }
    primary
}
```

---

### R4 : Rate-limiting ou blocage IP

**Sévérité** : 🟡 Important  
**Probabilité** : Faible à moyenne (si la fréquence est trop élevée)  
**Impact** : Impossible de télécharger les mises à jour

**Description** : Le serveur BDPM peut bloquer les requêtes trop fréquentes. Aucune documentation officielle sur les limites de débit n'est disponible.

**Mitigation** :
- Respecter un intervalle minimum de 2 secondes entre les requêtes
- Utiliser un User-Agent explicite : `BDPM-Importer/1.0 (contact@example.com)`
- Implémenter un backoff exponentiel en cas de code HTTP 429 ou 503
- Ne jamais lancer de requêtes parallèles
- Fréquence de vérification raisonnable : hebdomadaire pour les statiques, quotidien pour InfoImportantes

---

### R5 : Perte de données historiques

**Sévérité** : 🟡 Important  
**Probabilité** : Élevée (si l'implémentation écrase la base)  
**Impact** : Perte des médicaments retirés du répertoire CIS_bdpm.txt

**Description** : Les médicaments retirés du répertoire CIS_bdpm.txt (plus de 2 ans après l'arrêt) disparaissent des imports futurs. Si le pipeline écrase la base à chaque import au lieu de faire un import incrémental avec soft delete, les données historiques sont perdues.

**Mitigation** :
- Implémenter le modèle incrémental dès la phase 5 avec soft delete
- Archiver systématiquement les fichiers bruts (ne jamais supprimer les archives)
- La colonne `_is_active` permet de distinguer les enregistrements actifs des inactifs
- Les orphelins (enregistrements dans les tables HAS/GENER sans correspondance CIS active) sont conservés avec `is_orphan = 1`

---

## 8.2 Angles morts

### AM1 : Complétude des données

Les données de l'ANSM sont présumées complètes pour le périmètre déclaré, mais il n'existe aucun moyen de vérifier cette complétude de manière indépendante. Il est impossible de savoir si certains médicaments commercialisés sont manquants.

### AM2 : Fréquence exacte de mise à jour

La fréquence de mise à jour de chaque fichier n'est pas documentée officiellement (seulement la cadence mensuelle globale). Les variations observées (CIS_CIP mis à jour en dehors du cycle, CIS_MITM avec 2 mois de retard) pourraient être accidentelles plutôt que délibérées.

### AM3 : Canal MySQL alternatif

Le projet betagouv/infomedicament référence l'utilisation d'un dump MySQL de la BDPM, qui pourrait contenir des données supplémentaires (codes ATC, images, textes de RCP). L'accessibilité et le contenu de ce dump restent opaques.

### AM4 : Refonte potentielle du site

Le site BDPM utilise le DSFR (Système de Design de l'État) et des technologies qui pourraient évoluer. Une refonte du site pourrait changer les URLs de téléchargement, la structure des fichiers, ou le mode de distribution. L'impact est impossible à anticiper mais le risque existe à moyen terme.

---

## 8.3 Points à valider empiriquement

### PV1 : Stabilité du schéma dans le temps

**Priorité** : 🔴 Haute  
**Méthode** : Archiver les fichiers de 3 mois consécutifs et comparer les structures  
**Ce qu'il faut vérifier** :
- Le nombre de colonnes par fichier reste constant
- L'ordre des colonnes ne change pas
- Les énumérations n'ajoutent pas de nouvelles valeurs
- Aucun fichier n'est ajouté ou supprimé

**Action** : Configurer le fetcher pour archiver automatiquement chaque version, puis comparer après 3 mois d'archives.

---

### PV2 : Comportement du serveur sous charge

**Priorité** : 🟡 Moyenne  
**Méthode** : Tests de charge respectueux (1 fichier à la fois, intervalle 5s)  
**Ce qu'il faut vérifier** :
- Le serveur applique-t-il un rate-limiting ?
- Les téléchargements simultanés de tous les fichiers sont-ils possibles ?
- Quel User-Agent est acceptable ?
- Y a-t-il une limite de bande passante par IP ?

**Action** : Lancer un téléchargement complet des 11 fichiers avec un intervalle de 5 secondes et observer les temps de réponse et les codes HTTP.

---

### PV3 : Site de recette ANSM

**Priorité** : 🟢 Basse  
**URL** : `rec-bdm.ansm.integra.fr`  
**Ce qu'il faut vérifier** :
- Le site publie-t-il des fichiers au format différent ?
- Des fichiers supplémentaires y sont-ils disponibles ?
- Les structures sont-elles en avance sur la production ?

**Attention** : Ce site n'est pas officiellement documenté comme public. L'utilisation doit se limiter à l'observation ponctuelle, sans téléchargement automatique.

---

### PV4 : Existence du dump MySQL

**Priorité** : 🟡 Moyenne  
**Source** : Projet betagouv/infomedicament  
**Ce qu'il faut vérifier** :
- Le dump MySQL est-il publiquement accessible ?
- Contient-il des données supplémentaires (codes ATC, textes de RCP, images) ?
- Quelle est sa fréquence de mise à jour ?

**Action** : Contacter les mainteneurs du projet infomedicament ou rechercher l'URL du dump dans le code source du projet.

---

### PV5 : Récurrence de l'anomalie \r\r\n dans CIS_CPD

**Priorité** : 🟢 Basse  
**Méthode** : Surveiller CIS_CPD_bdpm.txt sur 3 mois consécutifs  
**Ce qu'il faut vérifier** :
- Les séquences \r\r\n réapparaissent-elles régulièrement ?
- Sont-elles toujours au même endroit ?
- Le nombre de lignes vides est-il constant ?

**Action** : Le filtrage des lignes vides est déjà prévu dans le parser. La surveillance sert uniquement à confirmer s'il s'agit d'un bug récurrent ou ponctuel.

---

### PV6 : Absence de crate Rust existante

**Priorité** : 🟢 Basse (à reconfirmer au lancement)  
**Méthode** : Recherche sur crates.io et GitHub au moment du lancement du projet  
**Ce qu'il faut vérifier** :
- Aucun crate `bdpm`, `bdpm-parser`, ou similaire n'a été publié
- Aucun projet Rust de parsing BDPM n'est apparu entre-temps

**Action** : Si un crate existe au lancement, évaluer s'il est utilisable comme base ou s'il faut repartir de zéro.

---

## 8.4 Matrice risque/probabilité/impact

| Risque | Probabilité | Impact | Score | Mitigation prioritaire |
|--------|------------|--------|-------|----------------------|
| R1 : Changement de schéma | Moyenne | Critique | 🔴 | Journaliser les écarts, schema_version |
| R2 : Fichier disparu | Faible | Critique | 🟡 | Alerte HTTP 404, vérification 11/11 |
| R3 : Encodage changeant | Faible | Important | 🟡 | Détecteur de secours avec fallback |
| R4 : Rate-limiting | Faible | Important | 🟢 | Intervalle minimum, backoff |
| R5 : Perte historique | Élevée | Important | 🟡 | Soft delete, archivage systématique |

---

## 8.5 Décisions d'architecture à prendre tôt

### D1 : Comment stocker les données historiques ?

**Options** :
1. **Soft delete** (`_is_active = 0`) : Simple, les données restent dans la même table
2. **Table d'historique** (shadow table `_history`) : Séparation propre, mais complexité accrue
3. **Base horodatée** (une base SQLite par mois) : Isolation complète, mais requêtes multi-périodes difficiles

**Recommandation** : Option 1 (soft delete) pour sa simplicité. La colonne `_is_active` est suffisante pour la majorité des cas d'usage.

### D2 : Faut-il normaliser les smart quotes ?

**Options** :
1. **Normaliser** (U+2019 → U+0027) : Cohérence en base, recherches textuelles fiables
2. **Conserver** (U+2019 tel quel) : Fidélité à la source, mais recherches possiblement cassées
3. **Les deux** (stocker la valeur brute + valeur normalisée) : Redondant mais sûr

**Recommandation** : Option 1 (normaliser) avec conservation de la valeur brute dans une colonne `_raw` pour les quelques cas où la fidélité exacte est nécessaire.

### D3 : Comment gérer les références orphelines ?

**Options** :
1. **Flag is_orphan** : Insérer avec un flag, pas de contrainte FK
2. **Table d'orphelins** : Séparer les orphelins dans une table dédiée
3. **Rejeter** : Ne pas insérer les orphelins (perte de données)

**Recommandation** : Option 1 (flag is_orphan). Les orphelins représentent 15-23% des données HAS/GENER et contiennent des informations précieuses.

### D4 : Comment gérer le contenu HTML de CIS_InfoImportantes ?

**Options** :
1. **Stocker tel quel** : HTML brut dans une colonne TEXT
2. **Extraire texte + URL** : Deux colonnes propres, HTML brute optionnel
3. **Parser complet** : Extraire tous les liens, structurer en JSON

**Recommandation** : Option 2 (texte + URL). L'analyse montre que le HTML est principalement des liens `<a>` simples. L'extraction est fiable et la base est plus facile à requêter.

### D5 : Quelle stratégie de mise à jour pour la base ?

**Options** :
1. **Reconstruction complète** : DROP + CREATE à chaque import
2. **Upsert incrémental** : INSERT ON CONFLICT UPDATE
3. **Diff + patch** : Comparer les lignes une par une et n'appliquer que les différences

**Recommandation** : Option 2 (upsert incrémental) avec soft delete pour les absents. C'est le meilleur compromis entre performance et conservation des données.

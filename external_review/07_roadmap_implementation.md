# 07 — Roadmap d'implémentation

---

## 7.1 Phases de mise en œuvre

L'implémentation suit un ordre précis qui minimise les risques et maximise la valeur délivrée à chaque étape. Chaque phase produit un artefact utilisable indépendamment.

```
Phase 1 : Foundation ────> Phase 2 : Parsing ────> Phase 3 : SQLite ────> Phase 4 : Validation
                                                                         │
                                                                         v
                                                          Phase 5 : Incremental ────> Phase 6 : API (futur)
```

---

### Phase 1 : Foundation (1-2 semaines)

**Objectif** : Mettre en place la structure du projet, les types de base et le téléchargement fiable.

**Crate** : `bdpm-core` + `bdpm-fetch`

**Livrable** : Un binaire CLI qui télécharge et archive les 11 fichiers avec manifeste JSON.

**Tâches** :
- [ ] Initialiser le workspace Cargo avec les 5 crates
- [ ] Implémenter `bdpm-core` : types d'énumération (strum), FileConfig, DateFormat, FileEncoding
- [ ] Implémenter `bdpm-fetch` : client HTTP (reqwest), calcul SHA-256, archivage horodaté
- [ ] Gérer les particularités : URL différente pour InfoImportantes, Content-Disposition dynamique
- [ ] Implémenter le mode de vérification SHA-256 (comparaison avec import précédent)
- [ ] Ajouter le User-Agent explicite et l'intervalle entre les requêtes
- [ ] Tests unitaires : mock HTTP, vérification SHA-256, archivage

**Critère de succès** : Le binaire télécharge les 11 fichiers, calcule leurs hashes, les archive avec un manifeste JSON, et détecte les changements d'un import à l'autre.

---

### Phase 2 : Parsing (2-3 semaines)

**Objectif** : Parser les 11 fichiers avec gestion correcte des encodages et normalisation.

**Crate** : `bdpm-parse`

**Livrable** : Un binaire CLI qui parse les 11 fichiers et exporte en JSON valide.

**Tâches** :
- [ ] Implémenter le décodage cp1252/latin-1/utf-8 avec encoding_rs
- [ ] Implémenter la normalisation des fins de ligne (strip \r)
- [ ] Implémenter le split tabulation avec validation du nombre de colonnes
- [ ] Implémenter la normalisation des dates (DD/MM/YYYY et YYYYMMDD → ISO 8601)
- [ ] Implémenter la conversion des nombres décimaux français (virgule → point)
- [ ] Implémenter la normalisation des apostrophes (smart quotes → apostrophe droite)
- [ ] Implémenter le parsing HTML pour CIS_InfoImportantes (extraction URL/texte)
- [ ] Implémenter le split des champs multi-valeurs (séparateur `;`)
- [ ] Tester chaque fichier individuellement avec les données réelles

**Ordre de parsing recommandé** (du plus simple au plus complexe) :
1. `HAS_LiensPageCT_bdpm.txt` (2 colonnes, ASCII pur)
2. `CIS_MITM.txt` (4 colonnes, cp1252 simple)
3. `CIS_CPD_bdpm.txt` (2 colonnes, cp1252 + anomalie \r\r\n)
4. `CIS_InfoImportantes.txt` (4 colonnes, UTF-8 + HTML)
5. `CIS_GENER_bdpm.txt` (5 colonnes, cp1252)
6. `CIS_HAS_SMR_bdpm.txt` (6 colonnes, cp1252 + date YYYYMMDD)
7. `CIS_HAS_ASMR_bdpm.txt` (6 colonnes, cp1252 + date YYYYMMDD)
8. `CIS_COMPO_bdpm.txt` (8 colonnes, cp1252)
9. `CIS_CIP_Dispo_Spec.txt` (8 colonnes, latin-1)
10. `CIS_bdpm.txt` (12 colonnes, cp1252)
11. `CIS_CIP_bdpm.txt` (13 colonnes, UTF-8 + virgules décimales)

**Critère de succès** : Les 11 fichiers sont parsés sans erreur, les 163 451 lignes sont converties en structures Rust typées, et l'export JSON est valide.

---

### Phase 3 : Base SQLite (1-2 semaines)

**Objectif** : Créer la base SQLite complète avec les 13 tables et les logs d'import.

**Crate** : `bdpm-db`

**Livrable** : Une base SQLite complète et requêtable avec les 11 tables + import_log.

**Tâches** :
- [ ] Définir les migrations SQL avec refinery (V001__initial_schema.sql)
- [ ] Implémenter l'ouverture de base avec configuration des PRAGMA (WAL, synchronous, cache)
- [ ] Implémenter l'insertion batch (transactions de 1000 lignes)
- [ ] Implémenter import_log (traçabilité de chaque import)
- [ ] Implémenter la création des index (après l'insertion pour la performance)
- [ ] Tester avec les données parsées de la phase 2
- [ ] Vérifier les counts de lignes par table vs sources

**Critère de succès** : La base SQLite contient toutes les données des 11 fichiers, les index sont créés, et les requêtes de base fonctionnent (recherche par CIS, par dénomination, jointures).

---

### Phase 4 : Validation (1-2 semaines)

**Objectif** : Ajouter les checks de qualité automatisés post-import.

**Crate** : `bdpm-validate`

**Livrable** : Un rapport de validation CLI après chaque import.

**Tâches** :
- [ ] Implémenter les checks de complétude (counts de lignes)
- [ ] Implémenter les checks d'intégrité référentielle (orphelins CIS)
- [ ] Implémenter les checks de validité des énumérations
- [ ] Implémenter les checks de cohérence temporelle (dates)
- [ ] Implémenter la détection de régression (comparaison avec import précédent)
- [ ] Générer un rapport de validation structuré (JSON + CLI)
- [ ] Intégrer les checks dans le pipeline principal

**Critère de succès** : Chaque import produit un rapport de validation détaillé avec statut pass/warn/fail par check.

---

### Phase 5 : Import incrémental (2 semaines)

**Objectif** : Mettre à jour la base sans reconstruction totale.

**Crate** : Intégration dans `bdpm-db`

**Livrable** : Un pipeline complet qui met à jour la base incrémentalement.

**Tâches** :
- [ ] Implémenter le calcul de hash par enregistrement pour la détection de modifications
- [ ] Implémenter l'upsert (INSERT ON CONFLICT UPDATE)
- [ ] Implémenter le soft delete (marquage _is_active = 0 pour les enregistrements absents)
- [ ] Implémenter la planification hebdomadaire + quotidienne
- [ ] Ajouter les notifications (email, webhook) en cas d'alerte
- [ ] Tester le cycle complet : premier import → import incrémental → détection de changement

**Critère de succès** : La base se met à jour en détectant uniquement les fichiers modifiés, en insérant les nouveaux enregistrements, en mettant à jour les modifiés, et en marquant les absents.

---

### Phase 6 : API (futur, 3-4 semaines)

**Objectif** : Exposer la base SQLite via une API REST/GraphQL.

**Technologies** : actix-web ou axum, avec recherche full-text SQLite (FTS5).

**Tâches** :
- [ ] Définir l'API REST (endpoints, filtres, pagination)
- [ ] Implémenter la recherche full-text avec SQLite FTS5
- [ ] Ajouter la documentation OpenAPI/Swagger
- [ ] Implémenter le cache (headers HTTP, ETag cette fois générés par l'API)
- [ ] Déployer en conteneur Docker

**Critère de succès** : L'API permet de rechercher des médicaments, consulter les détails, et filtrer par statut/date/ Substance.

---

## 7.2 Dépendances entre phases

```
Phase 1 ──────> Phase 2 ──────> Phase 3 ──────> Phase 5
                                    │                │
                                    v                v
                                Phase 4 ──────> Phase 6
```

- Phase 2 dépend de Phase 1 (les fichiers doivent être téléchargés pour être parsés)
- Phase 3 dépend de Phase 2 (les données doivent être parsées pour être insérées)
- Phase 4 peut commencer dès que Phase 3 est fonctionnelle
- Phase 5 dépend de Phase 3 et bénéficie de Phase 4
- Phase 6 est indépendante mais bénéficie de toutes les phases précédentes

---

## 7.3 Jalons clés

| Jalon | Phase | Signification |
|-------|-------|---------------|
| 🏗️ **M1** | Fin Phase 1 | Fichiers BDPM téléchargés et archivés automatiquement |
| 📋 **M2** | Fin Phase 2 | Les 11 fichiers sont parsés en structures Rust typées |
| 🗄️ **M3** | Fin Phase 3 | **Base SQLite exploitable** (requêtable par des outils tiers) |
| ✅ **M4** | Fin Phase 4 | Rapport de validation automatisé après chaque import |
| 🔄 **M5** | Fin Phase 5 | Pipeline incrémental complet en production |
| 🌐 **M6** | Fin Phase 6 | API REST documentée et déployée |

**Le jalon M3 est le plus important** : il marque la disponibilité d'une base SQLite exploitable par des outils tiers (DB Browser for SQLite, scripts Python, etc.), même sans le reste du pipeline.

---

## 7.4 Estimation totale

| Phase | Durée estimée |
|-------|---------------|
| Phase 1 : Foundation | 1-2 semaines |
| Phase 2 : Parsing | 2-3 semaines |
| Phase 3 : SQLite | 1-2 semaines |
| Phase 4 : Validation | 1-2 semaines |
| Phase 5 : Incremental | 2 semaines |
| Phase 6 : API | 3-4 semaines |
| **Total** | **10-15 semaines** |

Le noyau fonctionnel (phases 1 à 3) représente **4-7 semaines**. La base SQLite est exploitable dès la fin de la phase 3.

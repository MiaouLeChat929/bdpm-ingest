# Index de la Documentation Technique BDPM

> Base de Données Publique des Médicaments — Projet d'import Rust
> Date de création : 26 mai 2026

---

## Documents disponibles

| # | Document | Description |
|---|----------|-------------|
| 01 | [Inventaire des Fichiers](01_inventaire_des_fichiers.md) | Catalogue complet des 11 fichiers de données BDPM, leurs URLs, structures, champs, types et métadonnées |
| 02 | [Analyse d'Encodage et Quirks](02_analyse_encodage.md) | Analyse approfondie des problèmes d'encodage (CP1252/UTF-8), fins de ligne, trailing tabs, dates, HTML et autres incohérences |
| 03 | [Schéma SQLite](03_schema_sqlite.md) | Conception complète du schéma de base de données SQLite avec tables, index, vues et requêtes utilitaires |
| 04 | [Stratégie de Mise à Jour](04_strategie_mise_a_jour.md) | Mécanismes de détection de changements, politique anti-spam, planning de vérification et gestion du fichier dynamique |
| 05 | [Pipeline de Transformation](05_pipeline_transformation.md) | Description détaillée du pipeline : Fetch → Decode → Parse → Transform → Load avec code Rust |
| 06 | [Architecture Rust](06_architecture_rust.md) | Structure du projet Rust, stack technique, conception modulaire, CLI, tests et plan de développement |
| 07 | [Intégrité et Qualité des Données](07_integrite_donnees.md) | Analyse d'intégrité référentielle, orphelins, doublons, validité des dates, anomalies et recommandations |
| 08 | [APIs et Projets Communautaires](08_apis_communautaires.md) | Inventaire des APIs existantes, projets open source, sources complémentaires et recommandations d'intégration |

---

## Résumé des findings clés

### Encodage
- **9 fichiers sur 10 sont en Windows-1252 (CP1252)**, seulement CIS_CIP_bdpm.txt est en UTF-8
- Le byte `\x92` (apostrophe française) est omniprésent (52 000+ occurrences au total)
- Stratégie : UTF-8 d'abord, fallback CP1252

### Structure
- **Aucune ligne malformée** dans aucun fichier — la structure TSV est parfaitement respectée
- Le Code CIS est la clé primaire centrale, reliant tous les fichiers entre eux
- 145 000+ enregistrements au total, ~22 Mo de données brutes

### Intégrité
- **5 388 CIS codes orphelins** dans les fichiers HAS et GENER (spécialités anciennes non couvertes par le fichier central)
- 4 CIS codes orphelins dans les présentations (désynchronisation temporelle)
- Aucun doublon sur les clés primaires naturelles

### Mise à jour
- **Aucun mécanisme de notification** (pas de RSS, API, ETag, Last-Modified)
- Cycle principalement mensuel, mais CIS_CIP_bdpm et Dispo_Spec plus fréquents
- Stratégie : scraping de la date + hash SHA-256

### Format
- Deux formats de date : DD/MM/YYYY et YYYYMMDD
- HTML embarqué dans 4 849 lignes (champs libellés SMR/ASMR et indications CIP)
- Taux de remboursement inconsistants (avec/sans espace)
- Trailing tabs dans 96% des lignes de CIS_CIP_bdpm.txt

---

## Prochaines étapes

1. Valider ce rapport avec l'équipe
2. Initialiser le projet Rust (`cargo init bdpm-importer`)
3. Implémenter le module de décodage (Phase 1)
4. Implémenter le module de parsing TSV (Phase 1)
5. Créer le schéma SQLite et les migrations (Phase 2)
6. Implémenter le pipeline complet (Phase 3)
7. Ajouter la détection automatique de mises à jour (Phase 4)

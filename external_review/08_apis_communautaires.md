# APIs et Projets Communautaires — Écosystème BDPM

> Inventaire des APIs, projets open source et ressources communautaires liées à la BDPM.
> Date : 26 mai 2026

---

## 1. APIs existantes

### 1.1 API Médicaments FR (api-medicaments.fr)

| Propriété | Valeur |
|-----------|--------|
| **URL** | https://api-medicaments.fr |
| **GitHub** | https://github.com/giygas/medicaments-api |
| **Type** | API REST |
| **Statut** | Actif |
| **Fréquence de mise à jour** | 2 fois par jour (6h et 18h) |
| **Langage** | Non spécifié |
| **Licence** | Non spécifiée |

**Endpoints disponibles :**

| Endpoint | Description |
|----------|-------------|
| `GET /v1/medicaments?search={q}` | Recherche de médicaments |
| `GET /v1/medicaments/{cis}` | Détail d'un médicament par code CIS |
| `GET /v1/generiques?libelle={nom}` | Recherche de génériques |
| `GET /v1/presentations/{cip}` | Détail d'une présentation par code CIP |
| `GET /v1/medicaments/export` | Export complet de la base (~20 Mo) |

**Caractéristiques :**
- 15 800+ médicaments référencés
- Recherche par code CIS, CIP, nom, statut, taux de remboursement, prix, forme, composition
- Rate limiting : 1 000 tokens/IP, recharge de 3 tokens/seconde
- Gratuit : 100 requêtes/jour
- Payant : 19,90 €/mois (3 000 req/jour), 69,90 €/mois (50K+ req/jour)
- Référencé sur data.gouv.fr comme réutilisation officielle

**Utilité pour notre projet :**
- Peut servir de **signal de changement** : si leur API a des données plus récentes, la BDPM a été mise à jour
- Le endpoint `/v1/medicaments/export` permet de télécharger la base complète en JSON
- Ne remplace pas le téléchargement direct des fichiers source (risque de transformation/perte)

---

### 1.2 API GraphQL BDPM (axel-op)

| Propriété | Valeur |
|-----------|--------|
| **URL** | https://api-bdpm-graphql.axel-op.fr/graphql |
| **GitHub** | https://github.com/axel-op/api-bdpm-graphql |
| **Type** | API GraphQL |
| **Statut** | ⚠️ 503 Service Unavailable (mai 2026) |
| **Langage** | JavaScript/Node.js |

**Caractéristiques :**
- Parse les fichiers plats BDPM et les sert via GraphQL
- Permet des requêtes complexes avec résolution de relations
- Actuellement indisponible

**Utilité pour notre projet :**
- Le code source GitHub peut servir de référence pour le parsing
- L'approche GraphQL est intéressante pour une API future

---

## 2. Projets open source

### 2.1 betagouv/api-medicaments

| Propriété | Valeur |
|-----------|--------|
| **GitHub** | https://github.com/betagouv/api-medicaments |
| **Organisation** | betagouv (incubateur de services numériques) |
| **Type** | API REST + parsing BDPM |
| **Statut** | Projet historique |
| **CI/CD** | CircleCI |

**Caractéristiques :**
- Projet officiel de l'incubateur gouvernemental français
- Parsing des fichiers BDPM + API REST
- Couverture de tests

**Utilité :** Référence pour le design d'API et les patterns de parsing

---

### 2.2 betagouv/infomedicament

| Propriété | Valeur |
|-----------|--------|
| **GitHub** | https://github.com/betagouv/infomedicament |
| **Type** | Application web |
| **Statut** | Projet historique |

**Caractéristiques :**
- Application de recherche rapide de médicaments
- Vérification de disponibilité
- Utilise les données BDPM

---

### 2.3 scossin/FrenchSPC

| Propriété | Valeur |
|-----------|--------|
| **GitHub** | https://github.com/scossin/FrenchSPC |
| **Type** | Outil d'extraction RCP |

**Caractéristiques :**
- Télécharge les Résumés des Caractéristiques du Produit (RCP/SmPC)
- Extraction en HTML ou PDF depuis le site BDPM
- Complémentaire aux données tabulaires

**Utilité pour notre projet :** Extension future pour enrichir la base avec les RCP

---

### 2.4 NCBO BioPortal — BDPM Ontology

| Propriété | Valeur |
|-----------|--------|
| **URL** | https://bioportal.bioontology.org/ontologies/BDPM |
| **Type** | Ontologie / Knowledge Graph |
| **Version** | 2025-03-04 |

**Caractéristiques :**
- Représentation ontologique de la BDPM
- Endpoint SPARQL pour des requêtes sémantiques
- API REST pour l'accès programmatique à l'ontologie

**Utilité pour notre projet :** Référence pour la modélisation des relations entre entités médicamenteuses

---

## 3. Sources de données complémentaires

### 3.1 data.gouv.fr

| Source | URL | Statut |
|--------|-----|--------|
| BDPM sur data.gouv.fr | https://www.data.gouv.fr/datasets/base-de-donnees-publique-des-medicaments/ | ⚠️ Abandonné depuis 2014 |

Le jeu de données sur data.gouv.fr est obsolète. La fréquence est marquée comme "punctual" et la dernière mise à jour date de janvier 2014. **Ne pas utiliser cette source.**

### 3.2 Sources complémentaires potentielles

| Source | URL | Données | Licence |
|--------|-----|---------|---------|
| ANSM | https://ansm.sante.fr/ | Pharmacovigilance, ruptures de stock | Licence Ouverte |
| HAS | https://www.has-sante.fr/ | Avis CT, recommandations | Licence Ouverte |
| Ameli | https://www.ameli.fr/ | Tarifs, remboursements | Données publiques |
| Thériaque | https://www.theriaque.org/ | Données médicamenteuses complètes | Sur abonnement |
| Vidal | https://www.vidal.fr/ | Base de données complète | Sur abonnement |
| OpenMedic | https://www.data.gouv.fr/datasets/open-medic/ | Consommation médicaments | Licence Ouverte |
| ATC/WHO | https://www.whocc.no/atc_ddd_index/ | Classification ATC | Libre avec attribution |

---

## 4. Comparaison des approches de parsing

### 4.1 Approches observées dans les projets communautaires

| Projet | Langage | Stratégie d'encodage | Stratégie de parsing |
|--------|---------|---------------------|---------------------|
| api-medicaments | Non spécifié | Non documentée | Non documentée |
| api-bdpm-graphql | Node.js | Non documentée | Parsing ligne par ligne |
| api-medicaments (betagouv) | Ruby/Node | Non documentée | Parsing CSV avec header mapping |
| Notre projet | Rust | **UTF-8 d'abord, CP1252 en fallback** | **TSV avec validation de schéma** |

### 4.2 Avantages de notre approche

1. **Gestion explicite de l'encodage** : La plupart des projets existants ne documentent pas leur stratégie d'encodage, ce qui suggère qu'ils traitent probablement tout en UTF-8 (risque de corruption pour 9 fichiers sur 10)
2. **Validation de schéma** : Chaque fichier a un nombre de champs attendu, validé à chaque ligne
3. **Double stockage HTML** : Conservation du brut + version nettoyée, ce qu'aucun projet ne fait
4. **Traçabilité** : Enregistrement du hash SHA-256, de l'encodage détecté et des statistiques d'import
5. **Détection proactive de changements** : Hash comparison au lieu de re-télécharger aveuglément

---

## 5. Recommandations pour l'intégration

### 5.1 Court terme

- Utiliser l'**API-Medicaments.fr** comme signal auxiliaire de mise à jour (vérifier si leur base est plus récente)
- Étudier le code de **api-bdpm-graphql** pour les patterns de parsing GraphQL
- Considérer l'ajout d'un endpoint de type "health check" qui compare notre base avec l'API publique

### 5.2 Moyen terme

- Développer une **API REST locale** inspirée d'api-medicaments.fr mais avec des données plus complètes (incluant les orphelins HAS)
- Ajouter un **endpoint GraphQL** pour des requêtes complexes
- Intégrer les **données ATC** de la WHO pour enrichir les classifications

### 5.3 Long terme

- Enrichir la base avec les **RCP** (via FrenchSPC ou scraping)
- Intégrer les données de **consommation** (OpenMedic) pour des analyses épidémiologiques
- Créer un **modèle de données ontologique** inspiré du BioPortal BDPM

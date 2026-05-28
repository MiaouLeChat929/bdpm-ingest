//! HTTP integration tests that spin up the axum server and hit all endpoints.
//!
//! Tests the full HTTP layer: routing, serialization, status codes, response shapes.

use std::path::PathBuf;

/// Create a temporary SQLite database with minimal test data.
fn create_test_db() -> PathBuf {
    use rusqlite::Connection;

    let db_path = std::env::temp_dir().join(format!("test_server_{}_{}.db", std::process::id(), rand::random::<u32>()));

    // Remove any existing file from previous runs
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));

    // Create a minimal database with the proper schema
    let conn = Connection::open(&db_path).expect("Failed to open test DB");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA synchronous=NORMAL;"
    ).expect("Failed to set PRAGMA");

    // Run just the first migration (001_initial.sql) which creates all tables
    let schema_001 = include_str!("../src/db/migrations/001_initial.sql");
    conn.execute_batch(schema_001).expect("Failed to create tables from migration 001");

    // Run migration 003 (remove mitm FK)
    let schema_003 = include_str!("../src/db/migrations/003_remove_mitm_fk.sql");
    conn.execute_batch(schema_003).expect("Failed to apply migration 003");

    // Run migration 005 (safety_alerts)
    let schema_005 = include_str!("../src/db/migrations/005_safety_alerts.sql");
    conn.execute_batch(schema_005).expect("Failed to apply migration 005");

    // Create FTS5 virtual table for search (standalone, no content=)
    conn.execute_batch(r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS drugs_fts USING fts5(
            cis,
            name_raw,
            name,
            atc_code,
            form,
            lab_name,
            substance_name
        );
    "#).expect("Failed to create FTS table");

    // Populate FTS from drugs (must come after drug INSERTs)
    // Done after all test data is inserted below

    // Insert test data
    conn.execute(
        "INSERT OR REPLACE INTO drugs (cis, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, generic_type, atc_code)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "60004971",
            "DOLIPRANE 1000 mg, comprimé",
            "comprimé",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "1998-03-12",
            "SANOFI",
            0,
            "reference",
            "N02BE01"
        ]
    ).expect("Failed to insert test drug 1");

    conn.execute(
        "INSERT OR REPLACE INTO drugs (cis, name, form, route, auth_status, procedure_type, comm_status, auth_date, lab_name, is_patent, generic_type, atc_code)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "60004972",
            "EFFERALGAN 1000 mg, poudre",
            "poudre",
            "orale",
            "Autorisation active",
            "Procédure nationale",
            "Commercialisée",
            "2005-06-15",
            "UPSA",
            0,
            "generic",
            "N02BE01"
        ]
    ).expect("Failed to insert test drug 2");

    conn.execute(
        "INSERT OR REPLACE INTO presentations (cis, cip, cip_raw, labels, pres_status, comm_status, comm_date, ean13, reimbursable, reimb_rate, prix_ht_cents, prix_ville_cents, prix_rate_cents)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "60004971",
            "340093",
            "3400930000017",
            "16 comprimés",
            "Commercialisée",
            "Commercialisée",
            "1998-03-12",
            "3400930000017",
            "oui",
            0.65,
            2434,
            2560,
            100
        ]
    ).expect("Failed to insert presentation 1");

    conn.execute(
        "INSERT OR REPLACE INTO presentations (cis, cip, cip_raw, labels, pres_status, comm_status, comm_date, ean13, reimbursable, reimb_rate, prix_ht_cents, prix_ville_cents, prix_rate_cents)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "60004971",
            "340094",
            "3400940000018",
            "32 comprimés",
            "Commercialisée",
            "Commercialisée",
            "1998-03-12",
            "3400940000018",
            "oui",
            0.65,
            4800,
            5000,
            100
        ]
    ).expect("Failed to insert presentation 2");

    conn.execute(
        "INSERT OR REPLACE INTO compositions (cis, substance_code, substance_name, dosage, pharm_code, seq)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "60004971",
            "42215",
            "Paracétamol",
            "1000 mg",
            "SA",
            1
        ]
    ).expect("Failed to insert composition");

    conn.execute(
        "INSERT OR REPLACE INTO mitm (cis, atc_code, detail_url)
         VALUES (?, ?, ?)",
        rusqlite::params![
            "60004971",
            "N02BE01",
            "https://base-donnees-publique.medicaments.gouv.fr/affichageDoc.php?specid=60004971"
        ]
    ).expect("Failed to insert mitm");

    conn.execute(
        "INSERT OR REPLACE INTO atc_codes (atc_code, parent_5_char, parent_3_char, parent_1_char)
         VALUES (?, ?, ?, ?)",
        rusqlite::params!["N02BE01", "N02BE", "N02B", "N"]
    ).expect("Failed to insert ATC N02BE01");

    conn.execute(
        "INSERT OR REPLACE INTO atc_codes (atc_code, parent_5_char, parent_3_char, parent_1_char)
         VALUES (?, ?, ?, ?)",
        rusqlite::params!["N02BE", "N02B", "N02", "N"]
    ).expect("Failed to insert ATC N02BE");

    conn.execute(
        "INSERT OR REPLACE INTO atc_codes (atc_code, parent_5_char, parent_3_char, parent_1_char)
         VALUES (?, ?, ?, ?)",
        rusqlite::params!["N02B", None::<String>, "N02", "N"]
    ).expect("Failed to insert ATC N02B");

    conn.execute(
        "INSERT OR REPLACE INTO atc_codes (atc_code, parent_5_char, parent_3_char, parent_1_char)
         VALUES (?, ?, ?, ?)",
        rusqlite::params!["N02", None::<String>, "N02", "N"]
    ).expect("Failed to insert ATC N02");

    conn.execute(
        "INSERT OR REPLACE INTO atc_codes (atc_code, parent_5_char, parent_3_char, parent_1_char)
         VALUES (?, ?, ?, ?)",
        rusqlite::params!["N", None::<String>, None::<String>, "N"]
    ).expect("Failed to insert ATC N");

    conn.execute(
        "INSERT OR REPLACE INTO generic_groups (group_id, group_name, cis, type, sort_order, is_orphan)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["GRP001", "Paracétamol 1000mg", "60004971", "reference", 1, 0]
    ).expect("Failed to insert generic group 1");

    conn.execute(
        "INSERT OR REPLACE INTO generic_groups (group_id, group_name, cis, type, sort_order, is_orphan)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params!["GRP001", "Paracétamol 1000mg", "60004972", "generic", 2, 0]
    ).expect("Failed to insert generic group 2");

    conn.execute(
        "INSERT OR REPLACE INTO import_log (file_name, file_hash, file_size, row_count, status, bad_rows, skipped_rows, duration_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params!["test", "abc123", 100, 2, "success", 0, 0, 100]
    ).expect("Failed to insert import log");

    // Populate FTS from drugs with substance names from compositions
    conn.execute_batch(
        "INSERT INTO drugs_fts(cis, name_raw, name, atc_code, form, lab_name, substance_name)
         SELECT d.cis, d.name, d.name, d.atc_code, d.form, d.lab_name,
                COALESCE((SELECT GROUP_CONCAT(substance_name, ' ') FROM compositions WHERE cis = d.cis), '')
         FROM drugs d;"
    ).expect("Failed to populate FTS table");

    conn.close().expect("Failed to close test DB connection");
    db_path
}

/// Start test server and get the base URL.
async fn run_test_server(db_path: PathBuf) -> (tokio::task::JoinHandle<()>, String) {
    use tokio::net::TcpListener;

    let app = bdpm_ingest::build_app(db_path);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get local addr");
    let base_url = format!("http://{}", addr);

    let server = async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            })
            .await
            .ok();
    };

    let handle = tokio::spawn(server);
    (handle, base_url)
}

/// Helper to clean up the test database file.
fn cleanup_db(db_path: &PathBuf) {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
}

mod integration {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/health", base_url);
        let resp = reqwest::get(&url).await.expect("Health request failed");

        assert_eq!(resp.status(), 200, "Health endpoint should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse health response");
        assert_eq!(body["status"], "ok", "Health status should be 'ok'");
        assert!(body["drug_count"].as_i64().is_some(), "Should have drug_count");

        // Cleanup
        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_search_drugs() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs?q=DOLIPRANE", base_url);
        let resp = reqwest::get(&url).await.expect("Search request failed");

        assert_eq!(resp.status(), 200, "Search endpoint should return 200");

        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse search response");
        assert!(!body.is_empty(), "Search should return results for DOLIPRANE");
        assert_eq!(body[0]["cis"], "60004971", "First result should be DOLIPRANE");
        assert!(body[0]["name"].as_str().unwrap().contains("DOLIPRANE"), "Should contain drug name");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_search_drugs_empty() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs?q=XYZNONEXISTENT", base_url);
        let resp = reqwest::get(&url).await.expect("Search request failed");

        assert_eq!(resp.status(), 200, "Empty search should return 200");
        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert!(body.is_empty(), "Non-existent drug should return empty array");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_search_drugs_limit() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs?q=DOLIPRANE&limit=1", base_url);
        let resp = reqwest::get(&url).await.expect("Search with limit failed");

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert!(body.len() <= 1, "Should respect limit parameter");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_drug_detail() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs/60004971", base_url);
        let resp = reqwest::get(&url).await.expect("Drug detail request failed");

        assert_eq!(resp.status(), 200, "Drug detail should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse drug detail");
        assert_eq!(body["cis"], "60004971", "Should return correct CIS");
        assert!(body["name"].as_str().unwrap().contains("DOLIPRANE"), "Should include drug name");
        assert!(body["presentations"].is_array(), "Should include presentations array");
        assert!(body["presentations"].as_array().unwrap().len() > 0, "Should have at least one presentation");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_drug_detail_404() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs/99999999", base_url);
        let resp = reqwest::get(&url).await.expect("Drug detail request failed");

        assert_eq!(resp.status(), 404, "Non-existent drug should return 404");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_drug_safety() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs/60004971/safety", base_url);
        let resp = reqwest::get(&url).await.expect("Safety request failed");

        assert_eq!(resp.status(), 200, "Safety endpoint should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse safety response");
        assert_eq!(body["cis"], "60004971", "Should include CIS");
        assert!(body["data_available"].is_boolean(), "Should have data_available boolean");
        // No safety_alerts data in test DB, so data_available should be false
        assert_eq!(body["data_available"], false, "No safety alerts in test data");
        assert!(body["alerts"].is_array(), "Should have alerts array");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_drug_detail_returns_atc_code() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs/60004971", base_url);
        let resp = reqwest::get(&url).await.expect("Drug detail request failed");

        assert_eq!(resp.status(), 200, "Drug detail should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse drug detail");
        assert_eq!(body["atc_code"], "N02BE01", "Should return correct ATC code");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_drug_safety_404() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs/99999999/safety", base_url);
        let resp = reqwest::get(&url).await.expect("Safety request failed");

        assert_eq!(resp.status(), 404, "Non-existent drug safety should return 404");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_atc_top_level() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/atc", base_url);
        let resp = reqwest::get(&url).await.expect("ATC top-level request failed");

        assert_eq!(resp.status(), 200, "ATC top-level should return 200");

        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse ATC response");
        assert!(!body.is_empty(), "Should have at least one top-level ATC code");
        // 'N' is our test top-level code
        let has_n = body.iter().any(|v| v["atc_code"] == "N");
        assert!(has_n, "Should have ATC code 'N' in response");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_atc_detail() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/atc/N02BE01", base_url);
        let resp = reqwest::get(&url).await.expect("ATC detail request failed");

        assert_eq!(resp.status(), 200, "ATC detail should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse ATC detail");
        assert_eq!(body["atc_code"], "N02BE01", "Should return correct ATC code");
        assert!(body["children"].is_array(), "Should have children array");
        assert!(body["drugs_count"].is_number(), "Should have drugs_count");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_atc_not_found() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        // ATC code 'Z' doesn't exist in our test data
        let url = format!("{}/atc/Z", base_url);
        let resp = reqwest::get(&url).await.expect("ATC request failed");

        // Should still return 200 with empty children (graceful fallback)
        assert_eq!(resp.status(), 200, "Unknown ATC should return 200 with empty data");
        let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
        assert!(body["children"].as_array().unwrap().is_empty(), "Unknown ATC should have empty children");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_generic_groups() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/generic-groups", base_url);
        let resp = reqwest::get(&url).await.expect("Generic groups request failed");

        assert_eq!(resp.status(), 200, "Generic groups should return 200");

        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert!(!body.is_empty(), "Should have at least one generic group");
        let has_grp001 = body.iter().any(|v| v["group_id"] == "GRP001");
        assert!(has_grp001, "Should have group GRP001");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_generic_group_detail() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/generic-groups/GRP001", base_url);
        let resp = reqwest::get(&url).await.expect("Generic group detail request failed");

        assert_eq!(resp.status(), 200, "Generic group detail should return 200");

        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert_eq!(body.len(), 2, "GRP001 should have 2 members");
        assert!(body.iter().any(|v| v["cis"] == "60004971"), "Should include DOLIPRANE");
        assert!(body.iter().any(|v| v["cis"] == "60004972"), "Should include EFFERALGAN");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_generic_group_not_found() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/generic-groups/NONEXISTENT", base_url);
        let resp = reqwest::get(&url).await.expect("Generic group request failed");

        assert_eq!(resp.status(), 200, "Non-existent group should return empty array");
        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert!(body.is_empty(), "Non-existent group should return empty array");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_availability() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/availability", base_url);
        let resp = reqwest::get(&url).await.expect("Availability request failed");

        assert_eq!(resp.status(), 200, "Availability should return 200");

        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        // No availability data in test DB, should return empty array
        assert!(body.is_empty(), "Test DB should have no availability data");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_openapi_json() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/openapi.json", base_url);
        let resp = reqwest::get(&url).await.expect("OpenAPI JSON request failed");

        assert_eq!(resp.status(), 200, "OpenAPI JSON should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse OpenAPI JSON");
        assert!(body["openapi"].is_string(), "Should have openapi field");
        assert!(body["info"]["title"].is_string(), "Should have info.title");
        assert!(body["paths"].is_object(), "Should have paths object");

        // Verify key paths exist
        let paths = body["paths"].as_object().unwrap();
        assert!(paths.contains_key("/health"), "Should have /health path");
        assert!(paths.contains_key("/drugs"), "Should have /drugs path");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_openapi_yaml() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/openapi.yaml", base_url);
        let resp = reqwest::get(&url).await.expect("OpenAPI YAML request failed");

        assert_eq!(resp.status(), 200, "OpenAPI YAML should return 200");
        let content_type = resp.headers().get("content-type").map(|v| v.to_str().unwrap()).unwrap_or("");
        assert!(content_type.contains("yaml") || content_type.contains("x-yaml"), "Should be YAML content type");

        let body = resp.text().await.expect("Failed to read OpenAPI YAML");
        assert!(body.contains("openapi:"), "Should contain openapi: field");
        assert!(body.contains("paths:"), "Should contain paths section");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_availability_with_cis_filter() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        // Add availability data
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "INSERT INTO availability (cis, cip, status_type, status, date_start) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params!["60004971", "340093", 1, "Rupture de stock", "2026-05-01"]
            ).unwrap();
        }

        let url = format!("{}/availability?cis=60004971", base_url);
        let resp = reqwest::get(&url).await.expect("Availability filtered request failed");

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        assert!(!body.is_empty(), "Should have availability data for CIS 60004971");
        assert_eq!(body[0]["cis"], "60004971");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_atc_detail_with_drugs() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/atc/N02BE01", base_url);
        let resp = reqwest::get(&url).await.expect("ATC detail request failed");

        assert_eq!(resp.status(), 200, "ATC detail should return 200");

        let body: serde_json::Value = resp.json().await.expect("Failed to parse ATC detail");
        assert_eq!(body["atc_code"], "N02BE01", "Should return correct ATC code");
        assert!(body["drugs_count"].as_i64().unwrap_or(0) >= 1, "Should have at least one drug with this ATC code");

        server.abort();
        cleanup_db(&db_path);
    }

    #[tokio::test]
    async fn test_search_with_empty_query() {
        let db_path = create_test_db();
        let (server, base_url) = run_test_server(db_path.clone()).await;

        let url = format!("{}/drugs?q=", base_url);
        let resp = reqwest::get(&url).await.expect("Search with empty query failed");

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = resp.json().await.expect("Failed to parse response");
        // Empty query should return empty array
        assert!(body.is_empty(), "Empty query should return empty results");

        server.abort();
        cleanup_db(&db_path);
    }
}
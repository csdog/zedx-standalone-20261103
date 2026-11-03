use super::*;
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("zedx-db-{}.sqlite", uuid::Uuid::new_v4()));
        let connection = Connection::open(path.to_str().unwrap(), true).unwrap();
        connection.exec("CREATE TABLE data(id INTEGER, value TEXT, payload BLOB); INSERT INTO data VALUES(1, NULL, x'CAFE'),(2, 'hello', x'')").unwrap()().unwrap();
        Self(path)
    }
    fn query(&self, sql: &str) -> Result<QueryResult> {
        query(&self.0, sql, Arc::new(AtomicBool::new(false)))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(&self.0).unwrap();
    }
}
#[test]
fn displays_schema_values_null_and_blob() {
    let fixture = Fixture::new();
    let result = fixture
        .query("SELECT * FROM data ORDER BY id; -- comment")
        .unwrap();
    assert_eq!(result.columns, ["id", "value", "payload"]);
    assert_eq!(result.rows[0], ["1", "NULL", "[BLOB: 2 bytes]"]);
    assert!(!result.truncated);
    assert_eq!(
        fixture
            .query("SELECT name FROM sqlite_schema WHERE type='table'")
            .unwrap()
            .rows[0][0],
        "data"
    );
}
#[test]
fn rejects_writes_attach_and_multiple_statements_without_mutation() {
    let fixture = Fixture::new();
    for sql in [
        "DELETE FROM data",
        "DROP TABLE data",
        "ATTACH ':memory:' AS other",
        "PRAGMA user_version=2",
        "SELECT 1; DELETE FROM data",
        "SELECT 1; SELECT 2",
        "",
        "-- only comment",
    ] {
        assert!(fixture.query(sql).is_err(), "{sql}");
    }
    assert_eq!(
        fixture.query("SELECT COUNT(*) FROM data").unwrap().rows[0][0],
        "2"
    );
}
#[test]
fn bounds_recursive_queries_rows_and_cells() {
    let fixture = Fixture::new();
    let result = fixture.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<500) SELECT x FROM n").unwrap();
    assert_eq!(result.rows.len(), MAX_ROWS);
    assert!(result.truncated);
    let result = fixture.query("SELECT printf('%2000s', 'x')").unwrap();
    assert!(result.rows[0][0].len() <= MAX_CELL_BYTES + '…'.len_utf8());
}
#[test]
fn cancellation_and_missing_database_are_explicit() {
    let fixture = Fixture::new();
    assert!(query(&fixture.0, "SELECT 1", Arc::new(AtomicBool::new(true))).is_err());
    let missing = fixture.0.with_extension("missing");
    assert!(query(&missing, "SELECT 1", Arc::new(AtomicBool::new(false))).is_err());
    assert!(!missing.exists());
}
#[test]
fn quotes_table_names_as_identifiers() {
    assert_eq!(
        table_query("a\"; DROP TABLE data; --"),
        "SELECT * FROM \"a\"\"; DROP TABLE data; --\""
    );
}

#[test]
fn progress_budget_interrupts_unbounded_queries() {
    let fixture = Fixture::new();
    let result = fixture.query(
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT MAX(x) FROM n",
    );
    assert!(result.unwrap_err().to_string().contains("limit"));
}

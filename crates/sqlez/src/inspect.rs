//! Bounded, read-only inspection of user databases, separate from application storage.
use crate::connection::Connection;
use anyhow::{Context as _, Result, bail, ensure};
use libsqlite3_sys::*;
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    path::Path,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_ROWS: usize = 256;
const MAX_COLUMNS: usize = 64;
const MAX_CELL_BYTES: usize = 512;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

pub fn table_query(name: &str) -> String {
    format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""))
}

/// Queries never open a missing file, attach other databases, or change database contents.
pub fn query(path: &Path, sql: &str, cancel: Arc<AtomicBool>) -> Result<QueryResult> {
    let connection = Connection::open_read_only(path)?;
    // Bound allocations made inside SQLite before cell formatting sees the values.
    unsafe {
        sqlite3_limit(connection.sqlite3, SQLITE_LIMIT_LENGTH, 4 * 1024 * 1024);
        sqlite3_limit(connection.sqlite3, SQLITE_LIMIT_SQL_LENGTH, 64 * 1024);
    }
    inspect(&connection, sql, cancel)
}

struct Budget {
    deadline: Instant,
    cancel: Arc<AtomicBool>,
}
unsafe extern "C" fn progress(context: *mut c_void) -> c_int {
    // SAFETY: inspect keeps Budget alive and clears the callback before dropping it.
    let budget = unsafe { &*context.cast::<Budget>() };
    c_int::from(budget.cancel.load(Ordering::Relaxed) || Instant::now() >= budget.deadline)
}
unsafe extern "C" fn authorize(
    _: *mut c_void,
    action: c_int,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    match action {
        SQLITE_SELECT | SQLITE_READ | SQLITE_FUNCTION | SQLITE_RECURSIVE => SQLITE_OK,
        _ => SQLITE_DENY,
    }
}
struct Inspection<'a> {
    connection: &'a Connection,
    statement: *mut sqlite3_stmt,
}
impl Drop for Inspection<'_> {
    fn drop(&mut self) {
        // SAFETY: this guard uniquely owns the statement; the connection outlives it.
        unsafe {
            sqlite3_finalize(self.statement);
            sqlite3_progress_handler(self.connection.sqlite3, 0, None, ptr::null_mut());
            sqlite3_set_authorizer(self.connection.sqlite3, None, ptr::null_mut());
        }
    }
}

fn inspect(connection: &Connection, sql: &str, cancel: Arc<AtomicBool>) -> Result<QueryResult> {
    ensure!(!cancel.load(Ordering::Relaxed), "Query cancelled");
    let sql = CString::new(sql).context("SQL contains a NUL byte")?;
    let mut budget = Budget {
        deadline: Instant::now() + Duration::from_secs(2),
        cancel,
    };
    let mut inspection = Inspection {
        connection,
        statement: ptr::null_mut(),
    };
    let mut tail = ptr::null();
    // SAFETY: SQL is NUL-terminated, out pointers are valid, and the callback budget
    // outlives inspection. Inspection always finalizes and unregisters callbacks.
    unsafe {
        sqlite3_set_authorizer(connection.sqlite3, Some(authorize), ptr::null_mut());
        sqlite3_progress_handler(
            connection.sqlite3,
            1000,
            Some(progress),
            (&mut budget as *mut Budget).cast(),
        );
        let code = sqlite3_prepare_v2(
            connection.sqlite3,
            sql.as_ptr(),
            -1,
            &mut inspection.statement,
            &mut tail,
        );
        ensure!(code == SQLITE_OK, "{}", error_message(connection));
        ensure!(!inspection.statement.is_null(), "Enter a SELECT query");
        // Prepare the tail separately: comments and trailing semicolons are allowed,
        // while every additional executable statement is rejected before stepping.
        while !tail.is_null() && !CStr::from_ptr(tail).to_bytes().is_empty() {
            let mut extra = ptr::null_mut();
            let mut next = ptr::null();
            let code = sqlite3_prepare_v2(connection.sqlite3, tail, -1, &mut extra, &mut next);
            let has_extra = !extra.is_null();
            sqlite3_finalize(extra);
            ensure!(
                code == SQLITE_OK && !has_extra,
                "Run one SELECT statement at a time"
            );
            if next == tail {
                break;
            }
            tail = next;
        }
        ensure!(
            sqlite3_stmt_readonly(inspection.statement) != 0,
            "Only read-only queries are supported"
        );
        let count = usize::try_from(sqlite3_column_count(inspection.statement))?;
        ensure!(
            count <= MAX_COLUMNS,
            "Results are limited to {MAX_COLUMNS} columns"
        );
        let mut columns = Vec::with_capacity(count);
        for column in 0..count {
            let name = sqlite3_column_name(inspection.statement, column as c_int);
            ensure!(!name.is_null(), "Could not read result column names");
            columns.push(CStr::from_ptr(name).to_string_lossy().into_owned());
        }
        let mut result = QueryResult {
            columns,
            ..Default::default()
        };
        loop {
            ensure!(!budget.cancel.load(Ordering::Relaxed), "Query cancelled");
            match sqlite3_step(inspection.statement) {
                SQLITE_DONE => break,
                SQLITE_ROW => {
                    if result.rows.len() == MAX_ROWS {
                        result.truncated = true;
                        break;
                    }
                    result.rows.push(
                        (0..count)
                            .map(|column| cell(inspection.statement, column as c_int))
                            .collect(),
                    );
                }
                SQLITE_INTERRUPT => bail!("Query cancelled or exceeded the 2 second limit"),
                _ => bail!("{}", error_message(connection)),
            }
        }
        Ok(result)
    }
}

unsafe fn cell(statement: *mut sqlite3_stmt, column: c_int) -> String {
    // SAFETY: callers supply a live statement positioned on a row and a valid column.
    unsafe {
        match sqlite3_column_type(statement, column) {
            SQLITE_NULL => "NULL".into(),
            SQLITE_BLOB => format!("[BLOB: {} bytes]", sqlite3_column_bytes(statement, column)),
            _ => {
                let text = sqlite3_column_text(statement, column);
                if text.is_null() {
                    return String::new();
                }
                let length = sqlite3_column_bytes(statement, column).max(0) as usize;
                let bytes = std::slice::from_raw_parts(text, length.min(MAX_CELL_BYTES));
                let mut value = String::from_utf8_lossy(bytes).into_owned();
                if length > MAX_CELL_BYTES {
                    value.push('…');
                }
                value
            }
        }
    }
}
fn error_message(connection: &Connection) -> String {
    // SAFETY: the connection owns a valid SQLite handle throughout this call.
    unsafe {
        CStr::from_ptr(sqlite3_errmsg(connection.sqlite3))
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;

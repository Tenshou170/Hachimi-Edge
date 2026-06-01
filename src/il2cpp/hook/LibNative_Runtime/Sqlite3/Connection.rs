use std::sync::Mutex;

use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use sqlparser::{
    ast::BinaryOperator,
    dialect::SQLiteDialect,
    keywords::Keyword,
    parser::Parser
};

use crate::il2cpp::{
    api::{il2cpp_object_new, il2cpp_runtime_object_init},
    ext::Il2CppStringExt,
    sql::{self, ExprExt, SelectExt, SelectItemExt},
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = std::ptr::null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn new() -> *mut Il2CppObject {
    let object = il2cpp_object_new(class());
    il2cpp_runtime_object_init(object);
    object
}

pub static SELECT_QUERIES: Lazy<Mutex<FnvHashMap<usize, Box<dyn sql::SelectQueryState + Send + Sync>>>> =
    Lazy::new(|| Mutex::new(FnvHashMap::default()));

/// Cache of SQL string → parsed query template (table name + column/param layout).
/// Avoids re-running sqlparser on every repeated query call. The same handful of
/// SELECT statements are issued thousands of times per session.
///
/// The value is a factory closure that produces a fresh `SelectQueryState` box,
/// because each live query object needs its own mutable state (bound params etc.).
type QueryFactory = Box<dyn Fn() -> Box<dyn sql::SelectQueryState + Send + Sync> + Send + Sync>;
static QUERY_TEMPLATE_CACHE: Lazy<Mutex<FnvHashMap<String, QueryFactory>>> =
    Lazy::new(|| Mutex::new(FnvHashMap::default()));

#[inline(never)]
fn parse_query(query: *mut Il2CppObject, sql: *const Il2CppString) {
    let sql_str = unsafe { (*sql).as_utf16str() }.to_string();

    // quick escape!!!11
    if !sql_str.starts_with("SELECT") {
        return;
    }

    // Check the template cache first — avoid re-parsing the same SQL string.
    {
        let cache = QUERY_TEMPLATE_CACHE.lock().unwrap();
        if let Some(factory) = cache.get(&sql_str) {
            SELECT_QUERIES.lock().unwrap().insert(query as usize, factory());
            return;
        }
    }

    // parse the sql string
    let dialect = SQLiteDialect {};
    let parser_res = Parser::new(&dialect).try_with_sql(&sql_str);

    if let Ok(mut parser) = parser_res {
        // only care about select statements
        if !parser.parse_keyword(Keyword::SELECT) {
            return;
        }
        let Ok(select) = parser.parse_select() else {
            return;
        };

        // and their first table name (SELECT FROM table_name)
        let Some(table_name) = select.get_first_table_name() else {
            debug!("no table name");
            return;
        };

        // Collect column and param metadata from the parsed AST so we can
        // replay it cheaply for every future occurrence of this SQL string.
        let table_name = table_name.clone();

        // Gather column names in order
        let mut columns: Vec<String> = Vec::new();
        for item in select.projection.iter() {
            if let Some(name) = item.get_unnamed_expr_ident() {
                columns.push(name.clone());
            }
        }

        // Gather param names in order
        let mut params: Vec<String> = Vec::new();
        if let Some(selection) = select.selection {
            for expr in selection.binary_op_iter() {
                if *expr.op != BinaryOperator::Eq { continue; }
                if let Some(name) = expr.left.get_ident_value() {
                    if expr.right.is_placeholder_value() {
                        params.push(name.clone());
                    }
                }
            }
        }

        // Only cache tables we actually handle.
        if !matches!(table_name.as_ref(), "text_data" | "character_system_text" | "race_jikkyo_comment" | "race_jikkyo_message") {
            return;
        }

        // Build a factory closure that replays the metadata onto a fresh state.
        let factory: QueryFactory = Box::new(move || {
            let mut state: Box<dyn sql::SelectQueryState + Send + Sync> = match table_name.as_ref() {
                "text_data" => Box::new(sql::TextDataQuery::default()),
                "character_system_text" => Box::new(sql::CharacterSystemTextQuery::default()),
                "race_jikkyo_comment" => Box::new(sql::RaceJikkyoCommentQuery::default()),
                "race_jikkyo_message" => Box::new(sql::RaceJikkyoMessageQuery::default()),
                _ => unreachable!(),
            };
            for (i, col) in columns.iter().enumerate() {
                state.add_column(i as i32, col);
            }
            for (i, param) in params.iter().enumerate() {
                state.add_param(i as i32 + 1, param);
            }
            state
        });

        let query_state = factory();
        QUERY_TEMPLATE_CACHE.lock().unwrap().insert(sql_str, factory);
        SELECT_QUERIES.lock().unwrap().insert(query as usize, query_state);
    }
}

type QueryFn = extern "C" fn(this: *mut Il2CppObject, sql: *const Il2CppString) -> *mut Il2CppObject;
pub extern "C" fn Query(this: *mut Il2CppObject, sql: *const Il2CppString) -> *mut Il2CppObject {
    trace!("Query");
    let query = get_orig_fn!(Query, QueryFn)(this, sql);
    parse_query(query, sql);
    query
}

type PreparedQueryFn = extern "C" fn(this: *mut Il2CppObject, sql: *const Il2CppString) -> *mut Il2CppObject;
extern "C" fn PreparedQuery(this: *mut Il2CppObject, sql: *const Il2CppString) -> *mut Il2CppObject {
    trace!("PreparedQuery");
    let query = get_orig_fn!(PreparedQuery, PreparedQueryFn)(this, sql);
    parse_query(query, sql);
    query
}

static mut OPEN_ADDR: usize = 0;
impl_addr_wrapper_fn!(Open, OPEN_ADDR, bool,
    this: *mut Il2CppObject, fileName: *mut Il2CppString, vfsName: *mut Il2CppString, key: *mut Il2CppArray, cipherType: i32
);

static mut CLOSEDB_ADDR: usize = 0;
impl_addr_wrapper_fn!(CloseDB, CLOSEDB_ADDR, (), this: *mut Il2CppObject);

pub fn init(LibNative_Runtime: *const Il2CppImage) {
    get_class_or_return!(LibNative_Runtime, "LibNative.Sqlite3", Connection);

    let Query_addr = get_method_addr(Connection, c"Query", 1);
    let PreparedQuery_addr = get_method_addr(Connection, c"PreparedQuery", 1);

    new_hook!(Query_addr, Query);
    new_hook!(PreparedQuery_addr, PreparedQuery);

    unsafe {
        CLASS = Connection;
        OPEN_ADDR = get_method_addr(Connection, c"Open", 4);
        CLOSEDB_ADDR = get_method_addr(Connection, c"CloseDB", 0);
    }
}
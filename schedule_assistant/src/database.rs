use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_rusqlite::{from_rows, from_row};

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

pub fn connection() -> rusqlite::Result<Connection> {
    let path = "./assistant.db3";
    Connection::open(&path)
}

pub fn create_table(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "CREATE TABLE person (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL,
                  data            BLOB
                  )",
        params![],
    )
}

pub fn insert(conn: &Connection) -> rusqlite::Result<usize> {
    let me = Person {
        id: 0,
        name: "Steven".to_string(),
        data: None,
    };
    conn.execute(
        "INSERT INTO person (name, data) VALUES (?1, ?2)",
        params![me.name, me.data],
    )
}

pub fn query(conn: &Connection) -> rusqlite::Result<Vec<Person>> {
    let mut stmt = conn.prepare("SELECT id, name, data FROM person").unwrap();
    let person_iter = from_rows::<Person>(stmt.query(rusqlite::NO_PARAMS).unwrap());
    let people = person_iter.collect::<Result<Vec<_>,_>>().unwrap();
    println!("got customers: {:?}", people);
    Ok(people)
}

pub fn query_with_map(conn: &Connection) -> rusqlite::Result<Vec<Person>> {
    let mut stmt = conn.prepare("SELECT id, name, data FROM person").unwrap();
    let rows = stmt.query_and_then(rusqlite::NO_PARAMS, from_row::<Person>).unwrap();
    let people = rows.collect::<Result<Vec<_>,_>>().unwrap();
    println!("got customers: {:?}", people);
    Ok(people)
}

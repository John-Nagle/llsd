//  
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
use std::collections::HashMap;
//
//  The main primitive data item
//
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Boolean(bool),
    Real(f64),
    UUID([u8;16]),
    String(String),
    Date(i64),
    URI(String),
    Binary(Vec<u8>),
    Map(HashMap<String,JsonValue>),
    Array(Vec<JsonValue>),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

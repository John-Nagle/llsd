//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
//
//  Modules
//
pub mod xml;
//
use std::collections::HashMap;
//
///  The main primitive LLSD data item
#[derive(Debug, Clone, PartialEq)]
pub enum LLSDValue {
    Null,
    Boolean(bool),
    Real(f64),
    Integer(i32),
    UUID([u8; 16]),
    String(String),
    Date(i64),
    URI(String),
    Binary(Vec<u8>),
    Map(HashMap<String, LLSDValue>),
    Array(Vec<LLSDValue>),
}

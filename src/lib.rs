//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  Format documentation is at http://wiki.secondlife.com/wiki/LLSD
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
//
//  Modules
//
pub mod binary;
pub mod xml;
//
use std::collections::HashMap;
use uuid;
//
///  The primitive LLSD data item.
#[derive(Debug, Clone, PartialEq)]
pub enum LLSDValue {
    Undefined,
    Boolean(bool),
    Real(f64),
    Integer(i32),
    UUID(uuid::Uuid),
    String(String),
    Date(i64),
    URI(String),
    Binary(Vec<u8>),
    Map(HashMap<String, LLSDValue>),
    Array(Vec<LLSDValue>),
}

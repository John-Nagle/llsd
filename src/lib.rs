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
use anyhow::{anyhow, Error};
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

//  Implementation

impl LLSDValue {

    /// Parse LLSD, detecting format.
    pub fn parse(msg: &[u8]) -> Result<LLSDValue, Error> {
        //  Try binary first
        if msg.len() >= binary::LLSDBINARYPREFIX.len() &&
            &msg[0..binary::LLSDBINARYPREFIX.len()] == binary::LLSDBINARYPREFIX {
                return binary::parse(msg) }
        //  Not binary, must be some text format.
        let msgstring = std::str::from_utf8(msg)?; // convert to UTF-8 string
        if msgstring.starts_with(xml::LLSDXMLSENTINEL) { // try XML
            return xml::parse(msgstring) }  
        //  "Notation" syntax is not currently supported. 
        let snippet = &msgstring[0..usize::min(60,msgstring.len())]; // beginning of malformed LLSD      
        Err(anyhow!("LLSD format not recognized: {:?}", snippet))
    }
}

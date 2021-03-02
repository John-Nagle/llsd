//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  XML format.
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
use super::LLSDValue;
use anyhow::{anyhow, Error};
use quick_xml::events::Event;
use quick_xml::events::attributes::Attributes;  
use quick_xml::Reader;
use std::collections::HashMap;
use uuid;
use hex;
use base64;
use ascii85;

///    Parse LLSD expressed in XML into an LLSD tree.
pub fn parse(xmlstr: &str) -> Result<LLSDValue, Error> {
    let mut reader = Reader::from_str(xmlstr);
    reader.trim_text(true); // do not want trailing blanks
    reader.expand_empty_elements(true); // want end tag events always
    let mut txt = Vec::new();
    let mut buf = Vec::new();
    //  Outer parse. Find <llsd> and parse its interior.
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"llsd" => {
                        let mut buf = Vec::new();
                        match reader.read_event(&mut buf) {
                            Ok(Event::Start(ref e)) => {
                                let tagname = std::str::from_utf8(e.name())?; // tag name as string to start parse
                                let v = parse_value(&mut reader, tagname, &e.attributes())?; // parse next value
                                return Ok(v); // return key value pair
                            }
                            _ => {
                                return Err(anyhow!(
                                    "Expected LLSD data, found {:?} error at position {}",
                                    e.name(),
                                    reader.buffer_position()
                                ))
                            }
                        };
                    }
                    _ => {
                        return Err(anyhow!(
                            "Expected <llsd>, found {:?} error at position {}",
                            e.name(),
                            reader.buffer_position()
                        ))
                    }
                }
            }
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => println!("End <{:?}>", std::str::from_utf8(e.name())),
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => {
                return Err(anyhow!(
                    "Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear()
    }
    return Err(anyhow!("Unexpected end of data"));
}

/// Parse one value - real, integer, map, etc. Recursive.
fn parse_value(reader: &mut Reader<&[u8]>, starttag: &str, attrs: &Attributes) -> Result<LLSDValue, Error> {
    //  Entered with a start tag alread parsed and in starttag
    match starttag {
        "null" | "real" | "integer" | "bool" | "string" | "uri" | "binary" | "uuid" | "date" => {
            parse_primitive_value(reader, starttag, attrs)
        }
        "map" => parse_map(reader),
        "array" => parse_array(reader),
        _ => Err(anyhow!(
            "Unknown data type <{}> at position {}",
            starttag,
            reader.buffer_position()
        )),
    }
}

/// Parse one value - real, integer, map, etc. Recursive.
fn parse_primitive_value(reader: &mut Reader<&[u8]>, starttag: &str, attrs: &Attributes) -> Result<LLSDValue, Error> {
    //  Entered with a start tag already parsed and in starttag
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                println!("End <{:?}>", tagname);
                if starttag != tagname {
                    return Err(anyhow!(
                        "Unmatched XML tags: <{}> .. <{}>",
                        starttag,
                        tagname
                    ));
                };
                //  End of an XML tag. Value is in text.
                let text = texts.join(" "); // combine into one big string
                texts.clear();
                //   TODO: 
                //  1. Allow numeric values in "bool" fields.
                //  2. Parse ISO dates.
                //  3. Parse base64 for "binary".                                                                    
                //  Parse the primitive types.
                return match starttag {
                    "null" => Ok(LLSDValue::Null),
                    "real" => Ok(LLSDValue::Real(
                        if text.to_lowercase() == "nan" {
                            "NaN".to_string()
                        } else {
                            text
                        }
                        .parse::<f64>()?,
                    )),
                    "integer" => Ok(LLSDValue::Integer(text.parse::<i32>()?)),
                    "bool" => Ok(LLSDValue::Boolean(text.parse::<bool>()?)),
                    "string" => Ok(LLSDValue::String(text.trim().to_string())),
                    "uri" => Ok(LLSDValue::String(text.trim().to_string())),
                    "uuid" => Ok(LLSDValue::UUID(
                        *uuid::Uuid::parse_str(text.trim())?.as_bytes())),
                    "date" => Ok(LLSDValue::Date(parse_date(&text)?)),
                    "binary" => Ok(LLSDValue::Binary(parse_binary(&text, attrs)?)),
                    _ => Err(anyhow!(
                        "Unexpected primitive data type <{}> at position {}",
                        starttag,
                        reader.buffer_position()
                    )),
                };
            },
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data at position {}",
                    reader.buffer_position()
                ))
            },
            Ok(Event::Comment(_)) => {},    // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            },
            _ => {
                return Err(anyhow!(
                    "Unexpected value parse error at position {} while parsing: {:?}",
                    reader.buffer_position(),
                    starttag
                ))
            }
        }
    }
}

//  Parse one map.
fn parse_map(reader: &mut Reader<&[u8]>) -> Result<LLSDValue, Error> {
    //  Entered with a "map" start tag just parsed.
    println!("Entering parse_map");
    let mut map: HashMap<String, LLSDValue> = HashMap::new(); // accumulating map
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                match tagname {
                    "key" => {
                        let (k, v) = parse_map_entry(reader)?; // read one key/value pair
                        let _dup = map.insert(k, v); // insert into map
                                                     //  Duplicates are not errors, per LLSD spec.
                    }
                    _ => {
                        return Err(anyhow!("Expected 'key' in map, found '{}'", tagname));
                    }
                }
            },
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                //  End of an XML tag. No text expected.
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                println!("End <{:?}>", tagname);
                if "map" != tagname {
                    return Err(anyhow!("Unmatched XML tags: <{}> .. <{}>", "map", tagname));
                };
                return Ok(LLSDValue::Map(map)); // done, valid result
            },
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data at position {}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Comment(_)) => {},    // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            },
            _ => {
                return Err(anyhow!(
                    "Unexpected parse error at position {} while parsing a map",
                    reader.buffer_position()
                ))
            }
        }
    }
}

//  Parse one map entry.
//  Format <key> STRING> </key> LLSDVALUE
fn parse_map_entry(reader: &mut Reader<&[u8]>) -> Result<(String, LLSDValue), Error> {
    //  Entered with a "key" start tag just parsed.  Expecting text.
    println!("Entering parse_map_entry");
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                return Err(anyhow!("Expected 'key' in map, found '{}'", tagname));
            }
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                //  End of an XML tag. Should be </key>
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                println!("End <{:?}>", tagname);
                if "key" != tagname {
                    return Err(anyhow!("Unmatched XML tags: <{}> .. <{}>", "key", tagname));
                };
                let mut buf = Vec::new();
                let k = texts.join(" ").trim().to_string(); // the key
                match reader.read_event(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        let tagname = std::str::from_utf8(e.name())?; // tag name as string
                        let v = parse_value(reader, tagname, &e.attributes())?; // parse next value
                        return Ok((k, v)); // return key value pair
                    }
                    _ => {
                        return Err(anyhow!(
                            "Unexpected parse error at position {} while parsing map entry",
                            reader.buffer_position()
                        ))
                    }
                };
            },
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data at position {}",
                    reader.buffer_position()
                ))
            },
            Ok(Event::Comment(_)) => {},    // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            },
            _ => {
                return Err(anyhow!(
                    "Unexpected parse error at position {} while parsing a map entry",
                    reader.buffer_position()
                ))
            }
        }
    }
}

/// Parse one LLSD object. Recursive.
fn parse_array(reader: &mut Reader<&[u8]>) -> Result<LLSDValue, Error> {
    //  Entered with an <array> tag just parsed.
    Err(anyhow!("Unimplemented"))
}

/// Parse binary object.
/// Input in base64.
fn parse_binary(s: &str, attrs: &Attributes) -> Result<Vec::<u8>, Error> {
    // "Parsers must support base64 encoding. Parsers may support base16 and base85."
    let encoding = match get_attr(attrs, b"encoding")? {
        Some(enc) => enc,
        None => "base64".to_string()    // default
    };
    //  Decode appropriately.
    Ok(match encoding.as_str() {
        "base64" => base64::decode(s)?,
        "base16" => hex::decode(s)?,
        "base85" => match ascii85::decode(s) {Ok(v) => v, Err(e) => return Err(anyhow!("Base 85 decode error: {:?}",e))},
        _ => return Err(anyhow!("Unknown binary encoding: {}", encoding))
    })
}

/// Parse ISO 9660 date, simple form.
fn parse_date(s: &str) -> Result<i64, Error> {
    Err(anyhow!("Unimplemented"))
}

/// Search for attribute in attribute list
fn get_attr<'a>(attrs: &'a Attributes, key: &[u8]) -> Result<Option<String>,Error> {
    //  Each step has a possible error, so it's hard to do this more cleanly.
    for attr in attrs.clone() {
        let a = attr?;
        if a.key == key { continue } // not this one           
        let v = a.unescaped_value()?;
        let sv = std::str::from_utf8(&v)?;
        return Ok(Some(sv.to_string()))                   
    }
    Ok(None)
}

/// Prints out the value as an XML string.
pub fn dump(val: &LLSDValue) -> String {
    return "Unimplemented".to_string();
}

/// Pretty prints out the value as XML. Takes an argument that's
/// the number of spaces to indent new blocks.
pub fn pretty(val: &LLSDValue, spaces: u16) -> String {
    return "Unimplemented".to_string();
}

// Unit tests

#[test]
fn xmlparsetest1() {
    const TESTXML1: &str = r#""
<?xml version="1.0" encoding="UTF-8"?>
<llsd>
<map>
  <key>region_id</key>
    <uuid>67153d5b-3659-afb4-8510-adda2c034649</uuid>
  <key>scale</key>
    <string>one minute</string>
  <key>simulator statistics</key>
  <map>
    <key>time dilation</key><real>0.9878624</real>
    <key>sim fps</key><real>44.38898</real>
    <key>pysics fps</key><real>44.38906</real>
    <key>agent updates per second</key><real>nan</real>
    <key>lsl instructions per second</key><real>0</real>
    <key>total task count</key><real>4</real>
    <key>active task count</key><real>0</real>
    <key>active script count</key><real>4</real>
    <key>main agent count</key><real>0</real>
    <key>child agent count</key><real>0</real>
    <key>inbound packets per second</key><real>1.228283</real>
    <key>outbound packets per second</key><real>1.277508</real>
    <key>pending downloads</key><real>0</real>
    <key>pending uploads</key><real>0.0001096525</real>
    <key>frame ms</key><real>0.7757886</real>
    <key>net ms</key><real>0.3152919</real>
    <key>sim other ms</key><real>0.1826937</real>
    <key>sim physics ms</key><real>0.04323055</real>
    <key>agent ms</key><real>0.01599029</real>
    <key>image ms</key><real>0.01865955</real>
    <key>script ms</key><real>0.1338836</real>
  </map>
</map>
</llsd>
""#;

    let result = parse(TESTXML1);
    println!("Parse of {:?}: \n{:#?}", TESTXML1, result);
}

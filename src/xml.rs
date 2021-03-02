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
use quick_xml::events::{Event, BytesEnd, BytesStart};
use quick_xml::events::attributes::Attributes;  
use quick_xml::{Reader, Writer, escape};
use std::collections::HashMap;
use std::io::Cursor;
use std::io::Write;
use uuid;
use hex;
use base64;
use ascii85;
use chrono;
use chrono::TimeZone;

///    Parse LLSD expressed in XML into an LLSD tree.
pub fn parse(xmlstr: &str) -> Result<LLSDValue, Error> {
    let mut reader = Reader::from_str(xmlstr);
    reader.trim_text(true); // do not want trailing blanks
    reader.expand_empty_elements(true); // want end tag events always
    let mut buf = Vec::new();
    let mut output: Option<LLSDValue> = None;
    //  Outer parse. Find <llsd> and parse its interior.
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"llsd" => {
                        if output.is_some() { return Err(anyhow!("More than one <llsd> block in data")) }
                        let mut buf = Vec::new();
                        match reader.read_event(&mut buf) {
                            Ok(Event::Start(ref e)) => {
                                let tagname = std::str::from_utf8(e.name())?; // tag name as string to start parse
                                output = Some(parse_value(&mut reader, tagname, &e.attributes())?); // parse next value
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
            Ok(Event::Text(e)) => (), // Don't actually need random text
            Ok(Event::End(ref e)) => (), // Tag matching check is automatic.
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
    //  Final result, if stored
    match output {
        Some(out) => Ok(out),
        None => Err(anyhow!("Unexpected end of data, no <llsd> block.")) 
    } 
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
                        uuid::Uuid::parse_str(text.trim())?)),
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
                    "Unexpected end of data in primitive value at position {}",
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
                if "map" != tagname {
                    return Err(anyhow!("Unmatched XML tags: <{}> .. <{}>", "map", tagname));
                };
                return Ok(LLSDValue::Map(map)); // done, valid result
            },
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data in map at position {}",
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
/// Input in base64, base16, or base85.
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
        _ => return Err(anyhow!("Unknown encoding: <binary encoding=\"{}\">", encoding))
    })
}

/// Parse ISO 9660 date, simple form.
fn parse_date(s: &str) -> Result<i64, Error> {
    Ok(chrono::DateTime::parse_from_rfc3339(s)?.timestamp())
}

/// Search for attribute in attribute list
fn get_attr<'a>(attrs: &'a Attributes, key: &[u8]) -> Result<Option<String>,Error> {
    //  Each step has a possible error, so it's hard to do this more cleanly.
    for attr in attrs.clone() {
        let a = attr?;
        if a.key != key { continue } // not this one           
        let v = a.unescaped_value()?;
        let sv = std::str::from_utf8(&v)?;
        return Ok(Some(sv.to_string()))                   
    }
    Ok(None)
}

/// Prints out the value as an XML string.
pub fn dump(val: &LLSDValue) -> Result<Vec<u8>, Error> {
    pretty(val, 0)
}

/// Pretty prints out the value as XML. Takes an argument that's
/// the number of spaces to indent new blocks.
pub fn pretty(val: &LLSDValue, spaces: usize) -> Result<Vec<u8>,Error> {
    let mut s: Vec::<u8> = Vec::new();
    generate_value(&mut s, val, spaces, 0)?;
    s.flush();
    Ok(s)
}
fn generate_value(s: &mut Vec::<u8>, val: &LLSDValue, spaces: usize, indent: usize) -> Result<(), Error> {
    fn tagvalue(s: &mut Vec::<u8>, tag: &str, text: &str, indent: usize) {
        let _ = write!(*s, "<{}>{}</{}>", tag, xml_escape(text), tag);
    }
    match val {
        LLSDValue::Null => tagvalue(s,"null","",indent),
        LLSDValue::Boolean(v) => tagvalue(s, "boolean", if *v { "true" } else {"false"}, indent),
        LLSDValue::String(v)  => tagvalue(s, "string", v.as_str(), indent),
        LLSDValue::URI(v)  => tagvalue(s, "string", v.as_str(), indent),
        LLSDValue::Integer(v) => tagvalue(s, "integer", v.to_string().as_str(), indent),
        LLSDValue::Real(v)  => tagvalue(s, "real", v.to_string().as_str(), indent),
        LLSDValue::UUID(v) => tagvalue(s, "uuid", v.to_string().as_str(), indent), 
        LLSDValue::Binary(v) => tagvalue(s, "binary", base64::encode(v).as_str(), indent),  
        LLSDValue::Date(v) => tagvalue(s, "date", 
            &chrono::Utc.timestamp(*v,0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true), indent),     
        _ => return Err(anyhow!("Unreachable"))
    };
    Ok(())       
}

/// XML standard character escapes. 
fn xml_escape(unescaped: &str) -> String {
    let mut s = String::new();
    for ch in unescaped.chars() {
        match ch {
            '<' => s += "&lt;",
            '>' => s += "&gt;",
            '\'' => s += "&apos;",
            '&' => s +="&amp;",
            '"' => s +="&quot;",
            _ => s.push(ch)
        }
    }
    s
}
/*
fn generate_value(writer: &mut Writer<std::io::Cursor<Vec<u8>>>, val: &LLSDValue, spaces: usize, indent: usize) -> Result<(),Error> {
    //  Convenience functions
    fn starttag(writer: &mut Writer<std::io::Cursor<Vec<u8>>>, tag: &[u8]) -> Result<(),Error> {
       Ok(writer.write_event(Event::Start(BytesStart::borrowed_name(tag)))?) }
    fn endtag(writer: &mut Writer<std::io::Cursor<Vec<u8>>>, tag: &[u8]) -> Result<(),Error> {
       Ok(writer.write_event(Event::End(BytesEnd::borrowed(tag)))?) }
    match val {
        LLSDValue::Null => {
            ////let mut elem = BytesStart::owned(b"my_elem".to_vec(), "my_elem".len());
            ////let mut elem = BytesStart::borrowed_name(&(b"null")[..]);
            ////let mut elem = BytesStart::borrowed_name(b"null");
            /////writer.write_event(Event::Start(elem))?;
            ////let mut elem = BytesEnd::borrowed(b"null");
            ////writer.write_event(Event::End(elem))?;
            ////writer.write_event(Event::Start(BytesStart::borrowed_name(b"null")))?;
            starttag(writer, b"null");
            endtag(writer, b"null");
            ////writer.write_event(Event::End(BytesEnd::borrowed(b"null")))?;
        },
        
        LLSDValue::Bool => {
            starttag(writer, b"boolean");
            endtag(writer, b"boolean");
            
        
        
        _ => panic!("Unreachable")
        /*
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
        */
    }
    Ok(())
}
*/
        

// Unit tests

#[test]
fn xmlparsetest1() {
    const TESTXML1: &str = r#"
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
    <!-- Comment - some additional test values -->
    <key>hex number</key><binary encoding="base16">0fa1</binary>
    <key>base64 number</key><binary>SGVsbG8gd29ybGQ=</binary>
    <key>date</key><date>2006-02-01T14:29:53.43Z</date>
  </map>
</map>
</llsd>
"#;

    let result = parse(TESTXML1);
    println!("Parse of {:?}: \n{:#?}", TESTXML1, result);
    match result {
        Ok(v) => (),
        Err(e) => panic!("Parse failed: {:?}",e)
    }
        
}

#[test]
fn xmlgeneratetest1() {
    const TESTLLSD1: LLSDValue = 
        LLSDValue::Null;
    let generated = pretty(&TESTLLSD1, 4).unwrap();
    let xmlstr = std::str::from_utf8(&generated).unwrap();
    println!("Generated XML:\n{:?}", xmlstr);
}

# llsd
Linden Lab Structured Data (LLSD) serialization

Rust version.

This is a serialization system used by Second Life and Open Simulator. 
It is documented here: http://wiki.secondlife.com/wiki/LLSD

## Introduction

There are three formats - XML, binary, and "Notation". All store
the same data, which is roughly what JSON can represent.
Parsing and output functions are provided.

## Status

IN PROGRESS - do not use yet.

## Data types

- Boolean - converts to Rust "bool".
- Integer - Rust i32.
- Real - Rust f64
- UUID - Rust [u8;16]
- String - Rust String, Unicode
- Date - "an absolute point in time, UTC, with resolution to the second", as Rust i64.
- URI - Rust String that is a URI
- Binary - Vec<u8>

- A map is a HashMap mapping String keys to LLSD values. 

- An array is a Rust Vec of LLSD values. 

## LLSD values in Rust

These generally follow the conventions of the Rust crate "json".
An LLSD value is a tree.

//!
//! # Overview
//!
//! A command-line tool/library for file mass-renaming using regex or case names.
//!
//! # Example
//!
//! ```bash
//! > ls
//! test-dir
//! ├── Another Dir & Co
//! │   ├── Some [some#bs].txt
//! │   └── Some & Track.txt
//! └── Some Dir
//! ├── SOME_fILe.txt
//! └── some,text_file.txt
//!
//! > rrename
//! > ls
//!
//!```
//!
//! # kudos
//! [wrench](https://github.com/funnyboy-roks/wrench) was taken as a base

pub mod case;
pub mod cli;

pub use cli::Rrename;

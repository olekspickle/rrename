//!
//! # Overview
//!
//! A command-line tool/library for file mass-renaming using regex or case names.
//!
//! # Example
//!
//! ```bash
//! > ls
//!  backup
//!  ├── Another Dir & Co
//!  │   ├── Some [some#bs].txt
//!  │   └── Some & Track.txt
//!  └── Some Dir
//!      ├── SOME_fILe.txt
//!      ├── some, text_file.txt
//!      └── some,text_file.txt
//!
//! > rrename
//! > ls
//!
//!  test-dir
//!  ├── another-dir-and-co
//!  │   ├── some-and-track.txt
//!  │   └── some-[some#bs].txt
//!  └── some-dir
//!      ├── some-file.txt
//!      ├── some-text-file-25057.txt
//!      └── some-text-file-57497.txt
//!
//!```
//!
//! # kudos
//! [wrench](https://github.com/funnyboy-roks/wrench) was taken as a base

pub mod case;
pub mod cli;

pub use cli::Rrename;

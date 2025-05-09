//! A very opinionated command-line tool/library for file mass-renaming using regex
//!
//! # Example
//!
//! ## no arguments run
//! ```bash
//! > ls
//!  mock
//!  ├── Another Dir & Co
//!  │   ├── Some [some#bs].txt
//!  │   └── Some & Track.txt
//!  ├── Some Dir
//!  │   ├── SOME_fILe.txt
//!  │   ├── some, text_file.txt
//!  │   └── some,text_file.txt
//!  └── Some -  Word With III dCi135_
//!      └── Some Word F3500 dCi135 StereoM10.txt
//!
//! > rrename
//! > ls
//!  mock
//!  ├── another-dir-and-co
//!  │   ├── some-and-track.txt
//!  │   └── some-[some#bs].txt
//!  ├── some-dir
//!  │   ├── some-file.txt
//!  │   ├── some-text-file-25057.txt
//!  │   └── some-text-file-57497.txt
//!  └── some-word-with-iii-dci135-
//!      └── some-word-f3500-dci135-stereom10.txt
//!
//! ```
//!
//! ## using regex to substitute
//! ```bash
//! > ls
//! 3pv-some-file.mp4
//! > rrename -E "3pv-" -s ""
//! './3pv-some-file.mp4' -> './some-file.mp4'
//! Renamed: 1, depth:1
//! ```
//!
//! # kudos
//! [wrench](https://github.com/funnyboy-roks/wrench) was taken as a base
//! Powered by [jwalk](https://github.com/byron/jwalk) - walkdir on steroids using rayon

pub mod case;
pub mod cli;

pub use cli::Rrename;

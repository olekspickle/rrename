# rrename

A very opinionated command-line tool/library for file mass-renaming using regex

## Installation

Easiest way is to install via cargo:
```bash
cargo install rrename
```

## Features

- [x] rename files in batch
- [x] restore files after rename in batch
- [x] denoise files
- [x] regex substitution
- [x] verbocity levels ([clap_verbosity_flag] is amazing): just add `-v, -vv, -vvv` to your hearts content
- [ ] case conversion

## Example

### no arguments run
```bash
➜ tree test
test
├── Another Dir & Co
│   ├── Some [some#bs].txt
│   └── Some & Track.txt
├── Some Dir
│   ├── SOME_⧸fILe.txt
│   ├── some,text_file\#_$.txt
│   ├── some：, text_file.txt
│   └── some'weird'file"with"brackets.txt
├── Some -  Word With III dCi135_
│   └── Some Word F3500 dCi135 StereoM10.txt
└── Super
    └── NESTED
        └── UgLy
            └── dIIr
                └── with|file.txt

➜ rrename -d  test -L 5
2026-02-25T12:04:59.756742Z  INFO rrename::cli: Renamed: 15, depth: 5
2026-02-25T12:04:59.756802Z  INFO rrename: renamed list saved as rrenamed-2026-02-25-12:04:59.txt

➜ tree test
test
├── another-dir-and-co
│   ├── some-and-track.txt
│   └── some-[some-bs].txt
├── some-dir
│   ├── some-file.txt
│   ├── some-text-file-.txt
│   ├── some-text-file.txt
│   └── some-weird-file-with-brackets.txt
├── some-word-with-iii-dci135
│   └── some-word-f3500-dci135-stereom10.txt
└── super
    └── nested
        └── ugly
            └── diir
                └── with-file.txt

➜ rrename -r rrenamed-2026-02-25-12:04:59.txt
2026-02-25T12:05:14.740507Z  INFO rrename::cli: Restored: 15
2026-02-25T12:05:14.740569Z  INFO rrename: renamed list saved as rrenamed-2026-02-25-12:05:14.txt

➜ tree test
test
├── Another Dir & Co
│   ├── Some [some#bs].txt
│   └── Some & Track.txt
├── Some Dir
│   ├── SOME_⧸fILe.txt
│   ├── some,text_file\#_$.txt
│   ├── some：, text_file.txt
│   └── some'weird'file"with"brackets.txt
├── Some -  Word With III dCi135_
│   └── Some Word F3500 dCi135 StereoM10.txt
└── Super
    └── NESTED
        └── UgLy
            └── dIIr
                └── with|file.txt
```

### using regex to substitute

By default, if you do not specify -s,--sub, all regex matches will be replaced with empty strings.

```bash
➜  ls
3pv-some-file.mp4
➜   rrename -E "3pv-some-" -s "other-" .
2026-02-25T12:05:14.740507Z  INFO rrename::cli: Renamed: 1
➜   ls
other-file.mp4
```

## kudos
[wrench](https://github.com/funnyboy-roks/wrench) was taken as a base
Powered by [jwalk](https://github.com/byron/jwalk) - walkdir on steroids using rayon

License: MIT OR Apache-2.0

[clap_verbosity_flag]: https://docs.rs/clap-verbosity-flag/latest/clap_verbosity_flag/

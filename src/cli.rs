use crate::case::Case;
use clap::{Parser, crate_authors, crate_name, crate_version};
use jwalk::WalkDir;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use regex::{NoExpand, Regex};
use std::{
    fs,
    path::{Path, PathBuf},
};

const ABOUT: &str = "Rename files in batch.\nExample:\nrrename";

/// Rename files matching a regular expression by replacing parts of their name.
/// Similar to the util-linux `rename` command, but with support of regular expressions.
//#[command(arg_required_else_help = true)]
#[derive(Debug, Default, Clone, Parser)]
#[command(name = crate_name!())]
#[command(bin_name = crate_name!())]
#[command(version = crate_version!(),  author = crate_authors!("\n"), about = ABOUT)]
pub struct Rrename {
    /// Perform no filesystem operations and display to the user the changes that would happen
    /// without the flag
    #[clap(short = 'n', long, conflicts_with = "quiet")]
    pub dry_run: bool,

    /// Don't echo the renames to STDOUT
    #[clap(short, long)]
    pub quiet: bool,

    /// Opposite of verbose, basically - mention every file\dir even if it is not changed
    #[clap(short = 'v', long, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Depth program should go into
    #[clap(short = 'L', long, default_value = "3")]
    pub depth: usize,

    /// Specify case scheme change
    #[clap(short, long, default_value = "kebab-case")]
    pub case: Case,

    /// Rename all files within the directory provided
    #[clap(default_value = ".")]
    pub path: PathBuf,

    /// Recurse into subdirectories
    #[clap(short, long, default_value_t = true)]
    pub recursive: bool,

    /// Prevent Regex parts from being expanded (i.e., `$1`, `$name`)
    #[clap(long)]
    pub no_expand: bool,

    /// Replace only first match
    #[clap(long)]
    pub first: bool,

    /// Replace noisy chars with unix friendly: &,"'_
    #[clap(long, default_value_t = true)]
    pub denoise: bool,

    /// Regex to use to search in the string.
    #[clap(short = 'E', long)]
    pub regex: Option<Regex>,

    /// String to replace matches with.
    /// This will expand `$1` and `$name` into the groups matched by the regex provided.
    /// If this is not the desired behaviour, `--no-expand` should be used.
    #[clap(long, short)]
    pub sub: Option<String>,
}

impl Rrename {
    pub fn new() -> Rrename {
        Rrename::default()
    }

    pub fn with_path(mut self, p: impl Into<PathBuf>) -> Self {
        self.path = p.into();
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn run(&self) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
        let mut renamed = 0;
        let mut rng = SmallRng::seed_from_u64(405);
        let total: Vec<_> = WalkDir::new(&self.path)
            .into_iter()
            .filter_map(Result::ok)
            .collect();
        let mut renames: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(total.len() * 2);

        // Go from topmost entries to lower ones, iteratively breadth-first,
        // because walkdir entry canot be renamed if the parent is a subject to rename
        for d in 0..=self.depth {
            let mut entries: Vec<_> = WalkDir::new(&self.path)
                .min_depth(d)
                .max_depth(d)
                .into_iter()
                .filter_map(Result::ok)
                .collect();
            // Sort from shallowest to deepest
            entries.sort_by_key(|e| e.depth());
            let mut current: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(entries.len());

            for entry in entries.iter_mut() {
                let old_path = entry.path();
                if let Some(new_path) = self.rrename_entry(&old_path) {
                    current.push((old_path.to_path_buf(), new_path.clone()));
                }
            }

            if current.len() != entries.len() {
                anyhow::bail!(
                    "Rename count is different from the dir entries. {}-{}. Depth: {d}",
                    current.len(),
                    entries.len()
                );
            }

            // TODO: figure out how to avoid clonin' wholeass array each depth level
            let brchkd = current.clone();
            current.sort_by_key(|el| el.1.to_string_lossy().into_owned());
            for (from, to) in &mut current {
                if from == to {
                    if self.verbose {
                        println!("No change for '{}'", from.display());
                    }
                    continue;
                }

                let dupes = brchkd.iter().fold(0, |mut acc, el| {
                    if el.1 == *to {
                        acc += 1;
                    }
                    acc
                });

                if dupes > 1 {
                    let n: u16 = rng.random();
                    if let Some(ext) = to.extension() {
                        if let Some(name) = to.file_stem() {
                            let name = name.to_str().unwrap_or_default();
                            let ext = ext.to_str().unwrap_or_default();
                            to.set_file_name(format!("{name}-{n}.{ext}"));
                        }
                    }
                };

                if !self.quiet {
                    println!("'{}' -> '{}'", from.display(), to.display());
                }

                if !self.dry_run {
                    match fs::rename(&mut *from, to) {
                        Ok(_) => renamed += 1,
                        Err(e) => eprintln!("Failed to rename {}: {}", from.display(), e),
                    }
                }
            }

            renames.extend(current);
        }

        renames.sort_by_key(|el| el.1.to_string_lossy().into_owned());
        renames.dedup_by(|a, b| a.1 == b.1);
        renames.sort_by_key(|el| el.1.to_string_lossy().len());

        println!("Renamed: {renamed}, depth:{}", self.depth);

        Ok(renames)
    }

    /// My very specific opinion on what I consider noise
    /// Some are helpful though:
    /// for example '&' may cause issues with some unix tools, like find or ffmpeg
    fn denoise(s: &str) -> String {
        s.to_lowercase()
            //.replace(r"\[[^]]*\]", "")
            .replace(r#"[＂"]"#, "")
            .replace(r"\.-\| \.", "-")
            .replace("\\", "")
            .replace(" ", "-")
            .replace("_", "-")
            .replace(",", "-")
            .replace(",-", "-")
            .replace("\t", "-")
            .replace("&", "-and-")
            .replace("---", "-")
            .replace("--", "-")
    }

    pub fn rrename_entry(&self, path: &Path) -> Option<PathBuf> {
        let name = path.to_str()?;

        let regexed = if let Some(regex) = &self.regex {
            let rep = self.sub.clone().unwrap_or_default();
            let new = if self.no_expand {
                let rep = NoExpand(&rep);
                if self.first {
                    regex.replace(name, rep)
                } else {
                    regex.replace_all(name, rep)
                }
            } else if self.first {
                regex.replace(name, rep)
            } else {
                regex.replace_all(name, rep)
            };

            new.into_owned()
        } else {
            name.to_string()
        };

        let denoised = if self.denoise {
            Self::denoise(&regexed)
        } else {
            regexed
        };

        Some(denoised.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn rename_test_dir() {
        let cli = Rrename::parse().with_path("mock").dry_run();
        let renames = cli.run().unwrap();
        expect![[r#"
            [
                (
                    "mock",
                    "mock",
                ),
                (
                    "mock/Some Dir",
                    "mock/some-dir",
                ),
                (
                    "mock/Another Dir & Co",
                    "mock/another-dir-and-co",
                ),
                (
                    "mock/Some Dir/SOME_fILe.txt",
                    "mock/some-dir/some-file.txt",
                ),
                (
                    "mock/Some -  Word With III dCi135_",
                    "mock/some-word-with-iii-dci135-",
                ),
                (
                    "mock/Some Dir/some,text_file.txt",
                    "mock/some-dir/some-text-file-25057.txt",
                ),
                (
                    "mock/Some Dir/some, text_file.txt",
                    "mock/some-dir/some-text-file-57497.txt",
                ),
                (
                    "mock/Another Dir & Co/Some [some#bs].txt",
                    "mock/another-dir-and-co/some-[some#bs].txt",
                ),
                (
                    "mock/Another Dir & Co/Some & Track.txt",
                    "mock/another-dir-and-co/some-and-track.txt",
                ),
                (
                    "mock/Some -  Word With III dCi135_/Some Word F3500 dCi135 StereoM10.txt",
                    "mock/some-word-with-iii-dci135-/some-word-f3500-dci135-stereom10.txt",
                ),
            ]
        "#]]
        .assert_debug_eq(&renames);
    }
}

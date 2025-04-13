use crate::case::Case;
use clap::{Parser, crate_authors, crate_name, crate_version};
use regex::{NoExpand, Regex};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const ABOUT: &str = "Rename files in batch.\nExample:\nrrename -n";

/// Rename files matching a regular expression by replacing parts of their name.  Similar to the
/// util-linux `rename` command, but with support of regular expressions.
/// Also supports case names: kebab-case, TODO: others
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

    pub fn with_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn run(&self) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
        let total: Vec<_> = WalkDir::new(&self.path)
            .into_iter()
            .filter_map(Result::ok)
            .collect();
        let mut renames: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(total.len());
        let mut renamed = 0;

        // Go from topmost entries to lower ones, iteratievely breadth-first,
        // because entry canot be renamed if the parent is a subject to rename
        for d in 0..=self.depth {
            let mut entries: Vec<_> = WalkDir::new(&self.path)
                .max_depth(d)
                .into_iter()
                .filter_map(Result::ok)
                .collect();
            // Sort from shallowest to deepest
            entries.sort_by_key(|e| e.depth());
            let mut current: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(entries.len());

            for entry in entries.iter_mut() {
                let old_path = entry.path();
                if let Some(new_path) = self.rrename_entry(old_path) {
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

            current.sort_by_key(|el| el.1.as_os_str().len());
            current.dedup_by(|a, b| a.1 == b.1);
            for (from, to) in &current {
                if from == to {
                    if !self.quiet {
                        println!("No change for '{}'", from.display());
                    }
                    continue;
                }

                if !self.quiet {
                    println!("'{}' -> '{}'", from.display(), to.display());
                }

                if !self.dry_run {
                    match fs::rename(from, to) {
                        Ok(_) => renamed += 1,
                        Err(e) => eprintln!("Failed to rename {}: {}", from.display(), e),
                    }
                }
            }

            renames.extend(current);
        }

        renames.sort_by_key(|el| el.1.as_os_str().len());
        renames.dedup_by(|a, b| a.1 == b.1);

        println!("Renamed: {renamed}, depth:{}", self.depth);

        Ok(renames)
    }

    /// My very specific preferences on what I consider noise
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
            .replace("&", "-and-")
            .replace("---", "-")
            .replace("--", "-")
    }

    pub fn rrename_entry(&self, path: &Path) -> Option<PathBuf> {
        let name = path.to_str()?;
        //let name = path.file_stem()?.to_str()?;
        //let ext = path
        //    .extension()
        //    .unwrap_or_default()
        //    .to_str()
        //    .unwrap_or_default();
        //let parent = path.parent();

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

        //if let Some(p) = parent {
        //    let new_full = if ext.is_empty() {
        //        denoised
        //    } else {
        //        format!("{}.{}", denoised, ext)
        //    };
        //    return Some(p.join(new_full));
        //}
        Some(denoised.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn rename_test_dir() {
        let cli = Rrename::parse().with_path("test-dir").with_dry_run();
        let renames = cli.run().unwrap();
        expect![[r#"
            [
                (
                    "test-dir",
                    "test-dir",
                ),
                (
                    "test-dir/another-dir-and-co",
                    "test-dir/another-dir-and-co",
                ),
                (
                    "test-dir/another-dir-and-co/Some & Track.txt",
                    "test-dir/another-dir-and-co/some-and-track.txt",
                ),
                (
                    "test-dir/some-dir",
                    "test-dir/some-dir",
                ),
                (
                    "test-dir/some-dir/SOME_fILe.txt",
                    "test-dir/some-dir/some-file.txt",
                ),
                (
                    "test-dir/some-dir/some-text-file.txt",
                    "test-dir/some-dir/some-text-file.txt",
                ),
            ]
        "#]]
        .assert_debug_eq(&renames);
    }
}

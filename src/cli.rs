use crate::case::Case;
use chrono::{DateTime, Utc};
use clap::{Parser, crate_authors, crate_name, crate_version};
use clap_verbosity_flag::InfoLevel;
use jwalk::WalkDir;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use regex::{NoExpand, Regex};
use std::{
    cmp::Reverse,
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, error, info, trace, warn};

const PREFIX: &str = "rrenamed-";
const EXT: &str = ".txt";
const ABOUT: &str = "Rename files in batch.\nExample:\nrrename -vv --dry-run --denoise my-dir\nrrename -vv --restore rrename-2026-02-24-14:27:23.txt";

/// Rename files matching a regular expression by replacing parts of their name.
/// Similar to the util-linux `rename` command, but with support of regular expressions.
//#[command(arg_required_else_help = true)]
#[derive(Debug, Default, Clone, Parser)]
#[command(name = crate_name!())]
#[command(bin_name = crate_name!())]
#[command(version = crate_version!(),  author = crate_authors!("\n"), about = ABOUT)]
pub struct RrenameCli {
    /// Perform no filesystem operations and display to the user the changes that would happen
    /// without the flag
    #[clap(short = 'n', long)]
    pub dry_run: bool,

    /// Verbosity levels, supports [trace, debug, info, warn, error]: -q, -v, -vv, -vvv, -vvvv
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity<InfoLevel>,

    /// Depth program should go into
    #[clap(short = 'L', long, default_value = "3")]
    pub depth: usize,

    /// Specify case scheme change
    #[clap(short, long, default_value = "kebab-case")]
    pub case: Case,

    /// Rename all files within the directory provided
    #[clap(required_unless_present = "restore")]
    pub path: Option<PathBuf>,

    /// Provide a file with a list of renames to restore
    /// It should be have been created by rrename on complete
    /// Example: rrename -vv --restore rrename-2026-02-24-14:27:23.txt
    #[clap(short = 'r', long, conflicts_with_all = ["path", "dry_run", "no_expand", "first", "denoise", "regex", "sub"])]
    pub restore: Option<PathBuf>,

    /// Prevent Regex parts from being expanded (i.e., `$1`, `$name`)
    #[clap(long)]
    pub no_expand: bool,

    /// Replace only first match
    #[clap(long)]
    pub first: bool,

    /// Whether to save the renamed list to a file
    #[clap(short, long, default_value_t = true)]
    pub save: bool,

    /// Replace noisy chars like:
    /// (['：', ':', '|', '｜', '⧸', '\'', '"', ',', '#', '+', '_', '$'], "-")
    /// with unix friendly '-'
    #[clap(short, long, default_value_t = false)]
    pub denoise: bool,

    /// Regex to use to search in the string.
    /// Example: rrename -vv --regex '^Annoying-prefix-'
    /// Note: if -s,--sub is not provided, every capture will be replaced with empty string
    #[clap(short = 'E', long)]
    pub regex: Option<Regex>,

    /// Exclude files matching this regex
    #[clap(long, short)]
    pub exclude: Option<Regex>,

    /// String to replace matches with.
    /// This will expand `$1` and `$name` into the groups matched by the regex provided.
    /// If this is not the desired behaviour, `--no-expand` should be used.
    #[clap(long, short)]
    pub sub: Option<String>,
}

impl RrenameCli {
    pub fn with_path(mut self, p: impl Into<PathBuf>) -> Self {
        self.path = Some(p.into());
        self
    }
    pub fn with_restore(mut self, p: impl Into<PathBuf>) -> Self {
        self.restore = Some(p.into());
        self
    }
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }
    pub fn with_denoise(mut self) -> Self {
        self.denoise = true;
        self
    }
    pub fn with_save(mut self) -> Self {
        self.save = true;
        self
    }
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn run(&self) -> anyhow::Result<RenameOutput> {
        if let Some(path) = &self.path {
            debug!(path=%path.display(), "Renaming");
            return self.rename();
        }

        if let Some(path) = &self.restore {
            debug!(path=%path.display(), "Restoring");
            return self.restore();
        }

        Ok(Default::default())
    }

    pub fn rename(&self) -> anyhow::Result<RenameOutput> {
        let Some(path) = &self.path else {
            anyhow::bail!("Path is required for rrename");
        };

        let (mut actual_depth, mut count) = (0, 0);
        let mut rng = SmallRng::seed_from_u64(405);
        let mut renames = Vec::with_capacity(4096);

        // Go from topmost entries to lower ones, iteratively breadth-first,
        // because walkdir entry canot be renamed if the parent is a subject to rename
        for d in 0..=self.depth {
            actual_depth = d;
            let mut entries: Vec<_> = WalkDir::new(path)
                .min_depth(d)
                .max_depth(d)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|from| self.filter(from.path()))
                .collect();

            // Sort from shallowest to deepest
            entries.sort_by_key(|el| el.depth());

            let mut current = Vec::with_capacity(entries.len());
            for entry in entries.iter_mut() {
                let from = entry.path();
                let Some(to) = self.to(&from) else {
                    trace!("Invalid UTF8 '{}'", from.display());
                    continue;
                };
                if from == to {
                    trace!("No change for '{}'", from.display());
                    continue;
                }
                if !from.exists() {
                    warn!("Skip missing '{}'", from.display());
                    continue;
                }
                current.push((from.to_path_buf(), to.clone()));
            }

            // TODO: figure out how to avoid clonin' wholeass array each depth level
            let brchkd = current.clone();
            for (from, to) in &mut current {
                let dupes = brchkd.iter().fold(0, |mut acc, el| {
                    if el.1 == *to {
                        acc += 1;
                    }
                    acc
                });

                if dupes > 1 {
                    let n: u16 = rng.random();
                    if let (Some(stem), Some(ext)) = (to.file_stem(), to.extension()) {
                        let stem = stem.to_string_lossy();
                        let ext = ext.to_string_lossy();
                        to.set_file_name(format!("{stem}-{n}.{ext}"));
                    }
                }

                self.fs_rename(from, to, &mut count)?;
            }
            renames.extend(current);
        }
        renames.sort_by_key(|el| el.1.to_string_lossy().len());

        info!("Renamed: {count}, depth: {actual_depth}");
        let list_path = self.save_list(count, &renames)?;

        Ok(RenameOutput {
            list_path,
            renames,
            count,
        })
    }

    pub fn restore(&self) -> anyhow::Result<RenameOutput> {
        let Some(path) = &self.restore else {
            anyhow::bail!("--restore requires a file");
        };

        let content = std::fs::read_to_string(path)?;
        let mut renames = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let Some((from, to)) = line.split_once(" -> ") else {
                error!("Invalid restore line {}: {}", idx + 1, line);
                continue;
            };

            let (current, original) = (PathBuf::from(to), PathBuf::from(from));
            if !current.exists() {
                warn!("Skip missing '{}'", current.display());
                continue;
            }

            if self.filter(&current) {
                renames.push((current, original));
            }
        }

        // deepest first ordered by path len for restore
        renames.sort_by_key(|(from, _)| Reverse(from.components().count()));

        let mut count = 0;
        for (from, to) in &renames {
            if from == to {
                trace!("No change for '{}'", from.display());
                continue;
            }
            self.fs_rename(from, to, &mut count)?;
        }

        info!("Restored: {count}");
        let list_path = self.save_list(count, &renames)?;

        Ok(RenameOutput {
            list_path,
            renames,
            count,
        })
    }

    fn to(&self, path: &Path) -> Option<PathBuf> {
        let name = path.to_str()?;

        let regexed = match &self.regex {
            None => name.to_string(),
            Some(regex) => {
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
            }
        };

        let denoised = if self.denoise {
            Self::denoise(&regexed)
        } else {
            regexed
        };
        Some(denoised.into())
    }

    /// My very specific opinion on what I consider noise
    /// Some are helpful though:
    /// for example '&' may cause bash issues with some unix tools, like find or ffmpeg
    fn denoise(s: &str) -> String {
        s.to_lowercase()
            // \W will select all non "word" characters equivalent to [^a-zA-Z0-9_]
            .replace(
                [
                    '\\', '：', '|', ':', '｜', '⧸', '\'', '"', ',', '#', '+', '_', '$',
                ],
                "-",
            )
            .replace("-/", "/") // trailing -/
            .replace(r"[_,]", "-")
            .replace('&', "and")
            .replace(' ', "-")
            .replace("---", "-")
            .replace("--", "-")
            .replace("-.", ".")
            .trim_end_matches('-') // trailing -
            .to_string()
    }

    fn filter(&self, from: impl AsRef<Path>) -> bool {
        let from = from.as_ref();
        if let Some(exclude) = &self.exclude
            && exclude.is_match(&from.to_string_lossy())
        {
            debug!("SKIP '{}'", from.display());
            return false;
        }
        true
    }

    fn fs_rename(&self, from: &Path, to: &Path, renamed: &mut usize) -> anyhow::Result<()> {
        if !self.dry_run {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            match fs::rename(from, to) {
                Ok(_) => {
                    debug!("RENAME '{}' -> '{}'", from.display(), to.display());
                    *renamed += 1;
                    return Ok(());
                }
                Err(e) => anyhow::bail!("Failed to rename {}: {}", to.display(), e),
            }
        }
        Ok(())
    }

    fn save_list(&self, n: usize, list: &[(PathBuf, PathBuf)]) -> anyhow::Result<Option<String>> {
        if self.save && n > 0 {
            let s: String = list
                .iter()
                .map(|(from, to)| format!("{} -> {}", from.display(), to.display()))
                .collect::<Vec<_>>()
                .join("\n");
            // saves file as rrenamed-2026-02-24-14:27:23.txt
            let utc: DateTime<Utc> = Utc::now();
            let name = format!("{PREFIX}{}{EXT}", utc.format("%Y-%m-%d-%H:%M:%S"));
            std::fs::write(&name, s)?;
            info!("renamed list saved as {name}");
            return Ok(Some(name));
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenameOutput {
    pub count: usize,
    pub renames: Vec<(PathBuf, PathBuf)>,
    pub list_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use std::path::Path;
    use std::{fs, io};

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    #[test]
    fn test_rename_restore_mock_dir() {
        // backup
        copy_dir_all("mock", "mock.bak").expect("Failed to copy dir");

        // TEST RENAME
        let cli = RrenameCli::default()
            .with_path("mock")
            .with_depth(5)
            .with_save()
            .with_denoise();
        let out = cli.run().expect("Failed to run rrename");
        expect![[r#"
            [
                (
                    "mock/Super",
                    "mock/super",
                ),
                (
                    "mock/Some Dir",
                    "mock/some-dir",
                ),
                (
                    "mock/super/NESTED",
                    "mock/super/nested",
                ),
                (
                    "mock/super/nested/UgLy",
                    "mock/super/nested/ugly",
                ),
                (
                    "mock/Another Dir & Co",
                    "mock/another-dir-and-co",
                ),
                (
                    "mock/some-dir/SOME_⧸fILe.txt",
                    "mock/some-dir/some-file.txt",
                ),
                (
                    "mock/super/nested/ugly/dIIr",
                    "mock/super/nested/ugly/diir",
                ),
                (
                    "mock/Some -  Word With III dCi135_",
                    "mock/some-word-with-iii-dci135",
                ),
                (
                    "mock/some-dir/some,text_file\\#_$.txt",
                    "mock/some-dir/some-text-file-25057.txt",
                ),
                (
                    "mock/some-dir/some：, text_file.txt",
                    "mock/some-dir/some-text-file-57497.txt",
                ),
                (
                    "mock/super/nested/ugly/diir/with|file.txt",
                    "mock/super/nested/ugly/diir/with-file.txt",
                ),
                (
                    "mock/another-dir-and-co/Some::: & Track.txt",
                    "mock/another-dir-and-co/some-and-track.txt",
                ),
                (
                    "mock/another-dir-and-co/Some [some#bs].txt",
                    "mock/another-dir-and-co/some-[some-bs].txt",
                ),
                (
                    "mock/some-dir/some'weird'file\"with\"brackets.txt",
                    "mock/some-dir/some-weird-file-with-brackets.txt",
                ),
                (
                    "mock/some-word-with-iii-dci135/Some Word F3500 dCi135 StereoM10.txt",
                    "mock/some-word-with-iii-dci135/some-word-f3500-dci135-stereom10.txt",
                ),
            ]
        "#]]
        .assert_debug_eq(&out.renames);
        let rrename_out_file = out.list_path.clone().unwrap_or_default();
        println!("list_path: {}", rrename_out_file);
        assert_eq!(out.count, 15);

        // TEST RESTORE
        let cli = RrenameCli::default().with_restore(out.list_path.unwrap());
        let out = cli.run().unwrap();
        assert_eq!(out.count, 15);
        expect![[r#"
            [
                (
                    "mock/super/nested/ugly/diir/with-file.txt",
                    "mock/super/nested/ugly/diir/with|file.txt",
                ),
                (
                    "mock/super/nested/ugly/diir",
                    "mock/super/nested/ugly/dIIr",
                ),
                (
                    "mock/super/nested/ugly",
                    "mock/super/nested/UgLy",
                ),
                (
                    "mock/super/nested",
                    "mock/super/NESTED",
                ),
                (
                    "mock/some-dir/some-file.txt",
                    "mock/some-dir/SOME_⧸fILe.txt",
                ),
                (
                    "mock/some-dir/some-text-file-25057.txt",
                    "mock/some-dir/some,text_file\\#_$.txt",
                ),
                (
                    "mock/some-dir/some-text-file-57497.txt",
                    "mock/some-dir/some：, text_file.txt",
                ),
                (
                    "mock/another-dir-and-co/some-and-track.txt",
                    "mock/another-dir-and-co/Some::: & Track.txt",
                ),
                (
                    "mock/another-dir-and-co/some-[some-bs].txt",
                    "mock/another-dir-and-co/Some [some#bs].txt",
                ),
                (
                    "mock/some-dir/some-weird-file-with-brackets.txt",
                    "mock/some-dir/some'weird'file\"with\"brackets.txt",
                ),
                (
                    "mock/some-word-with-iii-dci135/some-word-f3500-dci135-stereom10.txt",
                    "mock/some-word-with-iii-dci135/Some Word F3500 dCi135 StereoM10.txt",
                ),
                (
                    "mock/super",
                    "mock/Super",
                ),
                (
                    "mock/some-dir",
                    "mock/Some Dir",
                ),
                (
                    "mock/another-dir-and-co",
                    "mock/Another Dir & Co",
                ),
                (
                    "mock/some-word-with-iii-dci135",
                    "mock/Some -  Word With III dCi135_",
                ),
            ]
        "#]]
        .assert_debug_eq(&out.renames);

        // CLEANUP
        fs::remove_file(rrename_out_file).expect("Failed to remove file");
        // FIXME: this is so stupid, but it works
        fs::remove_dir_all("mock").expect("Failed to remove dir");
        copy_dir_all("mock.bak", "mock").expect("Failed to copy dir");
        fs::remove_dir_all("mock.bak").expect("Failed to remove dir");
    }
}

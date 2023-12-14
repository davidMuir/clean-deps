use bytesize::ByteSize;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use std::{
    cmp::Reverse,
    env::{self},
    error::Error,
    ffi::OsStr,
    fmt::Display,
    fs::{self, File},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Args {
    path: Option<PathBuf>,
    #[arg(short, long)]
    delete: bool,
    #[arg(short, long)]
    language: Option<RepoLanguage>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let root = args.path.unwrap_or(env::current_dir()?);

    let mut repos = find_repos(root)?;

    if let Some(language) = args.language {
        repos.retain(|r| r.language == language);
    }

    repos.sort_by_key(|a| Reverse(a.deps_size));

    for repo in &repos {
        println!("{repo}");
    }

    let total_size = &repos
        .iter()
        .map(|r| r.deps_size)
        .fold(ByteSize(0), |a, b| a + b);

    println!();

    println!("Total size: {total_size}");

    if args.delete {
        println!();
        println!("Removing dependencies:");

        for repo in repos {
            repo.delete_deps()?;
        }
    }

    Ok(())
}

fn find_repos(path: PathBuf) -> Result<Vec<Repo>, Box<dyn Error>> {
    let mut repos = vec![];

    if Repo::is_repo(&path) {
        if let Some(repo) = Repo::new(&path)? {
            repos.push(repo);
        }
    }

    let content = fs::read_dir(&path)?;

    for entry in content.flatten() {
        if entry.file_type()?.is_dir() {
            repos.extend(find_repos(entry.path())?);
        }
    }

    Ok(repos)
}

#[derive(Debug)]
struct Repo {
    language: RepoLanguage,
    dir: PathBuf,
    deps_size: ByteSize,
}

impl Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<10} {:<45}\t{}",
            self.language,
            truncate_path_for_display(&self.dir, 40),
            self.deps_size
        )
    }
}

fn truncate_path_for_display(path: &Path, max_length: usize) -> String {
    if let Some(path) = path.to_str() {
        if path.len() <= max_length {
            String::from(path)
        } else {
            let mut str = String::from(&path[0..(max_length / 2) - 5]);

            str.push_str("[...]");

            str.push_str(&path[path.len() - (max_length / 2)..]);
            str
        }
    } else {
        String::from("Can't display path")
    }
}

impl Repo {
    fn new(path: &Path) -> Result<Option<Self>, Box<dyn Error>> {
        if let Some(language) = Self::get_language(path)? {
            Ok(Some(Self {
                dir: path.to_path_buf(),
                deps_size: Self::get_deps_size(path, &language)?,
                language,
            }))
        } else {
            Ok(None)
        }
    }

    fn is_repo(path: &Path) -> bool {
        matches!(Self::get_language(path), Ok(Some(_)))
    }

    fn get_language(path: &Path) -> Result<Option<RepoLanguage>, Box<dyn Error>> {
        for entry in fs::read_dir(path)?.flatten() {
            if entry.file_name().eq_ignore_ascii_case("Cargo.toml") {
                return Ok(Some(RepoLanguage::Rust));
            }

            if entry.path().extension() == Some(OsStr::new("sln")) {
                return Ok(Some(RepoLanguage::Dotnet));
            }

            if entry.path().extension() == Some(OsStr::new("csproj")) {
                return Ok(Some(RepoLanguage::Dotnet));
            }

            if entry.file_name().eq_ignore_ascii_case("package.json") {
                return Ok(Some(RepoLanguage::Javascript));
            }
        }

        Ok(None)
    }

    fn get_deps_size(path: &Path, language: &RepoLanguage) -> Result<ByteSize, Box<dyn Error>> {
        let mut size = 0;

        for dp in language.get_dep_paths() {
            size += get_size(&path.join(dp))?;
        }

        Ok(ByteSize(size))
    }

    fn delete_deps(self) -> Result<(), Box<dyn Error>> {
        for dp in self.language.get_dep_paths() {
            let path = self.dir.join(dp);

            if get_size(&path)? > 0 {
                println!("Removing {}", truncate_path_for_display(&path, 60));
                let _ = fs::remove_dir_all(path);
            } else {
                println!("Skipping empty: {}", truncate_path_for_display(&path, 60));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum RepoLanguage {
    Dotnet,
    Rust,
    Javascript,
}

impl Display for RepoLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            match self {
                Self::Dotnet => "dotnet".blue(),
                Self::Javascript => "js".yellow(),
                Self::Rust => "rust".red(),
            }
        )
    }
}

impl RepoLanguage {
    fn get_dep_paths(&self) -> Vec<PathBuf> {
        match self {
            Self::Dotnet => vec!["bin".into(), "obj".into()],
            Self::Javascript => vec!["node_modules".into()],
            Self::Rust => vec!["target".into()],
        }
    }
}

fn get_size(path: &PathBuf) -> Result<u64, Box<dyn Error>> {
    let mut size = 0;
    if let Ok(file) = File::open(path) {
        if file.metadata()?.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;

                size += get_size(&entry.path())?;
            }
        } else {
            size = file.metadata()?.size();
        }

        Ok(size)
    } else {
        Ok(0)
    }
}

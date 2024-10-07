use arboard::Clipboard;
use clap::Parser;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;

#[derive(Parser)]
#[command(
    name = "code_collector",
    about = "Collect code files into a buffer, respecting .gitignore and filtering by extension"
)]
struct Cli {
    /// The directory to process
    directory: String,

    /// File extensions to include (e.g., rs, py). Specify multiple times for multiple extensions.
    #[arg(short, long, value_name = "EXTENSION", use_value_delimiter = true)]
    extensions: Vec<String>,

    /// Directories to exclude
    #[arg(
        short = 'x',
        long,
        value_name = "DIRECTORY",
        use_value_delimiter = true
    )]
    exclude_dirs: Vec<String>,
}

enum CommentStyle {
    Line(&'static str),
    Block(&'static str, &'static str),
}

fn get_comment_syntax(extension: &str) -> CommentStyle {
    match extension {
        "rs" | "js" | "ts" | "c" | "h" | "cpp" | "hpp" | "java" | "cs" | "go" | "swift" | "kt"
        | "kts" => CommentStyle::Line("//"),
        "py" | "sh" | "yaml" | "yml" | "toml" | "ini" | "rb" | "pl" | "r" | "php" | "ps1"
        | "makefile" => CommentStyle::Line("#"),
        "html" | "xml" | "xhtml" => CommentStyle::Block("<!--", "-->"),
        "css" => CommentStyle::Block("/*", "*/"),
        _ => CommentStyle::Line("//"),
    }
}

struct TreeNode {
    name: String,
    children: HashMap<String, TreeNode>,
}

impl TreeNode {
    fn new(name: String) -> Self {
        TreeNode {
            name,
            children: HashMap::new(),
        }
    }

    fn add_path(&mut self, path_components: &[String]) {
        if path_components.is_empty() {
            return;
        }
        let name = path_components[0].clone();
        let node = self
            .children
            .entry(name.clone())
            .or_insert_with(|| TreeNode::new(name));
        node.add_path(&path_components[1..]);
    }

    fn print(&self, prefix: &str, is_last: bool) {
        if !self.name.is_empty() {
            println!(
                "{}{}{}",
                prefix,
                if is_last { "└── " } else { "├── " },
                self.name
            );
        }

        let mut keys: Vec<&String> = self.children.keys().collect();
        keys.sort();
        for (i, key) in keys.iter().enumerate() {
            let child = self.children.get(*key).unwrap();
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            child.print(&new_prefix, i == keys.len() - 1);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let directory = args.directory;
    println!("Processing directory: {}", directory);

    let extensions: Vec<String> = args.extensions.iter().map(|s| s.to_lowercase()).collect();

    let mut types_builder = TypesBuilder::new();

    if !extensions.is_empty() {
        for ext in &extensions {
            let pattern = format!("*.{}", ext);
            types_builder.add(ext, &pattern)?;
            types_builder.select(ext);
        }
    } else {
        types_builder.add_defaults();
    }

    let types_matcher = types_builder.build()?;

    let mut code_buffer = String::new();
    let mut copied_files = Vec::new();

    let mut excluded_dirs: HashSet<String> = [
        "node_modules",
        "target",
        "build",
        "dist",
        "venv",
        "env",
        ".venv",
        ".env",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    // Include user-specified directories to exclude
    for dir in args.exclude_dirs {
        excluded_dirs.insert(dir);
    }

    let walker = WalkBuilder::new(&directory)
        .types(types_matcher)
        .git_ignore(true)
        .hidden(true)
        .filter_entry(move |entry| {
            let path = entry.path();
            if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                if path.is_dir() && excluded_dirs.contains(dir_name) {
                    return false;
                }
            }
            true
        })
        .build();

    for result in walker {
        let entry = result?;
        let path = entry.path();

        if path.is_file() {
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !extensions.is_empty() && !extensions.iter().any(|e| e == &extension) {
                continue;
            }

            let comment_syntax = get_comment_syntax(&extension);

            let relative_path = path.strip_prefix(&directory)?;
            let full_relative_path = relative_path.to_string_lossy();

            match fs::read_to_string(path) {
                Ok(content) => {
                    let mut file_content = String::new();

                    match comment_syntax {
                        CommentStyle::Line(prefix) => {
                            file_content.push_str(&format!("{} {}\n", prefix, full_relative_path));
                        }
                        CommentStyle::Block(start, end) => {
                            file_content
                                .push_str(&format!("{} {}\n{}\n", start, full_relative_path, end));
                        }
                    }

                    file_content.push_str(&content);
                    file_content.push_str("\n\n");

                    code_buffer.push_str(&file_content);
                    copied_files.push(relative_path.to_owned());
                }
                Err(e) if e.kind() == ErrorKind::InvalidData => {
                    eprintln!("Skipping binary file {:?}", path);
                }
                Err(e) => {
                    eprintln!("Could not read file {:?}: {}", path, e);
                }
            }
        }
    }

    let mut root = TreeNode::new(String::new());

    for path in &copied_files {
        let components: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().into_owned())
            .collect();

        root.add_path(&components);
    }

    println!("Copied Files Tree:");
    root.print("", true);

    // Copy the collected code buffer to the OS clipboard using arboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(code_buffer)?;

    println!("Code buffer has been copied to the clipboard.");

    Ok(())
}

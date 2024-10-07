# Code-Collector

A simple CLI tool for copying the entirety of a codebase into the clipboard.

Just call it on a directory containing text files, optionally filtering e.g. using `-e rs,py,js` to filter only rust, python and javascript files.

Here is an example output of a python and rust mixed codebase.
```
# python_scripts/script1.py

def greet():
    print("Hello from script1")

if __name__ == "__main__":
    greet()


# python_scripts/script2.py

def add(a, b):
    return a + b

print("2 + 3 =", add(2, 3))


# rust_project/Cargo.toml
[package]
name = "rust_project"
version = "0.1.0"
edition = "2021"

[dependencies]


// rust_project/src/main.rs
fn main() {
    println!("Hello, world!");
}

```

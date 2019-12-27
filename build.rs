extern crate gcc;

fn main() {
    gcc::Config::new()
                .file("src/clib.h");
                // .include("src")
                // .compile("libhello.a");
}
